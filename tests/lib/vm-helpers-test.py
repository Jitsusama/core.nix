"""Test what VM tests' scripts share (vm_helpers.py), against a stand-in for a machine.

A helper's whole job is the command line it hands the driver, so each test reads that line
from a machine that only records it, which keeps these tests to milliseconds.
"""

from __future__ import annotations

import datetime as dt
import shlex
from typing import TYPE_CHECKING, Any, cast

import pytest
import vm_helpers

if TYPE_CHECKING:
    from collections.abc import Callable

    from test_driver.machine import BaseMachine


class Recorder:
    """A machine that records each command it's given, and says it worked or didn't."""

    def __init__(self) -> None:
        """Start with nothing recorded."""
        self.succeeded: list[str] = []
        self.failed: list[str] = []

    def succeed(self, command: str) -> str:
        """Record a command that must work, and give what it printed."""
        self.succeeded.append(command)
        return "printed"

    def fail(self, command: str) -> str:
        """Record a command that must fail, and give what it printed."""
        self.failed.append(command)
        return "refused"

    @property
    def machine(self) -> BaseMachine:
        """Itself, as the helpers take a machine."""
        return cast("BaseMachine", self)


@pytest.fixture
def recorder() -> Recorder:
    """Give a machine with nothing recorded."""
    return Recorder()


def inner(line: str) -> str:
    """Give the command su runs, from the line given to the machine."""
    return shlex.split(line)[-1]


def test_a_command_runs_as_joel_in_his_home_with_his_runtime_folder(recorder: Recorder) -> None:
    said = vm_helpers.as_account(recorder.machine, "systemctl --user status")
    assert said == "printed"
    (line,) = recorder.succeeded
    assert line.startswith("su - joel -c ")
    assert inner(line) == "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; systemctl --user status"


def test_another_account_and_runtime_folder_are_used_when_given(recorder: Recorder) -> None:
    vm_helpers.as_account(recorder.machine, "id", account="alice", runtime="/run/user/1001")
    (line,) = recorder.succeeded
    assert line.startswith("su - alice -c ")
    assert "XDG_RUNTIME_DIR=/run/user/1001" in inner(line)


def test_the_session_bus_is_given_when_asked(recorder: Recorder) -> None:
    vm_helpers.as_account(recorder.machine, "busctl --user list", bus=True)
    (line,) = recorder.succeeded
    assert "DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus" in inner(line)


def test_a_command_that_must_fail_is_given_as_one(recorder: Recorder) -> None:
    said = vm_helpers.as_account(recorder.machine, "false", succeed=False)
    assert said == "refused"
    assert recorder.succeeded == []
    assert len(recorder.failed) == 1


def test_a_command_with_quotes_reaches_the_shell_whole(recorder: Recorder) -> None:
    vm_helpers.as_account(recorder.machine, "echo 'it''s' \"$HOME\"")
    (line,) = recorder.succeeded
    assert inner(line).endswith("echo 'it''s' \"$HOME\"")


def test_the_last_journal_cursor_printed_is_given() -> None:
    said = "a line\n-- cursor: s=old\nanother\n-- cursor: s=abc;i=1f\n"
    assert vm_helpers.journal_cursor(said) == "s=abc;i=1f"


def test_output_with_no_cursor_is_refused_saying_what_it_was() -> None:
    with pytest.raises(ValueError, match="printed no cursor"):
        vm_helpers.journal_cursor("No entries\n")


def test_a_host_with_processors_to_spare_waits_as_long_as_stated() -> None:
    assert vm_helpers.pace(0.3) == 1.0


def test_a_host_with_more_tasks_than_processors_waits_as_many_times_longer() -> None:
    tasks_for_each_processor = 2.5
    assert vm_helpers.pace(tasks_for_each_processor) == tasks_for_each_processor


def test_however_busy_the_host_a_wait_is_held_to_its_most_pace() -> None:
    assert vm_helpers.pace(vm_helpers.MOST_PACE * 3) == vm_helpers.MOST_PACE


