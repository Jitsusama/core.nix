"""The screens test's keep-the-launcher area.

The launcher opens, finds and runs a row on each of two screens, and keeps working as either or
both go and come back, and through a sleep.

tests/screens.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The machine and each step it's taken
through are screens_session.py's fixtures, which every test asks for by name.
"""

from __future__ import annotations

import datetime as dt
import time
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from screens_session import Screens

pytest_plugins = ["screens_session"]


def test_the_launcher_opens_finds_and_runs_a_row_on_each_screen(unlocked: Screens) -> None:
    """The launcher opens, finds and runs a row on each of two screens."""
    unlocked.launcher_works_on_both()


def test_a_return_held_does_its_row_once(unlocked: Screens) -> None:
    """A Return held, or let go late by a busy machine, does its row once.

    A key let go past the repeat delay repeats as if held (modules/keyboard/repeat.nix), and
    this machine keeps repeat on, as Joel's does. Each repeat ran the row again: on a loaded
    host Keep Awake was started and stopped within half a second, and the next check found it
    off.
    """
    screens = unlocked
    machine = screens.machine
    machine.succeed(screens.in_niri("systemctl --user stop caffeinate.service"))
    since = machine.succeed("date +%s").strip()
    # Starts alone are counted, since the stop above may share the mark's second: one repeat
    # turns it off again, which the last look finds, and two start it twice.
    started = screens.in_niri(
        f"journalctl --user -u caffeinate.service --since @{since} -o cat"
        " | grep -c '^Starting ' || true"
    )
    try:
        screens.super_space_on("Virtual-1")
        machine.send_chars("keep awake")
        screens.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name") == "Keep Awake",
            "found 'Keep Awake'",
        )
        # Held a second, past the 200 ms delay at 40 a second, it repeats some thirty times.
        held = 1.0
        machine.send_monitor_command(f"sendkey ret {int(held * 1000)}")
        deadline = time.monotonic() + 10
        while not screens.awake():
            assert time.monotonic() < deadline, "a held Return never started Keep Awake"
            time.sleep(0.2)
        # Watched for as long as it's held and a second after, since a repeat comes only then.
        deadline = time.monotonic() + held + 1
        while time.monotonic() < deadline:
            said = int(machine.succeed(started))
            assert said == 1, f"a held Return started Keep Awake {said} times"
            time.sleep(0.2)
        assert screens.awake(), "a held Return turned Keep Awake on and off again"
    finally:
        machine.send_key("esc")
        screens.launcher_until(lambda shown: not shown["open"], "closed at Escape")
        machine.succeed(screens.in_niri("systemctl --user stop caffeinate.service"))


def test_the_first_screen_goes_with_the_launcher_open_on_it(first_gone: Screens) -> None:
    """The first screen goes with the launcher open on it, and it works on the other."""
    first_gone.launcher_works_on("Virtual-2")


def test_a_screen_coming_back_leaves_the_launcher_open_on_the_other(first_gone: Screens) -> None:
    """A screen coming back leaves the launcher open where it is, on the other.

    A screen plugged in beside the one the launcher is on takes nothing from it, so the launcher
    stays open under Joel's keys. Remade instead, a Super+Space pressed just before a loaded
    shell heard the screen come back opened the launcher and closed it again at once.
    """
    screens = first_gone
    # How often the shell has said it heard two screens, which it says once it's heard them
    # change and done what that asks of the launcher.
    heard = screens.in_niri(
        "journalctl --user -u quickshell -o cat"
        " | grep -c -F 'report[launcher heard 2 screens' || true"
    )
    before = int(screens.machine.succeed(heard))
    screens.super_space_on("Virtual-2")
    try:
        screens.come_back("Virtual-1")
        screens.machine.wait_until_succeeds(
            f"test $({heard}) -gt {before}", timeout=dt.timedelta(seconds=30)
        )
        shown = screens.launcher_shows()
        assert shown["open"], f"a screen coming back closed the launcher: {shown}"
        assert shown["keys"], f"a screen coming back took the launcher's keys: {shown}"
        assert screens.held_on("launcher") == {"Virtual-2"}, screens.held_on("launcher")
    finally:
        screens.machine.send_key("esc")
        screens.launcher_until(lambda shown: not shown["open"], "closed at Escape")


def test_the_first_screen_comes_back_and_the_launcher_works_on_both(first_back: Screens) -> None:
    """The first screen comes back, and the launcher works on both."""
    first_back.launcher_works_on_both()


def test_the_second_screen_goes_with_the_launcher_open_on_it(second_gone: Screens) -> None:
    """The second screen goes with the launcher open on it, and it works on the other."""
    second_gone.launcher_works_on("Virtual-1")


def test_the_second_screen_comes_back_and_the_launcher_works_on_both(
    second_back: Screens,
) -> None:
    """The second screen comes back, and the launcher works on both."""
    second_back.launcher_works_on_both()


def test_both_screens_go_with_the_launcher_open_and_come_back(second_back: Screens) -> None:
    """Both screens go with the launcher open, come back, and it works on both."""
    second_back.gone_while_open(*sorted(second_back.both))
    second_back.come_back(*sorted(second_back.both))
    second_back.launcher_works_on_both()


def test_a_sleep_with_the_launcher_open_and_it_works_after(slept_open: Screens) -> None:
    """A sleep with the launcher open, and after the wake it works on both screens."""
    slept_open.launcher_works_on_both()


def test_after_the_wake_the_launchers_foot_still_tells_the_time(slept_open: Screens) -> None:
    """After the wake, the launcher's foot still tells the time as it passes.

    A clock that stopped in the sleep would still say the minute it stopped at, so the foot
    must follow the minute after the wake. A closed launcher holds its foot as it last showed
    it, drawing nothing nobody sees, so the foot is read open, once the minute has turned.
    """
    woke = slept_open.minute()
    deadline = time.monotonic() + 90
    while slept_open.minute() == woke:
        assert time.monotonic() < deadline, f"the minute never turned from {woke}"
        time.sleep(1)
    slept_open.super_space_on("Virtual-1")
    try:
        slept_open.launcher_until(
            lambda shown: slept_open.minute() in shown["glance"], "told the time after the wake"
        )
    finally:
        slept_open.machine.send_key("esc")
        slept_open.launcher_until(lambda shown: not shown["open"], "closed at Escape")


def test_quickshell_warned_of_nothing_as_the_screens_came_and_went(
    unlocked: Screens,
    slept_open: Screens,  # noqa: ARG001 # Asked for so a run alone takes the sleep too.
) -> None:
    """Quickshell warned of nothing as the launcher's screens came and went.

    A surface made where there's no screen, or a binding that throws or assigns the wrong type
    as a screen comes or goes, only warns.
    """
    warned = unlocked.machine.succeed(
        unlocked.in_niri(
            f"journalctl --user -u quickshell --since @{unlocked.launcher_since} --no-pager -o cat"
            " | grep -a -F WARN || true"
        )
    )
    assert warned.strip() == "", f"Quickshell warned:\n{warned}"
