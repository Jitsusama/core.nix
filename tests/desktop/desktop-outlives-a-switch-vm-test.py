"""The desktop test's outlives a switch area.

Here a switch of the home onto the sessions' kitty kept through a switch leaves it running, with
every session's window in it. The machine boots on the unit as it was before, with no switch
method, so a change restarted it and closed every session's window, as at 06:46 on 2026-11-09.
Its specialisation is the unit as modules/quickshell keeps it, with a changed command line as a
new kitty brings, so only the switch method keeps it.

And a switch changing the shell beside a unit slow to stop leaves the shell running until that
stop is done, then restarts it at once. Stopped with the slow unit, as a unit with no switch
method is, it waited for that stop before starting again: on 2026-11-10 disk-guard's stop ran
to its 90 s timeout, and the desktop had no launcher, notices or lock client all that time.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it, so any test runs alone.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import STARTING

if TYPE_CHECKING:
    from desktop_session import Desktop
    from test_driver.machine import BaseMachine

# The sessions' windows the test opens in the sessions' kitty, by their titles.
TITLES = ("Session one", "Session two")
# A window of the sessions' kitty, asked over its own socket as the shell asks.
IN_SESSIONS = "kitten @ --to unix:$XDG_RUNTIME_DIR/pi-sessions/kitty.sock launch --type=os-window"


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


# How long the slow unit's stop takes (tests/desktop.nix), and the most the shell may be gone.
SLOW_STOP = 30
SHELL_GONE_AT_MOST = 5


def shown(desktop: Desktop, unit: str, *names: str) -> dict[str, str]:
    """Give these of a user unit's properties, as systemd says them."""
    said = desktop.as_account(
        f"systemctl --user show {unit}" + "".join(f" -p {name}" for name in names)
    )
    return dict(line.split("=", 1) for line in said.splitlines() if "=" in line)


def kept(desktop: Desktop) -> tuple[str, str]:
    """Give the sessions' kitty's process and how often systemd restarted it."""
    fields = shown(desktop, "sessions-kitty", "MainPID", "NRestarts")
    return fields["MainPID"], fields["NRestarts"]


# The ids of what systemd says of a unit's jobs, from systemd's catalog: a stop begun, a stop
# done and a start done. A unit's own timestamps don't serve: one stopped when the units
# reload is dropped by the reload and made again with none.
STOPPING = "de5b426a63be47a7b6ac3eaac82e2f6f"
STOPPED = "9d1aaa27d60140bd96365438aad20286"
STARTED = "39f53479d3a045ac8e11786248231fbf"


def lasted(desktop: Desktop, unit: str, began: str, ended: str) -> float:
    """Give the seconds from what systemd last said a user unit began to what ended it."""
    said = desktop.as_account(
        f"journalctl --user -u {unit} -o json MESSAGE_ID={began} + MESSAGE_ID={ended}"
    )
    events = [json.loads(line) for line in said.splitlines() if line.startswith("{")]
    beginning = max(
        int(event["__MONOTONIC_TIMESTAMP"]) for event in events if event["MESSAGE_ID"] == began
    )
    ending = min(
        int(event["__MONOTONIC_TIMESTAMP"])
        for event in events
        if event["MESSAGE_ID"] == ended and int(event["__MONOTONIC_TIMESTAMP"]) > beginning
    )
    return (ending - beginning) / 1_000_000


def sessions(desktop: Desktop) -> dict[str, tuple[int, int]]:
    """Give each session's window niri lists, by its title, as its id and its process."""
    return {
        window["title"]: (window["id"], window["pid"])
        for window in desktop.windows()
        if window["title"] in TITLES
    }


