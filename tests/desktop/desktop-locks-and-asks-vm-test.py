"""The desktop test's locks and asks area.

Here the desktop locks and asks: the lock screen and what lets it go, polkit's prompt and the
askpass.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it: unlocked, the shell running, nothing of its
own still open, so any test runs alone.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import PICTURE_COLOURS, SIGNING, STARTING, retry

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop
    from test_driver.machine import BaseMachine

# Where lock-screen and the shell say whether the screens are locked.
LOCKED = "$XDG_RUNTIME_DIR/quickshell-locked"


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture(autouse=True)
def launcher_closed_after(desktop: Desktop) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    desktop.launcher_left_closed()


def test_the_lock_screen_takes_only_the_right_password(desktop: Desktop) -> None:
    """The lock screen takes only the right password, and shows its picture meanwhile."""
    machine = desktop.machine
    with desktop.told("keys lock"):
        machine.send_key("meta_l-l")
    desktop.as_account("qs ipc call lock isLocked | grep -q true")
    # PAM answers a wrong password after a delay, which the lock says.
    with desktop.told("refused lock"):
        machine.send_chars("wrong-password\n")
    desktop.as_account("qs ipc call lock isLocked | grep -q true")

    # Its picture fades in, as the wallpaper's does.
    def pictured(last: bool) -> bool:  # As retry calls it.
        """Say whether the lock screen shows its picture rather than a flat colour."""
        _, colours, _ = desktop.colours_and_light("locked")
        assert not last or colours > PICTURE_COLOURS, "the lock screen shows a flat colour"
        return colours > PICTURE_COLOURS

    retry(pictured, timeout=timedelta(seconds=30))
    with desktop.told("released lock"):
        machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.as_account("qs ipc call lock isLocked | grep -q false")


def test_lock_screen_returns_once_the_lock_is_drawn(desktop: Desktop) -> None:
    """What locks before the machine sleeps returns only once niri has drawn the lock.

    So nothing else shows on waking.
    """
    desktop.as_account(f"grep -qx unlocked {LOCKED}")
    desktop.as_account("lock-screen")
    desktop.as_account(f"grep -qx locked {LOCKED}")
    with desktop.told("released lock"):
        desktop.machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")


def test_the_lock_lets_go_without_its_password_when_released(desktop: Desktop) -> None:
    """The lock lets go without its password when released, and starts empty the next time.

    swayidle releases it when logind says to unlock, as after a resume from hibernation, where
    the disk's prompt has already asked for the password.
    """
    desktop.as_account("lock-screen")
    desktop.as_account(f"grep -qx locked {LOCKED}")
    # Half a password, left in the field.
    desktop.machine.send_chars("half-a-pass")
    with desktop.told("released lock"):
        desktop.as_account("qs ipc call lock release")
    desktop.as_account("qs ipc call lock isLocked | grep -qx false")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")
    # Released again while unlocked, nothing happens.
    desktop.as_account("qs ipc call lock release")
    desktop.as_account("qs ipc call lock isLocked | grep -qx false")
    # The next lock takes the password alone, so nothing was left behind.
    desktop.as_account("lock-screen")
    with desktop.told("released lock"):
        desktop.machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")


def test_logind_saying_to_unlock_lets_the_lock_go(desktop: Desktop) -> None:
    """Logind saying to unlock lets the lock go.

    As after a resume from hibernation, where the disk's prompt has already asked for the
    password, so the lock doesn't ask again.
    """
    desktop.as_account("lock-screen")
    desktop.as_account(f"grep -qx locked {LOCKED}")
    with desktop.told("released lock"):
        desktop.machine.succeed("loginctl unlock-sessions")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")


# Where swayidle-away marks someone at the machine.
HERE = "$XDG_RUNTIME_DIR/someone-at-the-machine"

# swayidle-away's own seconds, made short: the mark, the lock, and the screens going off, late
# enough that the test never turns them off.
SOON = "2 4 3600"


def test_the_screen_locks_as_joel_steps_away_past_holds_that_keep_it_awake(
    desktop: Desktop,
) -> None:
    """The screen locks as Joel steps away though mycelium holds logind's idle off.

    A swayidle that hears logind drops every timeout while anything holds logind's lock
    against going idle, and mycelium holds it whenever it holds work, so the screen went
    unlocked for as long as Joel was away (2026-11-10). swayidle-away asks logind nothing.
    The test runs it with short times, since typing by wtype counts as no key here.
    """
    machine = desktop.machine
    desktop.as_account(f"grep -qx unlocked {LOCKED}")
    desktop.as_account(
        "systemd-run --user --unit=holds-sessions systemd-inhibit --what=idle --mode=block"
        " --who=mycelium '--why=It holds sessions for the realm' sleep infinity"
    )
    try:
        desktop.as_account(f"touch {HERE}")
        with desktop.told("secured lock", timeout=timedelta(seconds=60)):
            desktop.as_account(f"systemd-run --user --unit=away-soon swayidle-away {SOON}")
        desktop.as_account(f"grep -qx locked {LOCKED}")
        desktop.as_account(f"test ! -e {HERE}")
        # Each lock says why it came and that it was drawn, so a lock that stops coming
        # shows as a gap in the journal.
        desktop.as_account("journalctl --user -u away-soon | grep -q 'locking: Joel stepped away'")
        machine.wait_until_succeeds(
            desktop.in_session("journalctl --user -u quickshell | grep -q 'lock: drawn'"),
            timeout=timedelta(seconds=30),
        )
        with desktop.told("released lock"):
            desktop.as_account("qs ipc call lock release")
        machine.wait_until_succeeds(
            desktop.in_session("journalctl --user -u quickshell | grep -q 'lock: lifted'"),
            timeout=timedelta(seconds=30),
        )
    finally:
        desktop.as_account("systemctl --user stop away-soon holds-sessions || true")
        desktop.as_account("qs ipc call lock release")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")


def test_a_restart_of_swayidle_away_marks_nobody_at_the_machine(desktop: Desktop) -> None:
    """Restarting swayidle-away marks nobody at the machine, and keeps a mark already there.

    A switch restarts swayidle-away, whose start used to touch the mark, so every switch read
    as Joel at the machine for two minutes and stopped what waits for him to be away
    (2026-11-11). Only a key or a touch marks him now, and typing by wtype counts as none here.
    """
    desktop.as_account(f"rm -f {HERE}")
    try:
        desktop.as_account("systemctl --user start swayidle-away")
        desktop.as_account("systemctl --user restart swayidle-away")
        desktop.as_account("systemctl --user is-active swayidle-away")
        desktop.as_account(f"test ! -e {HERE}")
        desktop.as_account(f"touch {HERE}")
        desktop.as_account("systemctl --user restart swayidle-away")
        desktop.as_account(f"test -e {HERE}")
    finally:
        desktop.as_account("systemctl --user stop swayidle-away || true")
        desktop.as_account(f"rm -f {HERE}")


def test_keep_awake_keeps_the_screen_unlocked_as_joel_steps_away(desktop: Desktop) -> None:
    """Keep Awake keeps the screen on and unlocked as Joel steps away, as a call's hold does."""
    machine = desktop.machine
    desktop.as_account(f"grep -qx unlocked {LOCKED}")
    desktop.as_account("systemctl --user start keep-awake.service")
    try:
        desktop.as_account(f"systemd-run --user --unit=away-kept swayidle-away {SOON}")
        machine.wait_until_succeeds(
            desktop.in_session(
                "journalctl --user -u away-kept | grep -q 'the screen stays: Keep Awake holds it'"
            ),
            timeout=timedelta(seconds=60),
        )
        desktop.as_account(f"grep -qx unlocked {LOCKED}")
    finally:
        desktop.as_account("systemctl --user stop away-kept keep-awake.service || true")
        desktop.as_account("qs ipc call lock release")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")