class StandInRetry:
    """The driver's retry, which records how long it was told to wait, and fails if asked."""

    def __init__(self, *, fails: bool = False) -> None:
        """Start with nothing recorded."""
        self.fails = fails
        self.told: list[tuple[int | dt.timedelta | None, dt.timedelta | None]] = []

    def __call__(
        self,
        fn: Callable[..., Any],
        timeout_seconds: int | dt.timedelta | None = None,
        timeout: dt.timedelta | None = None,
    ) -> None:
        """Record the time given, calling fn once as the last try."""
        self.told.append((timeout_seconds, timeout))
        fn(True)  # noqa: FBT003 # As the driver calls it, last.
        if self.fails:
            message = "action timed out"
            raise AssertionError(message)


@pytest.fixture
def retry() -> StandInRetry:
    """Give the driver's retry, recording and never failing."""
    return StandInRetry()


def nothing(_: bool) -> bool:  # noqa: FBT001 # The driver tells it whether it's the last try.
    """Say yes at once, as a wait for what's already so does."""
    return True


def test_a_wait_in_seconds_is_given_as_a_time_that_much_longer(retry: StandInRetry) -> None:
    vm_helpers.paced(retry, busy=lambda: 3.0)(nothing, timeout_seconds=10)
    assert retry.told == [(None, dt.timedelta(seconds=30))]


def test_a_wait_given_as_the_machine_gives_it_is_paced_the_same(retry: StandInRetry) -> None:
    vm_helpers.paced(retry, busy=lambda: 2.0)(nothing, dt.timedelta(seconds=60))
    assert retry.told == [(None, dt.timedelta(minutes=2))]


def test_a_wait_told_no_time_paces_the_drivers_fifteen_minutes(retry: StandInRetry) -> None:
    vm_helpers.paced(retry, busy=lambda: 1.5)(nothing)
    assert retry.told == [(None, dt.timedelta(minutes=22, seconds=30))]


def test_a_quiet_host_waits_exactly_as_stated(retry: StandInRetry) -> None:
    vm_helpers.paced(retry, busy=lambda: 0.1)(nothing, timeout=dt.timedelta(seconds=7))
    assert retry.told == [(None, dt.timedelta(seconds=7))]


def test_busyness_is_read_as_each_wait_starts(retry: StandInRetry) -> None:
    readings = iter([1.0, 3.0])
    waits = vm_helpers.paced(retry, busy=lambda: next(readings))
    waits(nothing, timeout_seconds=10)
    waits(nothing, timeout_seconds=10)
    assert [told for _, told in retry.told] == [dt.timedelta(seconds=s) for s in (10, 30)]


def test_both_kinds_of_time_reach_the_driver_to_be_refused(retry: StandInRetry) -> None:
    vm_helpers.paced(retry, busy=lambda: 2.0)(nothing, 5, dt.timedelta(seconds=5))
    assert retry.told == [(5, dt.timedelta(seconds=5))]


def test_a_wait_that_fails_describes_the_machine_then_fails() -> None:
    described: list[str] = []
    waits = vm_helpers.paced(
        StandInRetry(fails=True), busy=lambda: 1.0, describe=lambda: described.append("shown")
    )
    with pytest.raises(AssertionError, match="timed out"):
        waits(nothing, timeout_seconds=10)
    assert described == ["shown"]


def test_a_wait_that_succeeds_describes_nothing(retry: StandInRetry) -> None:
    described: list[str] = []
    waits = vm_helpers.paced(retry, busy=lambda: 1.0, describe=lambda: described.append("shown"))
    waits(nothing, timeout_seconds=10)
    assert described == []


