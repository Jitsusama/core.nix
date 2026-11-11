"""A YubiKey's touch answering polkit only once the person is in, with canokey as the key.

tests/yubikey-touch.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. Each test says what the shell's lock last
wrote before it asks, so any runs alone. run0 is asked by typing into the desktop's terminal or
the text console, and from an ssh login through ssh's own terminal.

The desktop's terminal can't be read as a console can, so a prompt that skipped the touch shows
as the gate's refusal in the journal, polkit then waiting for a password with nothing let in; a
touch taken lets run0 in before anything is typed.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING, cast

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine, QemuMachine

# How long polkit and the key may take to answer one run0.
ANSWERING = timedelta(seconds=30)

# What pam_exec logs each time the gate keeps the touch from being asked.
REFUSED = "journalctl --boot --output=cat | grep -c 'yubikey-once-in failed' || true"


class Seat:
    """The person signed in at the seat, with a key plugged in and enrolled for them."""

    def __init__(self, machine: QemuMachine, runtime: str) -> None:
        """Sit at that machine, whose shell's lock writes its word in that runtime folder."""
        self.machine = machine
        self.runtime = runtime
        self.asked = 0

    def lock_says(self, word: str | None) -> None:
        """Have the shell's lock have written that word last, or nothing at all."""
        marker = f"{self.runtime}/quickshell-locked"
        if word is None:
            self.machine.succeed(f"rm -f {marker}")
        else:
            self.machine.succeed(f"su someone -c 'printf \"%s\\n\" {word} > {marker}'")

    def refusals(self) -> int:
        """How many times the gate has kept the touch from being asked."""
        return int(self.machine.succeed(REFUSED).strip())

    def run0(self) -> tuple[str, int]:
        """Ask run0 in whatever has the keys, giving the file it makes and the refusals before."""
        self.asked += 1
        made = f"/run/let-in-{self.asked}"
        before = self.refusals()
        self.machine.send_chars(f"run0 touch {made}\n")
        return made, before

    def let_in(self, made: str) -> None:
        """Wait for run0 to have been let in."""
        self.machine.wait_until_succeeds(f"test -e {made}", timeout=ANSWERING.seconds)

    def asked_for_the_password(self, made: str, before: int) -> None:
        """Wait for the gate to have kept the touch from being asked, with nothing let in."""
        self.machine.wait_until_succeeds(
            f"test $({REFUSED}) -gt {before}", timeout=ANSWERING.seconds
        )
        self.machine.fail(f"test -e {made}")

    def types_the_password(self, made: str) -> None:
        """Type the password to the prompt waiting, and see it let run0 in."""
        self.machine.send_chars("typed\n")
        self.let_in(made)


@pytest.fixture(scope="module")
def seat(machine: BaseMachine) -> Seat:
    """Give the person at the desktop, the key found and enrolled for them."""
    qemu = cast("QemuMachine", machine)
    qemu.wait_for_unit("multi-user.target")
    uid = qemu.succeed("id -u someone").strip()
    qemu.wait_for_file(f"/run/user/{uid}/wayland-0.lock")
    qemu.wait_until_succeeds("pgrep -u someone -x bash")
    qemu.wait_until_succeeds(
        "loginctl show-session $(loginctl show-user someone --property=Display --value)"
        " --property=Type --value | grep -qx wayland"
    )
    qemu.wait_until_succeeds("fido2-token -L | grep -q ^/dev/hidraw")
    qemu.succeed(
        "pamu2fcfg --username=someone --origin=pam://administrators"
        " --appid=pam://administrators > /run/u2f-mappings"
    )
    qemu.succeed("chmod 0444 /run/u2f-mappings")
    return Seat(qemu, f"/run/user/{uid}")


def test_a_touch_answers_at_the_unlocked_desktop(seat: Seat) -> None:
    """From the desktop's terminal with the lock let go, the touch alone lets run0 in."""
    seat.lock_says("unlocked")
    made, _ = seat.run0()
    seat.let_in(made)


@pytest.mark.parametrize(
    "word",
    [
        pytest.param("locked", id="the lock up"),
        pytest.param(None, id="no word from the lock"),
    ],
)
def test_the_password_is_asked_unless_unlocked(seat: Seat, word: str | None) -> None:
    """With the lock up or saying nothing, the desktop is asked for the password, and it works."""
    seat.lock_says(word)
    made, before = seat.run0()
    seat.asked_for_the_password(made, before)
    seat.types_the_password(made)


def test_a_text_console_is_asked_for_the_password(seat: Seat) -> None:
    """From a text console signed in beside the unlocked desktop, the touch never lets run0 in."""
    machine = seat.machine
    seat.lock_says("unlocked")
    machine.succeed("chvt 2")
    machine.wait_until_tty_matches("2", r"login:\s*\Z", timeout=ANSWERING.seconds)
    machine.send_chars("someone\n")
    machine.wait_until_tty_matches("2", r"Password:\s*\Z", timeout=ANSWERING.seconds)
    machine.send_chars("typed\n")
    machine.wait_until_tty_matches("2", r"\$\s*\Z", timeout=ANSWERING.seconds)
    made, before = seat.run0()
    seat.asked_for_the_password(made, before)
    seat.types_the_password(made)
    machine.send_chars("exit\n")
    machine.succeed("chvt 1")


def test_an_ssh_login_is_asked_for_the_password(seat: Seat) -> None:
    """From an ssh login, with the desktop unlocked, the touch never lets run0 in.

    Nothing is typed, since polkit's prompt throws away what was typed ahead: a touch taken would
    let run0 in before the prompt reads the end of what ssh was given.
    """
    machine = seat.machine
    seat.lock_says("unlocked")
    machine.succeed(
        "test -e /root/key || ssh-keygen -q -t ed25519 -N '' -f /root/key",
        "install -d -o someone -m 0700 /home/someone/.ssh",
        "install -o someone -m 0600 /root/key.pub /home/someone/.ssh/authorized_keys",
    )
    status, said = machine.execute(
        f"timeout {ANSWERING.seconds} ssh -tt -i /root/key -o StrictHostKeyChecking=no"
        " -o BatchMode=yes someone@localhost run0 true < /dev/null 2>&1"
    )
    assert status != 0, said
    assert "Password" in said, said
