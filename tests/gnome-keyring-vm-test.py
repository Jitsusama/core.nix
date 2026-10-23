"""A keyring in a virtual machine that opens with its TPM-sealed password, asking nobody.

tests/gnome-keyring.nix runs this, and its opening comment says what the test proves.
run-subtests cuts the script into subtests at each top-level `with subtest(`, so every
subtest stays at the top level under its own name, and what sits outside them, the boot and
the helpers, runs whichever subtests are chosen. The account's runtime folder is
gnome-keyring.nix's, so it comes from the JSON file it writes, whose path it puts in place of
the marker below.
"""

import json
import shlex
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

# Joel's runtime folder, which holds his session's bus, as gnome-keyring.nix has it.
values = json.loads(Path("@values@").read_text())
runtime: str = values["runtime"]
# How a credential's header opens, as od writes it, when systemd sealed it with the TPM's key
# and the host's, scoped to one account.
TPM_AND_HOST = "ef 4a c1 36 79 a9 48 0e a7 db 68 89 7f 9f 16 5d"


def as_joel(command: str) -> str:
    """Run a command as Joel, in his home and on his session's bus, and give what it said."""
    environment = (
        f"export XDG_RUNTIME_DIR={runtime} DBUS_SESSION_BUS_ADDRESS=unix:path={runtime}/bus; cd ~; "
    )
    return machine.succeed("su - joel -c " + shlex.quote(environment + command))


def log_in() -> None:
    """Start the account's default target, as its manager does when it signs in.

    That brings the keyring up beside the compositor.
    """
    as_joel("systemctl --user start default.target compositor.service")
    keyring_is_up()


def keyring_is_up() -> None:
    """Wait until the keyring's unit is up, and check its daemon is alone and reaps its own."""
    as_joel(
        "timeout 30 sh -c 'until systemctl --user is-active -q"
        " gnome-keyring.service; do sleep 0.2; done'"
    )
    # The unit's daemon is the only one: none started beside it. The
    # brackets keep this command's own shell from matching.
    daemons = "pgrep -u joel -f '[b]in/gnome-keyring-daemon'"
    as_joel(
        f"test $({daemons} | wc -l) = 1 && {daemons} | grep -qx"
        " $(systemctl --user show -p MainPID --value gnome-keyring.service)"
    )
    # The daemon never reaps a child it didn't start, so none may be left
    # to it by the script that started it.
    as_joel(
        "! ps -o stat= --ppid"
        " $(systemctl --user show -p MainPID --value gnome-keyring.service)"
        " | grep -q Z"
    )


def log_out() -> None:
    """Stop the compositor and the keyring, as signing out does."""
    as_joel("systemctl --user stop compositor.service gnome-keyring.service")


machine.wait_for_unit("multi-user.target")
machine.succeed("loginctl enable-linger joel")
machine.wait_for_unit("user@1000.service")

# A target waits for every unit it wants, so the keyring wanted by one
# of the session's would hold the compositor back while the TPM unseals.
with subtest("the session doesn't wait for the keyring"):
    for target in ["graphical-session-pre.target", "graphical-session.target"]:
        after = as_joel(f"systemctl --user show -p After --value {target}")
        t.assertNotIn("gnome-keyring.service", after.split(), f"{target} waits for it")

with subtest("the keyring's password is sealed to the TPM at the first login"):
    log_in()
    # The header's first 16 bytes name the key: the TPM's and the host's,
    # scoped to one account. systemd falls back to the host's alone,
    # silently, when it can't reach the TPM.
    key = as_joel("base64 -d ~/.local/state/gnome-keyring/login.cred | head -c 16 | od -An -tx1")
    t.assertEqual(key.split(), TPM_AND_HOST.split())

with subtest("a secret is stored without a prompt, and not in the clear"):
    as_joel(
        "printf hunter2 | timeout 10 secret-tool store --label=Slack service slack account joel"
    )
    as_joel("test -s ~/.local/share/keyrings/login.keyring")
    as_joel("! grep -q hunter2 ~/.local/share/keyrings/login.keyring")

with subtest("the next login reads it back without a prompt"):
    log_out()
    log_in()
    secret = as_joel("timeout 10 secret-tool lookup service slack account joel")
    t.assertEqual(secret, "hunter2")

# A program can ask before the keyring is up. Asking starts the unit, and
# the answer waits for it to unlock, where gnome-keyring's own D-Bus
# files would start a second daemon with the keyring locked.
with subtest("a program that asks first gets the unlocked keyring"):
    log_out()
    secret = as_joel("timeout 10 secret-tool lookup service slack account joel")
    t.assertEqual(secret, "hunter2")
    keyring_is_up()

# PAM's keyring step would start a daemon of its own at every sign-in,
# before the session, with a password that never opens the keyring.
with subtest("signing in starts no keyring of PAM's"):
    machine.wait_until_tty_matches("1", "login: ")
    machine.send_chars("joel\n")
    machine.wait_until_tty_matches("1", "Password: ")
    machine.send_chars("hunter3\n")
    # The shell's prompt comes once every PAM step has run.
    machine.wait_until_tty_matches("1", r"joel@machine.*\$ ", timeout=60)
    t.assertEqual(machine.succeed("journalctl -b -q -o cat --grep gkr-pam || true"), "")
    keyring_is_up()
