"""Writing firmware and restarting into its setup ask an administrator, even at the seat.

firmware-asks.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py). someone, in wheel, is signed in on the first console, so its shell
is in an active, local session: the most polkit gives without asking, and what an agent
running as the account counts as. pkcheck asks polkit about that shell as root would, without
letting it ask anyone, so an action that needs an answer comes back as needing one. A plain
restart and a check of the firmware still need none, which also shows the session is the
active one. The answer is typed at the console to pkcheck's own agent, through polkit's
sign-in, where a YubiKey is named and not there, so the password stands in for its touch;
it's kept for that action alone, and the test takes it back as it ends, so any runs alone.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import QemuMachine

FWUPD = "org.freedesktop.fwupd"
SETUP = "org.freedesktop.login1.set-reboot-to-firmware-setup"
# pkcheck's status when polkit would ask for an answer it wasn't let ask.
ASKS = 2
# What pkcheck says of an answer polkit keeps a few minutes, a dot in a detail's name written
# as its octal escape.
KEPT = "polkit\\56retains_authorization_after_challenge=1"
# Where the console's pkcheck says its status, in someone's own folder.
ANSWERED = "/home/someone/answered"

# What writes firmware, changes a device's settings or opens the firmware's setup, each of
# which polkit or fwupd's own rules gave the seat's session with no answer.
ASKED = [
    f"{FWUPD}.update-internal",
    f"{FWUPD}.update-internal-trusted",
    f"{FWUPD}.update-hotplug",
    f"{FWUPD}.update-hotplug-trusted",
    f"{FWUPD}.downgrade-internal",
    f"{FWUPD}.downgrade-internal-trusted",
    f"{FWUPD}.downgrade-hotplug",
    f"{FWUPD}.downgrade-hotplug-trusted",
    f"{FWUPD}.modify-device",
    SETUP,
]


def shell(machine: QemuMachine) -> int:
    """Give the process id of someone's shell on the first console, once it's signed in."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_until_tty_matches("1", "someone@machine")
    return int(machine.succeed("pgrep -o -u someone -t tty1 -x bash").strip())


def check(machine: QemuMachine, action: str, pid: int) -> tuple[int, str]:
    """Ask polkit whether the process may do the action, letting it ask nobody."""
    return machine.execute(f"pkcheck --action-id {action} --process {pid} 2>&1")


@pytest.fixture
def seated(machine: QemuMachine) -> int:
    """Give the process id of the shell in someone's session at the seat."""
    return shell(machine)


@pytest.mark.parametrize("action", ASKED)
def test_the_seat_must_answer(machine: QemuMachine, seated: int, action: str) -> None:
    """Each needs an administrator's answer, kept a few minutes, even in the seat's session."""
    status, said = check(machine, action, seated)
    assert status == ASKS, f"{action} gave {status}: {said}"
    assert KEPT in said, said


@pytest.mark.parametrize(
    "action",
    [f"{FWUPD}.fix-host-security-attr", f"{FWUPD}.undo-host-security-attr"],
)
def test_a_security_change_asks_each_time(machine: QemuMachine, seated: int, action: str) -> None:
    """Fixing or undoing a security attribute still asks, and keeps no answer."""
    status, said = check(machine, action, seated)
    assert status == ASKS, f"{action} gave {status}: {said}"
    assert KEPT not in said, said


@pytest.mark.parametrize(
    "action",
    ["org.freedesktop.login1.reboot", f"{FWUPD}.verify"],
)
def test_the_seat_still_restarts_and_checks_unasked(
    machine: QemuMachine, seated: int, action: str
) -> None:
    """A plain restart and a check of the firmware need no answer in the seat's session."""
    status, said = check(machine, action, seated)
    assert status == 0, f"{action} gave {status}: {said}"


@pytest.fixture
def answered(machine: QemuMachine) -> Iterator[int]:
    """Answer polkit for an update from the console, and take the answer back as it ends."""
    pid = shell(machine)
    machine.succeed(f"rm -f {ANSWERED}")
    machine.send_chars(
        f"pkcheck -u --enable-internal-agent --action-id {FWUPD}.update-internal "
        f"--process $$; echo $? > {ANSWERED}\n"
    )
    machine.wait_until_tty_matches("1", "Password")
    machine.send_chars("typed\n")
    machine.wait_for_file(ANSWERED)
    yield pid
    # polkit keeps what it was answered in memory alone, so restarting it forgets the answer.
    machine.succeed("systemctl restart polkit")
    status, said = check(machine, f"{FWUPD}.update-internal", pid)
    assert status == ASKS, f"the answer outlived polkit: {status}: {said}"


def test_the_password_answers_for_that_action_alone(machine: QemuMachine, answered: int) -> None:
    """The answer lets the update through, is kept for it, and answers nothing else."""
    said = machine.succeed(f"cat {ANSWERED}").strip()
    assert said == "0", f"pkcheck's answered update gave {said}"
    status, kept = check(machine, f"{FWUPD}.update-internal", answered)
    assert status == 0, f"the answer wasn't kept: {status}: {kept}"
    status, other = check(machine, SETUP, answered)
    assert status == ASKS, f"the answer answered the setup too: {status}: {other}"
