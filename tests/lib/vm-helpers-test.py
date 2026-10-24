"""Test what VM tests' scripts share (vm_helpers.py), against a stand-in for a machine.

A helper's whole job is the command line it hands the driver, so each test reads that line
from a machine that only records it, which keeps these tests to milliseconds.
"""

import datetime as dt
import shlex
import unittest
from typing import TYPE_CHECKING, Any, cast

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


def inner(line: str) -> str:
    """Give the command su runs, from the line given to the machine."""
    return shlex.split(line)[-1]


class AsAccount(unittest.TestCase):
    """Running a command as an account."""

    def setUp(self) -> None:
        self.machine = Recorder()
        self.as_machine = cast("BaseMachine", self.machine)

    def test_it_runs_as_joel_in_his_home_with_his_runtime_folder(self) -> None:
        said = vm_helpers.as_account(self.as_machine, "systemctl --user status")
        self.assertEqual(said, "printed")
        (line,) = self.machine.succeeded
        self.assertTrue(line.startswith("su - joel -c "))
        self.assertEqual(
            inner(line), "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; systemctl --user status"
        )

    def test_another_account_and_runtime_folder_are_used_when_given(self) -> None:
        vm_helpers.as_account(self.as_machine, "id", account="alice", runtime="/run/user/1001")
        (line,) = self.machine.succeeded
        self.assertTrue(line.startswith("su - alice -c "))
        self.assertIn("XDG_RUNTIME_DIR=/run/user/1001", inner(line))

    def test_the_session_bus_is_given_when_asked(self) -> None:
        vm_helpers.as_account(self.as_machine, "busctl --user list", bus=True)
        self.assertIn(
            "DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus",
            inner(self.machine.succeeded[0]),
        )

    def test_a_command_that_must_fail_is_given_as_one(self) -> None:
        said = vm_helpers.as_account(self.as_machine, "false", succeed=False)
        self.assertEqual((said, self.machine.succeeded), ("refused", []))
        self.assertEqual(len(self.machine.failed), 1)

    def test_a_command_with_quotes_reaches_the_shell_whole(self) -> None:
        vm_helpers.as_account(self.as_machine, "echo 'it''s' \"$HOME\"")
        self.assertTrue(inner(self.machine.succeeded[0]).endswith("echo 'it''s' \"$HOME\""))


class JournalCursor(unittest.TestCase):
    """Reading where a journal had reached."""

    def test_the_last_cursor_printed_is_given(self) -> None:
        said = "a line\n-- cursor: s=old\nanother\n-- cursor: s=abc;i=1f\n"
        self.assertEqual(vm_helpers.journal_cursor(said), "s=abc;i=1f")

    def test_output_with_no_cursor_is_refused_saying_what_it_was(self) -> None:
        with self.assertRaisesRegex(ValueError, "printed no cursor"):
            vm_helpers.journal_cursor("No entries\n")


class Pace(unittest.TestCase):
    """How much longer a wait may take on a host this busy."""

    def test_a_host_with_processors_to_spare_waits_as_long_as_stated(self) -> None:
        self.assertEqual(vm_helpers.pace(0.3), 1.0)

    def test_a_host_with_more_tasks_than_processors_waits_as_many_times_longer(self) -> None:
        self.assertEqual(vm_helpers.pace(2.5), 2.5)

    def test_however_busy_the_host_a_wait_takes_at_most_four_times_as_long(self) -> None:
        self.assertEqual(vm_helpers.pace(9.0), 4.0)


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


def nothing(_: bool) -> bool:  # noqa: FBT001 # The driver tells it whether it's the last try.
    """Say yes at once, as a wait for what's already so does."""
    return True


class Paced(unittest.TestCase):
    """Waiting through the driver's retry, paced to how busy the host is."""

    def test_a_wait_in_seconds_is_given_as_a_time_that_much_longer(self) -> None:
        retry = StandInRetry()
        vm_helpers.paced(retry, busy=lambda: 3.0)(nothing, timeout_seconds=10)
        self.assertEqual(retry.told, [(None, dt.timedelta(seconds=30))])

    def test_a_wait_given_as_the_machine_gives_it_is_paced_the_same(self) -> None:
        retry = StandInRetry()
        vm_helpers.paced(retry, busy=lambda: 2.0)(nothing, dt.timedelta(seconds=60))
        self.assertEqual(retry.told, [(None, dt.timedelta(minutes=2))])

    def test_a_wait_told_no_time_paces_the_drivers_fifteen_minutes(self) -> None:
        retry = StandInRetry()
        vm_helpers.paced(retry, busy=lambda: 1.5)(nothing)
        self.assertEqual(retry.told, [(None, dt.timedelta(minutes=22, seconds=30))])

    def test_a_quiet_host_waits_exactly_as_stated(self) -> None:
        retry = StandInRetry()
        vm_helpers.paced(retry, busy=lambda: 0.1)(nothing, timeout=dt.timedelta(seconds=7))
        self.assertEqual(retry.told, [(None, dt.timedelta(seconds=7))])

    def test_busyness_is_read_as_each_wait_starts(self) -> None:
        retry = StandInRetry()
        readings = iter([1.0, 3.0])
        waits = vm_helpers.paced(retry, busy=lambda: next(readings))
        waits(nothing, timeout_seconds=10)
        waits(nothing, timeout_seconds=10)
        self.assertEqual(
            [told for _, told in retry.told], [dt.timedelta(seconds=s) for s in (10, 30)]
        )

    def test_both_kinds_of_time_reach_the_driver_to_be_refused(self) -> None:
        retry = StandInRetry()
        vm_helpers.paced(retry, busy=lambda: 2.0)(nothing, 5, dt.timedelta(seconds=5))
        self.assertEqual(retry.told, [(5, dt.timedelta(seconds=5))])

    def test_a_wait_that_fails_describes_the_machine_then_fails(self) -> None:
        described: list[str] = []
        waits = vm_helpers.paced(
            StandInRetry(fails=True), busy=lambda: 1.0, describe=lambda: described.append("shown")
        )
        with self.assertRaisesRegex(AssertionError, "timed out"):
            waits(nothing, timeout_seconds=10)
        self.assertEqual(described, ["shown"])

    def test_a_wait_that_succeeds_describes_nothing(self) -> None:
        described: list[str] = []
        waits = vm_helpers.paced(
            StandInRetry(), busy=lambda: 1.0, describe=lambda: described.append("shown")
        )
        waits(nothing, timeout_seconds=10)
        self.assertEqual(described, [])


if __name__ == "__main__":
    unittest.main()
