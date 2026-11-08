"""The screens test's come-and-go area.

The shell's surfaces stay through the screens going and coming back, a console switch and a
sleep, and on two screens apart, one going and back.

tests/screens.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The machine and each step it's taken
through are screens_session.py's fixtures, which every test asks for by name.
"""

from __future__ import annotations

import datetime as dt
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from screens_session import Apart, Screens

# The fixtures are a plugin's, never imported here, so pytest loads the module once and checks
# its asserts as it does a test's.
pytest_plugins = ["screens_session"]

# Every surface the shell keeps up while a notice, Keep Awake and dictation are on.
EVERY_SURFACE = ("wallpaper", "launcher", "notifications", "margin", "dictation")


def test_a_critical_notice_is_up_before_the_screen_goes(noticed: Screens) -> None:
    """A critical notice is up before the screen goes."""
    noticed.wait_for_surfaces("notifications")


def test_the_margin_shows_a_mark_and_dictation_listens_first(marked: Screens) -> None:
    """The margin shows a mark, and dictation listens, before the screen goes."""
    marked.wait_for_surfaces("margin", "dictation")


def test_desk_facts_says_what_the_screen_is_before_it_goes(booted: Screens) -> None:
    """Desk-facts says what the screen is before it goes."""
    booted.displays_say(named=True)


def test_the_screen_goes_and_niri_lets_it_go(gone: Screens) -> None:
    """The screen goes, and niri lets it go."""
    gone.machine.succeed(gone.in_niri("niri msg --json outputs | grep -qx '{}'"))


def test_desk_facts_no_longer_names_the_screen_once_its_gone(gone: Screens) -> None:
    """Desk-facts no longer names the screen once it's gone."""
    gone.displays_say(named=False)


def test_the_screen_comes_back_and_the_launcher_and_notice_with_it(back: Screens) -> None:
    """The screen comes back, and the launcher and the notice with it."""
    back.wait_for_surfaces("wallpaper", "launcher", "notifications")


def test_the_margins_mark_and_dictations_bar_come_back(back: Screens) -> None:
    """The margin's mark and dictation's bar come back with the screen."""
    back.wait_for_surfaces("margin", "dictation")


def test_desk_facts_names_the_screen_again_once_its_back(back: Screens) -> None:
    """Desk-facts names the screen again once it's back.

    bochs-drm reads the screen's EDID from QEMU at each probe, as a real display's driver does
    at a replug, so the screen comes back whole.
    """
    edid = f"{back.connectors['Virtual-1']}/edid"
    back.machine.log(f"EDID: {back.machine.succeed(f'wc -c < {edid}').strip()} bytes")
    back.displays_say(named=True)


def test_a_console_switch_away_and_back_keeps_them_all(switched: Screens) -> None:
    """A console switch away and back keeps them all."""
    switched.niri_answers("a console switch")
    switched.wait_for_surfaces(*EVERY_SURFACE)


def test_the_machine_sleeps_and_wakes_at_a_key_and_keeps_them_all(slept: Screens) -> None:
    """The machine sleeps and wakes at a key, and keeps them all."""
    said = slept.machine.succeed("journalctl --boot --identifier=systemd-sleep --output=cat")
    assert "System returned from sleep" in said, f"the machine never slept: {said}"
    slept.wait_for_surfaces(*EVERY_SURFACE)


# A surface drawn again says nothing of what it hears. Each of these goes
# by what the shell is told, over the bus, a socket or a unit's state, so
# a connection that died in the sleep and wasn't made again would leave it
# standing.


def test_after_the_wake_a_notice_still_arrives(slept: Screens) -> None:
    """After the wake, a notice still arrives."""
    slept.machine.succeed(slept.in_niri("notify-send 'After the wake' 'Heard over the bus'"))
    slept.machine.wait_until_succeeds(
        slept.in_niri(
            "journalctl --user --unit=quickshell --output=cat"
            " | grep -qF 'report[shows card After the wake]'"
        ),
        timeout=dt.timedelta(seconds=30),
    )


def test_after_the_wake_dictation_still_hears_its_keys(slept: Screens) -> None:
    """After the wake, dictation still hears its keys, and its bar goes when it's done."""
    assert slept.listening(), "dictation stopped listening in the sleep"
    slept.quiet()


def test_after_the_wake_keep_awake_let_go_takes_its_mark_away(slept: Screens) -> None:
    """After the wake, Keep Awake let go takes its mark away."""
    # With nothing else on, the margin goes with its last mark.
    slept.quiet()
    slept.machine.succeed(slept.in_niri("systemctl --user stop keep-awake.service"))
    slept.wait_for_no_surface("margin")


# Joel docks with two screens, and an undock can take either. The margin is
# a surface on each screen; dictation's bar is one, on the screen niri
# gives it, so the screen to take is the one it's on.


def test_on_two_screens_the_margin_is_on_each_and_dictation_on_one(apart: Apart) -> None:
    """On two screens, the margin shows on each and dictation's bar on one."""
    apart.screens.wait_for_held_on("margin", apart.screens.both)
    assert apart.screens.held_on("dictation") == {apart.listening}


def test_one_of_two_screens_goes_and_the_margin_and_bar_stay(one_gone: Apart) -> None:
    """One of two screens goes, and the margin and dictation's bar stay on the other."""
    one_gone.screens.wait_for_held_on("margin", one_gone.left)
    one_gone.screens.wait_for_held_on("dictation", one_gone.left)


def test_the_screen_comes_back_and_the_margin_is_on_both_again(one_back: Apart) -> None:
    """The screen comes back, and the margin is on both again."""
    one_back.screens.wait_for_held_on("margin", one_back.screens.both)
    # Still listening through it all, so the second press ends it. The shell remakes the bar as
    # the screens are put back, gone for under a second, so one look can land in that gap.
    one_back.screens.wait_for_surfaces("dictation")
    one_back.screens.quiet()