def test_polkits_prompt_lets_the_account_start_a_system_service(desktop: Desktop) -> None:
    """Polkit's prompt lets the account start a system service."""
    machine = desktop.machine
    with desktop.told("keys polkit"):
        desktop.as_account("niri msg action spawn -- systemctl start polkit-allowed.service")
    desktop.shoot("polkit")
    machine.send_chars(f"{desktop.values['secret']}\n")
    machine.wait_for_file("/run/polkit-allowed", timeout=timedelta(seconds=30))


def test_the_askpass_asks_for_a_yes_a_pin_and_a_touch(desktop: Desktop) -> None:
    """The askpass asks for a yes, a PIN and a touch, each answered as OpenSSH expects."""
    machine = desktop.machine
    with desktop.told("keys askpass"):
        desktop.ask("confirm", SIGNING)
    desktop.shoot("askpass")
    machine.send_key("ret")
    desktop.answered_with("confirm", 0)

    with desktop.told("keys askpass"):
        desktop.ask("confirm", SIGNING)
    with desktop.told("gone askpass"):
        machine.send_key("esc")
    desktop.answered_with("confirm", 1)

    with desktop.told("keys askpass"):
        desktop.ask(
            "secret", f"Enter PIN for ED25519-SK key /home/{desktop.account}/.ssh/id_ed25519_sk: "
        )
    desktop.shoot("askpass-pin")
    machine.send_chars("2468\n")
    desktop.answered_with("secret", 0, "2468\n")

    # A touch takes no keys, and goes once ssh stops waiting on it.
    with desktop.told("drawn askpass"):
        desktop.ask("none", "Confirm user presence for key ED25519-SK SHA256:abc")
    with desktop.told("gone askpass"):
        desktop.as_account("pkill -TERM -f '^[^ ]*bash [^ ]*ssh-askpass Confirm'")
    desktop.answered_with("none", 1)
    desktop.as_account("! ls $XDG_RUNTIME_DIR | grep -q '^askpass'")


