"""A keyring in a virtual machine that opens with its TPM-sealed password, asking nobody.

tests/gnome-keyring.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. Each test asks for the state it starts
from as a fixture: Joel's account up, then signed in once, which seals the keyring's password,
then a secret stored. A fixture's work is done once a run, however many tests ask for it, so
any test runs alone with `-k`. Joel's runtime folder is gnome-keyring.nix's, given as the
`values` fixture.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import pytest
from vm_helpers import as_account

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine, QemuMachine

# How a credential's header opens, as od writes it, when systemd sealed it with the TPM's key
# and the host's, scoped to one account.
TPM_AND_HOST = "ef 4a c1 36 79 a9 48 0e a7 db 68 89 7f 9f 16 5d"
# The secret stored, as Slack stores one.
SLACKS = "hunter2"
LOOKUP = "timeout 10 secret-tool lookup service slack account joel"


class Joel:
    """Joel's account, in his home and on his session's bus."""

    def __init__(self, machine: QemuMachine, runtime: str) -> None:
        """Act on Joel's account on that machine, his runtime folder that one."""
        self.machine = machine
        self.runtime = runtime

    def run(self, command: str) -> str:
        """Run a command as Joel, and give what it said."""
        return as_account(self.machine, command, runtime=self.runtime, bus=True)

    def log_in(self) -> None:
        """Start the account's default target, as its manager does when it signs in.

        That brings the keyring up beside the compositor.
        """
        self.run("systemctl --user start default.target compositor.service")
        self.keyring_is_up()

    def keyring_is_up(self) -> None:
        """Wait until the keyring's unit is up, and check its daemon is alone and reaps its own."""
        self.run(
            "timeout 30 sh -c 'until systemctl --user is-active -q"
            " gnome-keyring.service; do sleep 0.2; done'"
        )
        # The unit's daemon is the only one: none started beside it. The
        # brackets keep this command's own shell from matching.
        daemons = "pgrep -u joel -f '[b]in/gnome-keyring-daemon'"
        self.run(
            f"test $({daemons} | wc -l) = 1 && {daemons} | grep -qx"
            " $(systemctl --user show -p MainPID --value gnome-keyring.service)"
        )
        # The daemon never reaps a child it didn't start, so none may be left
        # to it by the script that started it.
        self.run(
            "! ps -o stat= --ppid"
            " $(systemctl --user show -p MainPID --value gnome-keyring.service)"
            " | grep -q Z"
        )

    def log_out(self) -> None:
        """Stop the compositor and the keyring, as signing out does."""
        self.run("systemctl --user stop compositor.service gnome-keyring.service")


@pytest.fixture(scope="module")
def joel(machine: BaseMachine, values: dict[str, Any]) -> Joel:
    """Give Joel's account, its service manager running and nobody signed in."""
    # The test's one machine is QEMU's, which takes keys at its console.
    qemu = cast("QemuMachine", machine)
    qemu.wait_for_unit("multi-user.target")
    qemu.succeed("loginctl enable-linger joel")
    qemu.wait_for_unit("user@1000.service")
    return Joel(qemu, values["runtime"])


@pytest.fixture(scope="module")
def signed_in(joel: Joel) -> Joel:
    """Give Joel's account once it has signed in for the first time."""
    joel.log_in()
    return joel


@pytest.fixture(scope="module")
def stored(signed_in: Joel) -> Joel:
    """Give Joel's account with a secret stored as Slack stores one."""
    signed_in.run(
        f"printf {SLACKS} | timeout 10 secret-tool store --label=Slack service slack account joel"
    )
    return signed_in


def test_the_session_doesnt_wait_for_the_keyring(joel: Joel) -> None:
    """The session doesn't wait for the keyring.

    A target waits for every unit it wants, so the keyring wanted by one of the session's
    would hold the compositor back while the TPM unseals.
    """
    for target in ["graphical-session-pre.target", "graphical-session.target"]:
        after = joel.run(f"systemctl --user show -p After --value {target}")
        assert "gnome-keyring.service" not in after.split(), target


def test_the_password_is_sealed_to_the_tpm_at_the_first_login(signed_in: Joel) -> None:
    """The keyring's password is sealed to the TPM at the first login.

    The header's first 16 bytes name the key: the TPM's and the host's, scoped to one
    account. systemd falls back to the host's alone, silently, when it can't reach the TPM.
    """
    key = signed_in.run(
        "base64 -d ~/.local/state/gnome-keyring/login.cred | head -c 16 | od -An -tx1"
    )
    assert key.split() == TPM_AND_HOST.split()


def test_a_secret_is_stored_without_a_prompt_and_not_in_the_clear(stored: Joel) -> None:
    """A secret is stored without a prompt, and not in the clear."""
    stored.run("test -s ~/.local/share/keyrings/login.keyring")
    stored.run(f"! grep -q {SLACKS} ~/.local/share/keyrings/login.keyring")


def test_the_next_login_reads_it_back_without_a_prompt(stored: Joel) -> None:
    """The next login reads it back without a prompt."""
    stored.log_out()
    stored.log_in()
    assert stored.run(LOOKUP) == SLACKS


def test_a_program_that_asks_first_gets_the_unlocked_keyring(stored: Joel) -> None:
    """A program that asks first gets the unlocked keyring.

    A program can ask before the keyring is up. Asking starts the unit, and the answer waits
    for it to unlock, where gnome-keyring's own D-Bus files would start a second daemon with
    the keyring locked.
    """
    stored.log_out()
    assert stored.run(LOOKUP) == SLACKS
    stored.keyring_is_up()


def test_signing_in_starts_no_keyring_of_pams(signed_in: Joel) -> None:
    """Signing in starts no keyring of PAM's.

    PAM's keyring step would start a daemon of its own at every sign-in, before the session,
    with a password that never opens the keyring.
    """
    machine = signed_in.machine
    machine.wait_until_tty_matches("1", "login: ")
    machine.send_chars("joel\n")
    machine.wait_until_tty_matches("1", "Password: ")
    machine.send_chars("hunter3\n")
    # The shell's prompt comes once every PAM step has run.
    machine.wait_until_tty_matches("1", r"joel@machine.*\$ ", timeout=timedelta(seconds=60))
    assert machine.succeed("journalctl -b -q -o cat --grep gkr-pam || true") == ""
    signed_in.keyring_is_up()