class FadingGuest:
    """A guest whose drawer fades in a frame at a time, on a clock of its own.

    Its frames come as far apart as a busy host lets it draw them, each changing the screen,
    and looking at the screen takes a moment, as QEMU's screendump does.
    """

    LOOKING = 0.05

    def __init__(
        self, frames: int, apart: float, *, other: bool = False, asked: bool = False
    ) -> None:
        """Start the fade at nought, its frames this far apart.

        Other moves a window of its own; asked draws the launcher a frame, which changes
        nothing on screen, each time its frames are asked for.
        """
        self.now = 0.0
        self.frames = frames
        self.apart = apart
        self.other = other
        self.asked = asked
        self.counted = 0
        self.owed = 0

    @property
    def faded_by(self) -> float:
        """Give when the fade's last frame is drawn."""
        return self.frames * self.apart

    def drawn_by_now(self) -> int:
        """Give how many of the fade's frames are drawn."""
        return min(self.frames, int(self.now / self.apart))

    def clock(self) -> float:
        """Give the guest's time."""
        return self.now

    def wait(self, span: float) -> None:
        """Let the time pass."""
        self.now += span

    def screen(self) -> bytes:
        """Give the screen's picture, which another window moving changes each time."""
        self.now += self.LOOKING
        moved = int(self.now / self.LOOKING) if self.other else 0
        return f"{self.drawn_by_now()} {moved}".encode()

    def drawn(self) -> dict[str, int]:
        """Give the shell's frames since it was last asked, as `qs ipc call frames` says them."""
        drawn, self.counted = self.drawn_by_now() - self.counted, self.drawn_by_now()
        drawn, self.owed = drawn + self.owed, int(self.asked)
        return {"launcher": drawn} if drawn else {}


def settled(guest: FadingGuest, span: float, within: float, asking: int = 0) -> str | None:
    """Wait on the guest's screen to stand still."""
    return vm_helpers.settled(
        guest.screen,
        guest.drawn,
        span,
        within,
        asking=asking,
        clock=guest.clock,
        wait=guest.wait,
    )


def test_a_drawer_fading_slower_than_half_a_second_a_frame_is_waited_out() -> None:
    # A host twice as busy as it has processors, its fade's frames 0.6 s apart (ZJ62K2).
    guest = FadingGuest(frames=5, apart=0.6)
    pace = vm_helpers.pace(2.0)
    assert settled(guest, 0.5 * pace, 10 * pace) is None
    assert guest.now >= guest.faded_by, f"still at {guest.now:.2f} s, the fade ends at 3 s"


def test_a_screen_already_still_settles_in_two_spans() -> None:
    guest = FadingGuest(frames=0, apart=1.0)
    two_spans_and_their_looks = 1.5
    assert settled(guest, 0.5, 10) is None
    assert guest.now < two_spans_and_their_looks, f"a still screen took {guest.now:.2f} s"


def test_a_screen_still_drawing_at_the_bound_says_so() -> None:
    guest = FadingGuest(frames=100, apart=0.3)
    why = settled(guest, 0.5, 5) or ""
    said = "the screen never stood still for 2 spans of 0.5 s in 5 s: the shell was still drawing"
    assert why.startswith(said), why


def test_the_frame_asking_costs_a_still_launcher_is_rest() -> None:
    # Every gallery's launcher shots on 2026-11-09 counted one frame a span, however long.
    assert settled(FadingGuest(frames=0, apart=1.0, asked=True), 0.5, 5, asking=1) is None
    why = settled(FadingGuest(frames=0, apart=1.0, asked=True), 0.5, 5) or ""
    assert why.endswith("the shell was still drawing, last {'launcher': 1}"), why


def test_a_fade_is_waited_out_past_the_frames_asking_costs() -> None:
    guest = FadingGuest(frames=5, apart=0.6, asked=True)
    pace = vm_helpers.pace(2.0)
    assert settled(guest, 0.5 * pace, 10 * pace, asking=1) is None
    assert guest.now >= guest.faded_by, f"still at {guest.now:.2f} s, the fade ends at 3 s"


def test_another_programs_window_moving_is_no_rest_either() -> None:
    guest = FadingGuest(frames=0, apart=1.0, other=True)
    why = settled(guest, 0.5, 5)
    assert why == (
        "the screen never stood still for 2 spans of 0.5 s in 5 s:"
        " the screen changed with no frame of the shell's"
    ), why