def test_a_switch_keeps_the_sessions_kitty_and_its_windows(desktop: Desktop) -> None:
    """Switching onto the kept unit leaves the sessions' kitty and its windows as they were.

    The unit changes in a way that would restart it, but the new unit says keep-old, which is
    what sd-switch acts on, so the same process keeps the same windows.
    """
    machine = desktop.machine
    booted = machine.succeed("readlink -f /run/booted-system").strip()
    desktop.as_account("systemctl --user start sessions-kitty")
    machine.wait_until_succeeds(
        desktop.in_session("systemctl --user is-active sessions-kitty"), timeout=STARTING
    )
    unit = "systemctl --user cat sessions-kitty"
    try:
        assert "X-SwitchMethod" not in desktop.as_account(unit), (
            "the boot's unit says a switch method, so the switch isn't from the unit before"
        )
        for title in TITLES:
            desktop.as_account(
                f"niri msg action spawn -- {IN_SESSIONS} --os-window-class kitty"
                f" --title '{title}' sleep infinity"
            )
        machine.wait_until_succeeds(
            desktop.in_session(
                "niri msg --json windows"
                + "".join(f' | grep -F \'"title":"{title}"\'' for title in TITLES)
            ),
            timeout=STARTING,
        )
        before, restarts = kept(desktop)
        windows = sessions(desktop)
        assert {pid for _, pid in windows.values()} == {int(before)}, (
            f"the sessions' windows {json.dumps(windows)} aren't the kept kitty's, {before}"
        )

        machine.succeed(f"{booted}/specialisation/switched/bin/switch-to-configuration test")

        machine.succeed(
            "journalctl -b -u home-manager-jitsusama"
            " | grep -F 'Keeping old units: sessions-kitty.service'"
        )
        switched = desktop.as_account(unit)
        assert "X-SwitchMethod=keep-old" in switched, f"the switch didn't land: {switched}"
        assert "SESSIONS_KITTY_SWITCHED=1" in switched, f"the unit didn't change: {switched}"
        assert kept(desktop) == (before, restarts), (
            f"the sessions' kitty went from {before}, {restarts} restarts to {kept(desktop)}"
        )
        assert sessions(desktop) == windows, (
            f"the sessions' windows went from {json.dumps(windows)}"
            f" to {json.dumps(sessions(desktop))}"
        )
    finally:
        for window in desktop.windows():
            if window["title"] in TITLES:
                desktop.close(window)
        machine.succeed(f"{booted}/bin/switch-to-configuration test")


def test_a_switch_restarts_the_shell_once_a_slow_stop_is_done(desktop: Desktop) -> None:
    """A switch changing the shell beside a unit slow to stop leaves the shell gone a moment.

    The switch stops the slow unit, and every other it stops, in one batch and waits for the
    slowest. The shell, restarted rather than stopped with them, runs on until that batch is
    done, and its new process follows the old one at once. Through that batch the old shell
    runs over the new shell.qml, and loads it no second time.
    """
    machine = desktop.machine
    booted = machine.succeed("readlink -f /run/booted-system").strip()
    desktop.as_account("systemctl --user start slow-to-stop")
    machine.wait_until_succeeds(
        desktop.in_session("systemctl --user is-active slow-to-stop"), timeout=STARTING
    )
    old = shown(desktop, "quickshell", "MainPID")["MainPID"]
    try:
        machine.succeed(f"{booted}/specialisation/slowed/bin/switch-to-configuration test")

        assert "SHELL_SWITCHED=1" in desktop.as_account("systemctl --user cat quickshell"), (
            "the switch didn't change the shell's unit"
        )
        assert "The switch's own" in desktop.as_account("cat ~/.config/quickshell/shell.qml"), (
            "the switch didn't change the shell's files"
        )
        # Quickshell says each load, or reload, of its files, and each that fails, by this word.
        loads = [
            line
            for line in desktop.as_account(f"journalctl --user _PID={old} -o cat").splitlines()
            if "configuration" in line.lower()
        ]
        assert len(loads) == 1, f"the old shell loaded its files again in the switch: {loads}"
        stopping = lasted(desktop, "slow-to-stop", STOPPING, STOPPED)
        assert stopping > SLOW_STOP - 5, f"the slow unit stopped in {stopping:.1f} s, not slowly"
        new = shown(desktop, "quickshell", "MainPID")["MainPID"]
        assert new not in {old, "0"}, f"the shell is still {old}, or none"
        gone = lasted(desktop, "quickshell", STOPPING, STARTED)
        assert gone < SHELL_GONE_AT_MOST, (
            f"the shell was gone {gone:.1f} s of the switch, behind a {stopping:.1f} s stop"
        )
        machine.wait_until_succeeds(
            desktop.in_session("qs ipc show | grep -q launcher"), timeout=STARTING
        )
    finally:
        # Killed, the slow unit is no longer running for the switch back to stop.
        desktop.as_account("systemctl --user kill --signal=KILL slow-to-stop")
        machine.wait_until_fails(desktop.in_session("systemctl --user is-active slow-to-stop"))
        desktop.as_account("systemctl --user reset-failed slow-to-stop || true")
        machine.succeed(f"{booted}/bin/switch-to-configuration test")
        machine.wait_until_succeeds(
            desktop.in_session("qs ipc show | grep -q launcher"), timeout=STARTING
        )
