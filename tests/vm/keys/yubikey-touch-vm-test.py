"""A YubiKey's touch answering polkit only once the person is in, with canokey as the key.

yubikey-touch.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. Each test says what the shell's lock last
wrote before it asks, so any runs alone. run0, and sudo as the terminal types it, is asked by
typing into the desktop's terminal or a text console, and from an ssh login made at a console.

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

# Far longer than canokey takes to answer a touch, which is under a second.
TOUCHING = timedelta(seconds=5)

# What pam_exec logs each time the gate keeps the touch from being asked.
REFUSED = "journalctl --boot --output=cat | grep -c 'yubikey-once-in failed' || true"

# run0 itself, and sudo as the person's terminal has it, which goes through run0.
ASKINGS = [
    pytest.param("run0", id="run0"),
    pytest.param("sudo-by-touch", id="sudo"),
]

# Each asked from a text console of its own, since a console signed out of isn't asking for a
# login again by the time the next would sign in.
AT_CONSOLES = [
    pytest.param("run0", "2", id="run0"),
    pytest.param("sudo-by-touch", "4", id="sudo"),
]
OVER_SSH = [
    pytest.param("run0", "3", id="run0"),
    pytest.param("sudo-by-touch", "5", id="sudo"),
]


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

    def run0(self, asking: str = "run0") -> tuple[str, int]:
        """Ask run0, or what goes through it, in whatever has the keys.

        Gives the file it makes and the refusals before.
        """
        self.asked += 1
        made = f"/run/let-in-{self.asked}"
        before = self.refusals()
        self.machine.send_chars(f"{asking} touch {made}\n")
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

    def not_let_in(self, made: str) -> None:
        """See polkit's prompt open and, past the second a touch takes, nothing let in.

        The helper is a oneshot, so it's activating for as long as its prompt is open.
        """
        self.machine.wait_until_succeeds(
            "systemctl list-units --state=activating --no-legend 'polkit-agent-helper@*'"
            " | grep -q .",
            timeout=ANSWERING.seconds,
        )
        self.machine.fail(
            f"timeout {TOUCHING.seconds} sh -c 'until test -e {made}; do sleep 0.2; done'"
        )

    def types_the_password(self, made: str) -> None:
        """Type the password to the prompt waiting, and see it let run0 in."""
        self.machine.send_chars("typed\n")
        self.let_in(made)

    def signs_in_at(self, tty: str) -> None:
        """Sign in at that text console, beside the desktop, and have it take the keys."""
        machine = self.machine
        machine.succeed(f"chvt {tty}")
        machine.wait_until_tty_matches(tty, r"login:\s*\Z", timeout=ANSWERING.seconds)
        machine.send_chars("someone\n")
        machine.wait_until_tty_matches(tty, r"Password:\s*\Z", timeout=ANSWERING.seconds)
        machine.send_chars("typed\n")
        machine.wait_until_tty_matches(tty, r"\$\s*\Z", timeout=ANSWERING.seconds)

    def signs_out(self) -> None:
        """Leave the text console, and give the desktop the keys back."""
        self.machine.send_chars("exit\n")
        self.machine.succeed("chvt 1")


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


def test_a_terminals_sudo_takes_the_touch(seat: Seat) -> None:
    """The terminal's sudo goes through run0, so at the unlocked desktop the touch lets it in."""
    seat.lock_says("unlocked")
    made, _ = seat.run0("sudo-by-touch")
    seat.let_in(made)


@pytest.mark.parametrize("asking", ASKINGS)
@pytest.mark.parametrize(
    "word",
    [
        pytest.param("locked", id="the lock up"),
        pytest.param(None, id="no word from the lock"),
    ],
)
def test_the_password_is_asked_unless_unlocked(seat: Seat, word: str | None, asking: str) -> None:
    """With the lock up or saying nothing, the desktop is asked for the password, and it works."""
    seat.lock_says(word)
    made, before = seat.run0(asking)
    seat.asked_for_the_password(made, before)
    seat.types_the_password(made)


@pytest.mark.parametrize(("asking", "tty"), AT_CONSOLES)
def test_a_text_console_is_asked_for_the_password(seat: Seat, asking: str, tty: str) -> None:
    """From a text console signed in beside the unlocked desktop, only the password lets in."""
    seat.lock_says("unlocked")
    seat.signs_in_at(tty)
    made, before = seat.run0(asking)
    seat.asked_for_the_password(made, before)
    seat.types_the_password(made)
    seat.signs_out()


@pytest.mark.parametrize(("asking", "tty"), OVER_SSH)
def test_an_ssh_login_is_asked_for_the_password(seat: Seat, asking: str, tty: str) -> None:
    """From an ssh login, with the desktop unlocked, only the password lets in.

    ssh is made from a text console, whose screen shows polkit's prompt, so the password is
    typed only once the prompt is there: polkit throws away what was typed ahead.
    """
    machine = seat.machine
    seat.lock_says("unlocked")
    machine.succeed(
        "su someone -c 'mkdir -p -m 0700 ~/.ssh'",
        "su someone -c 'test -e ~/.ssh/id_ed25519"
        ' || ssh-keygen -q -t ed25519 -N "" -f ~/.ssh/id_ed25519\'',
        "su someone -c 'cp ~/.ssh/id_ed25519.pub ~/.ssh/authorized_keys'",
    )
    seat.signs_in_at(tty)
    made, before = seat.run0(
        f"ssh -t -o StrictHostKeyChecking=no -o BatchMode=yes localhost {asking}"
    )
    seat.asked_for_the_password(made, before)
    machine.wait_until_tty_matches(tty, r"Password:\s*\Z", timeout=ANSWERING.seconds)
    seat.types_the_password(made)
    seat.signs_out()


def test_with_no_key_the_password_lets_sudo_in(seat: Seat) -> None:
    """With the key pulled out at the unlocked desktop, sudo waits for the password, which works.

    Last, since the key isn't put back.
    """
    machine = seat.machine
    seat.lock_says("unlocked")
    machine.send_monitor_command("device_del key")
    machine.wait_until_fails("fido2-token -L | grep -q ^/dev/hidraw", timeout=ANSWERING.seconds)
    made, _ = seat.run0("sudo-by-touch")
    seat.not_let_in(made)
    seat.types_the_password(made)
