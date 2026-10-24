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