def test_the_askpass_finds_the_shell_for_a_program_started_before_the_desktop(
    desktop: Desktop,
) -> None:
    """The askpass finds the shell for a program started before the desktop.

    linux-id's service starts with the account's systemd, before the desktop says which
    display it's on, and an askpass it runs knows no display.
    """
    with desktop.told("keys askpass"):
        desktop.as_account(
            "systemd-run --user --collect --quiet -- env -u WAYLAND_DISPLAY "
            f"{desktop.values['asking']} confirm '{SIGNING}'"
        )
    desktop.machine.send_key("ret")
    desktop.answered_with("confirm", 0)


def test_the_askpass_asks_a_restarted_shell_again_and_gives_up_on_a_gone_one(
    desktop: Desktop,
) -> None:
    """The askpass asks a restarted shell again, and gives up on a gone one.

    The shell holds an ask only in memory, as a switch restarts it.
    """
    with desktop.told("keys askpass"):
        desktop.ask("confirm", SIGNING)
    with desktop.told("keys askpass", timeout=timedelta(seconds=60)):
        desktop.as_account("systemctl --user restart quickshell")
    desktop.machine.send_key("ret")
    desktop.answered_with("confirm", 0)

    with desktop.told("keys askpass"):
        desktop.ask("confirm", SIGNING)
    desktop.as_account("systemctl --user stop quickshell")
    desktop.answered_with("confirm", 1, timeout=timedelta(seconds=60))
    desktop.as_account("! ls $XDG_RUNTIME_DIR | grep -q '^askpass'")
    desktop.as_account("systemctl --user start quickshell")
    desktop.machine.wait_until_succeeds(
        desktop.in_session("qs ipc show | grep -q askpass"), timeout=STARTING
    )


def test_the_lock_holds_when_the_shell_restarts_and_only_for_its_niri(desktop: Desktop) -> None:
    """The lock holds when the shell restarts, and only for its niri.

    A switch restarts the shell, and can come while Joel is away and the screens are locked.
    niri holds a lock whose shell died, so the shell it starts again has to take that lock
    back, or nothing is left to type the password into.
    """
    holder = "$XDG_RUNTIME_DIR/quickshell-lock-holder"
    desktop.as_account("lock-screen")
    since = desktop.mark()
    desktop.as_account("systemctl --user restart quickshell")
    for word in ("secured lock", "keys lock"):
        desktop.reported(word, lambda rest: rest == "", since, timeout=timedelta(seconds=60))
    desktop.as_account("qs ipc call lock isLocked | grep -qx true")
    desktop.as_account(f"grep -qx locked {LOCKED}")
    with desktop.told("released lock"):
        desktop.machine.send_chars(f"{desktop.values['secret']}\n")
    desktop.as_account(f"grep -qx unlocked {LOCKED}")

    def started_unlocked() -> None:
        """Restart the shell, and check that it starts with the screens unlocked."""
        desktop.as_account("systemctl --user restart quickshell")
        desktop.machine.wait_until_succeeds(
            desktop.in_session("qs ipc show | grep -q askpass"), timeout=STARTING
        )
        desktop.as_account("qs ipc call lock isLocked | grep -qx false")
        desktop.as_account(f"grep -qx unlocked {LOCKED}")

    # Let go, the lock isn't taken again.
    started_unlocked()
    # The runtime directory outlives a session while the account
    # lingers, so a lock a crashed niri held is left to that niri.
    desktop.as_account(f"echo $XDG_RUNTIME_DIR/niri.wayland-1.1.sock > {holder}")
    desktop.as_account(f"echo locked > {LOCKED}")
    started_unlocked()


def test_quickshell_warned_of_nothing_it_drew_all_along(desktop: Desktop) -> None:
    """Quickshell warned of nothing it drew all along.

    A binding that throws or assigns the wrong type only warns, and only once a surface draws
    it, which can be long after the start, so this comes last.
    """
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = desktop.as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"
