"""Test what VM tests' scripts share (vm_helpers.py), against a stand-in for a machine.

A helper's whole job is the command line it hands the driver, so each test reads that line
from a machine that only records it, which keeps these tests to milliseconds.
"""

import shlex
import unittest
from typing import TYPE_CHECKING, cast

import vm_helpers

if TYPE_CHECKING:
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


if __name__ == "__main__":
    unittest.main()
