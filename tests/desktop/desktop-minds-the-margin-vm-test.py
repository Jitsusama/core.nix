"""The desktop test's minds the margin area.

Here the margin shows what stands: keeping awake, cameras, tokens running out, do not
disturb, waiting, the microphone, dictation, what's playing and caps lock.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import json
import re
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import (
    CAPITALS,
    LAUNCHER_OPEN,
    MARGIN,
    NOTICES,
    changed_at_the_top,
    for_you_lit,
    for_you_read,
    margin_points,
    margin_sets,
)

if TYPE_CHECKING:
    from desktop_session import Desktop
    from test_driver.machine import BaseMachine


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


# How far right of a glyph's middle a click lands, in pixels: inside its
# cell, and past where what takes the pointer would end had it stayed half
# a cell to the left, where the row was before it slid.
BESIDE_THE_MIDDLE = 2

# How long the same wait lasts before the VM's margin marks it, in seconds,
# as tests/desktop.nix sets the Lag piece's `lasts`.
LAG_LASTS = 3600

# The scales Joel's screens are seen at, which a design's shots are judged at.
JOELS_SCALES = ("1.5", "2")

# Whether Keep Awake holds something, and whether it holds the lid.
HELD = "systemd-inhibit --list --no-pager | grep -F 'Keep Awake'"
LID_HELD = f"{HELD} | grep -q handle-lid-switch"
# The stand-in sleep leaves this file (tests/desktop.nix).
ASLEEP = "test -e /run/suspended"
# logind sees the lid open and no sleep is coming, queued or running. The sleep unit's state
# is read, since is-active calls a oneshot still running "activating".
SETTLED = (
    "busctl get-property org.freedesktop.login1 /org/freedesktop/login1"
    " org.freedesktop.login1.Manager LidClosed PreparingForSleep | grep -cx 'b false' | grep -qx 2"
    " && ! systemctl list-jobs --no-legend | grep -q suspend"
    " && systemctl show -P ActiveState systemd-suspend.service | grep -qx inactive"
)
# How long logind is given to act on a shut lid it mustn't sleep for.
LOOKING = timedelta(seconds=5)


def now_down_to(desktop: Desktop, name: str) -> None:
    """Open Now with Super+O, and move down to this row from For You.

    Now opens on its first row, For You, as every place opens on
    its first, so the same keys land in the same place each time.
    """
    desktop.machine.send_key("meta_l-o")
    desktop.in_now("For You")
    desktop.down_to(name)


def stays_showing_it_gone(desktop: Desktop, name: str) -> None:
    """Wait for the launcher to stay open showing a row gone, then close it.

    An act that changes how things stand, rather than taking Joel
    anywhere, keeps the launcher, and the row it ended stays in its place
    marked gone, so nothing under it moves.
    """
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and any(
                thing["gone"] and thing["reads"].split(" \u00b7 ")[0] == name
                for thing in shown["things"]
            )
        )
    )
    desktop.close_launcher()


def test_the_launcher_keeps_the_machine_awake_lid_shut_too_then_lets_it_sleep(
    desktop: Desktop,
) -> None:
    """The launcher keeps the machine awake, lid shut too, then lets it sleep."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("keep awake")
    desktop.shoot("launcher-keep-awake")
    # Its one act is its row's, so Ctrl+] goes to where it lives rather
    # than into a list of it (Joel, 2026-11-11), and Return there does it.
    # Keeping it awake changes something, so the launcher stays.
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            shown["held"] != [] and (shown["selected"] or {}).get("name") == "Keep Awake"
        )
    )
    machine.send_key("ret")
    desktop.close_launcher()
    machine.wait_until_succeeds(f"{HELD} | grep -qw idle", timeout=timedelta(seconds=10))
    machine.wait_until_succeeds(LID_HELD, timeout=timedelta(seconds=10))
    # Kept awake, a suspend asked for by the power key or the launcher
    # still goes through.
    machine.fail(f"{HELD} | grep -q sleep")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.shoot("launcher-home-awake")
    desktop.type_in("let it sleep", selecting="Keep Awake")
    # Return from the find lands on it, since it changes how things stand, and a second does it.
    desktop.land()
    machine.send_key("ret")
    machine.wait_until_fails(HELD, timeout=timedelta(seconds=10))
    # It changed something, so the launcher stays until it's closed.
    desktop.close_launcher()


def test_kept_awake_a_lid_shut_on_battery_locks_and_runs_on_and_let_go_it_sleeps(
    desktop: Desktop,
) -> None:
    """Kept awake, a lid shut on battery locks the screen and runs on; let go, it sleeps."""
    machine = desktop.machine
    locked = desktop.in_session("qs ipc call lock isLocked | grep -q true")
    desktop.as_account("systemctl --user start keep-awake.service keep-awake-lid.service")
    machine.wait_until_succeeds(LID_HELD, timeout=timedelta(seconds=10))
    # Joel keeps it awake to run with no power and the lid shut (2026-11-10).
    desktop.power("off")
    try:
        machine.succeed(LID_HELD)
        desktop.lid(shut=True)
        # The lid's hold hears it shut and locks the screen, as the sleep would have.
        machine.wait_until_succeeds(locked, timeout=timedelta(seconds=10))
        # A sleep is logind's to start, so it's given a while to start one, which it mustn't.
        machine.sleep(LOOKING)
        machine.fail(ASLEEP)
        desktop.lid(shut=False)
        desktop.as_account("qs ipc call lock release && systemctl --user stop keep-awake.service")
        machine.wait_until_fails(HELD, timeout=timedelta(seconds=10))
        desktop.lid(shut=True)
        machine.wait_until_succeeds(ASLEEP, timeout=timedelta(minutes=1))
    finally:
        desktop.lid(shut=False)
        # The stand-in sleep ends at once and logind may queue another, so it settles first.
        machine.wait_until_succeeds(SETTLED, timeout=timedelta(minutes=1))
        machine.succeed("rm -f /run/suspended")
        desktop.power("on")
        # The sleep locked the screen first, as it does before every one.
        desktop.as_account("systemctl --user stop keep-awake.service && qs ipc call lock release")
        machine.wait_until_fails(locked, timeout=timedelta(seconds=10))


def test_a_camera_opened_is_said_then_shows_in_the_margin_and_the_launcher_says_by_what(
    desktop: Desktop,
) -> None:
    """A camera opened is said, then shows in the margin, and the launcher says by what."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    # What's true as the shell starts isn't news, so it's waited out.
    desktop.reported("settles arrivals", lambda rest: rest == "", "")
    # Written whole, as the camera's module writes it. A program opened
    # it by itself, so it's said as a notice of the camera's.
    since = desktop.mark()
    with desktop.told("shows card chrome has the camera open"):
        machine.succeed(
            "echo chrome > /run/camera-holders/.camera"
            " && mv /run/camera-holders/.camera /run/camera-holders/camera"
        )
    desktop.shoot("notice-camera")
    # A card or a mark: once the card goes, the glyph shows, on air, so
    # strong, and the card wasn't missed, since the glyph says it.
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.reported("sets margin ", margin_sets(("camera", "onAir", "strong")), since)
    desktop.shoot("margin-camera")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("camera")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "in use by chrome"
    )
    desktop.close_launcher()
    # Let go, it lingers a moment, muted, then goes, and its notice is taken back.
    since = desktop.mark()
    machine.succeed(
        ": > /run/camera-holders/.camera"
        " && mv /run/camera-holders/.camera /run/camera-holders/camera"
    )
    desktop.reported("sets margin ", margin_sets(("camera", "onAir", "muted")), since)
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    machine.wait_until_succeeds(desktop.notice_kept('.app == "Camera" and .withdrawn'))


def test_a_usb_cameras_stream_shows_in_the_margin_too_and_leaves_with_the_camera(
    desktop: Desktop,
) -> None:
    """A USB camera's stream shows in the margin too, and leaves with the camera."""
    machine = desktop.machine
    # A camera the machine doesn't name, written as usb-camera-holders
    # writes one: a file named for the camera, there only while it is.
    usb = "/run/camera-holders/MX Brio 705 for Business"
    with desktop.told("drawn margin"):
        machine.succeed(f"echo zoom > '{usb}.1' && mv '{usb}.1' '{usb}'")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("camera")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "in use by zoom")
    desktop.close_launcher()
    machine.succeed(f"rm '{usb}'")
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))


def test_tokens_running_out_are_one_card_that_counts_down_and_rises_as_they_near(
    desktop: Desktop,
) -> None:
    """Tokens running out are one card, counting down in place and raised as they near."""
    machine = desktop.machine
    desktop.reported("settles arrivals", lambda rest: rest == "", "")
    # The VM runs no spend-watch, so this writes its status as it does,
    # whole and renamed in, its heartbeat the guest's now.
    status = "$XDG_RUNTIME_DIR/spend-watch/status.json"

    def watch_says(title: str, level: str, now: bool) -> None:
        """Write spend-watch's status, its tokens warning with this title, or none."""
        stamp = int(machine.succeed("date +%s").strip())
        quiet = {"value": "today", "badge": "", "hint": "today", "context": ""}
        said: dict[str, object] = {
            "v": 1,
            "heartbeat": stamp,
            "title": "AI spend",
            "open": quiet,
            "quiet": quiet,
            "actions": [],
        }
        if title:
            said["tokens"] = {
                "level": level,
                "now": now,
                "title": title,
                "value": "resets 14:00",
                # As spend-watch words it (tokens.rs), so the shots
                # read as Joel's card will.
                "context": "at $300 an hour · 92% used",
                "quiet": {"title": title, "value": "resets 14:00", "context": ""},
                "actions": [{"kind": "spends", "name": "See What Spends"}],
            }
        desktop.as_account(
            f"mkdir -p $(dirname {status}) && echo {shlex.quote(json.dumps(said))} > {status}.new"
            f" && mv {status}.new {status}"
        )

    since = desktop.mark()
    with desktop.told("shows card Tokens Run Out in About 40 Min"):
        watch_says("Tokens Run Out in About 40 Min", "hour", now=False)
    desktop.shoot("notice-tokens")
    # A minute on, the same card says so in place.
    with desktop.told("rewords Tokens Run Out in About 39 Min"):
        watch_says("Tokens Run Out in About 39 Min", "hour", now=False)
    # A level lower raises the same card again, urgent.
    with desktop.told("raises Tokens Run Out in About 14 Min"):
        watch_says("Tokens Run Out in About 14 Min", "minutes", now=True)
    desktop.shoot("notice-tokens-minutes")
    said = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)}"
    )
    arrived = re.findall(r"\barrives Tokens", said)
    assert len(arrived) == 1, f"the tokens were said as {len(arrived)} cards, not one"
    # Now lists it among what wants Joel, in the same words.
    now_down_to(desktop, "Tokens Run Out in About 14 Min")
    row = desktop.showing()["selected"] or {}
    assert "resets 14:00" in json.dumps(row), row
    assert row.get("note") == "at $300 an hour \u00b7 92% used", row
    desktop.close_launcher()
    # Once it no longer warns, its card is taken back.
    watch_says("", "", now=False)
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=15))
    desktop.as_account(f"rm -f {status}")


def test_keep_awake_shows_in_the_margin_and_now_says_it_and_lets_it_sleep(
    desktop: Desktop,
) -> None:
    """Keep Awake shows in the margin, and Now says it and lets it sleep."""
    machine = desktop.machine
    # Keep Awake was just let go, and the margin leaves as it fades.
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    # A camera's card from before is gone first, so what changes along the
    # top is the glyph alone.
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    # Left on, so muted.
    desktop.held_still()
    empty = desktop.screen_now()
    since = desktop.mark()
    with desktop.told("drawn margin"):
        desktop.as_account("systemctl --user start keep-awake.service")
    desktop.reported("sets margin ", margin_sets(("awake", "leftOn", "muted")), since)
    # Super+O opens Now, on For You, since a mode left on is
    # never what it opens on; on its row, the mark its keys are on wears
    # the accent, nothing grown; closed, it's as it was.
    glyph = changed_at_the_top(empty, desktop.settled_screen("margin-awake"))
    since = desktop.mark()
    now_down_to(desktop, "Keep Awake")
    desktop.reported("sets margin ", margin_points("awake"), since)
    # Settled, as Joel sees it.
    desktop.held_still()
    desktop.shoot("launcher-now")
    since = desktop.mark()
    desktop.close_launcher()
    desktop.reported("sets margin ", margin_points(""), since)
    # Another mark comes and goes, so the row slides aside and back, and
    # what takes the pointer has to follow it there. Both left on, the later
    # in the pieces' order stands further from the corner, whichever came first.
    since = desktop.mark()
    desktop.as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 1")
    desktop.reported("holds margin ", lambda rest: rest.split() == ["muted", "awake"], since)
    desktop.held_still()
    since = desktop.mark()
    desktop.as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 0")
    desktop.reported("holds margin ", lambda rest: rest.split() == ["awake"], since)
    desktop.held_still()
    glyph = (glyph[0] + BESIDE_THE_MIDDLE / desktop.screen_now().width, glyph[1])
    # A click there opens it the same, and Return lets the machine sleep.
    desktop.click(*glyph)
    try:
        desktop.in_now("Keep Awake")
    except AssertionError:
        # Where the click went, beside where the shell's surfaces are.
        print(f"clicked at {glyph} of the screen")
        print(desktop.as_account("niri msg layers; niri msg --json focused-output"))
        raise
    # From the find, Return lands on it, since it changes how things stand, and a second does it.
    desktop.land()
    machine.send_key("ret")
    machine.wait_until_fails(HELD, timeout=timedelta(seconds=10))
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    # Letting it sleep stays in the launcher, which shows it done.
    stays_showing_it_gone(desktop, "Keep Awake")


def test_do_not_disturb_holds_a_notice_shows_it_held_and_ends_from_the_margin(
    desktop: Desktop,
) -> None:
    """Do not disturb holds a notice, shows it held, and ends from the margin."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("do not disturb")
    # Ctrl+] then Return holds them for an hour and closes.
    desktop.follow()
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.wait_for_surface("margin")
    # The shell decides as the notice arrives, so once it says it held
    # one, there's no card to come.
    with desktop.told("held Held back"):
        desktop.as_account("notify-send 'Held back' 'until it ends'")
    machine.fail(desktop.in_session(NOTICES))
    desktop.shoot("margin-quiet")
    # Its row ends it, and Ctrl+] offers the rest, what's held last:
    # An Hour More comes first only when it ends by itself.
    now_down_to(desktop, "Do Not Disturb")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][-1:] == ["Show What's Held"])
    desktop.type_in("show", selecting="Show What's Held")
    machine.send_key("ret")
    # What's held is unread, so it's For You's Desktop, where every way to
    # the notices goes (VKCF3Z, 2026-11-09).
    desktop.launcher_until(
        lambda shown: (
            shown["keys"]
            and shown["held"][-1:] == ["Desktop"]
            and any(row.startswith("Held back") for row in shown["rows"])
        )
    )
    desktop.shoot("launcher-held")
    desktop.close_launcher()
    now_down_to(desktop, "Do Not Disturb")
    # From the find, Return lands on it, since it changes how things stand, and a second does it.
    desktop.land()
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    desktop.as_account("test ! -s $XDG_RUNTIME_DIR/quiet")
    # Ending it stays in the launcher, which shows it done.
    stays_showing_it_gone(desktop, "Do Not Disturb")


def test_now_shows_the_notices_and_a_critical_one_unread_lights_for_yous_tray(
    desktop: Desktop,
) -> None:
    """Now shows the notices, u undoes dd there, and a critical one unread lights the tray."""
    machine = desktop.machine
    # Missed, a notice is what came lately, lighting nothing; open to its sender, d forgets it.
    with desktop.told("shows card Peter"):
        desktop.as_account("notify-send --expire-time=1000 'Peter' 'are you there?'")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=15))
    # Now opens on For You, its first row, however much the notices
    # want Joel. The time and the date are among Now's own rows, each its
    # own answer as it's placed anywhere; the VM has no forecast and no
    # calendar, so the location follows them. The notices aren't a row of
    # Now's: they're For You's, among what came for Joel.
    machine.send_key("meta_l-o")
    desktop.in_now("For You")
    things = desktop.showing()["things"]
    kinds = [thing["is"] for thing in things]
    at = kinds.index("answer:time")
    time, today = things[at : at + 2]
    assert kinds[at + 2] == "on:location", things
    assert not any(thing["reads"].startswith("Notices") for thing in things), things
    # This year's date, which says no year; ISO Date inside says it.
    date = machine.succeed("date '+%A %-d %B'").strip()
    assert time["is"] == "answer:time", time
    assert re.match(r"Time \u00b7 \d\d:\d\d", time["reads"]), time
    assert today["is"] == "answer:today", today
    assert today["reads"].startswith(f"Date \u00b7 {date}"), today
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: "Desktop" in shown["rows"])
    desktop.down_to("Desktop")
    # A place's row: nothing after its name or below it, how many to read
    # at its right, and who sent them in its drawer (VKCF3Z, 2026-11-09).
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("count") == ""
            and shown["selected"].get("note") == ""
            and re.fullmatch(r"\d+\sto read", shown["selected"].get("side", "")) is not None
            and shown["drawer"]["out"]
        )
    )
    # For You's Lately band, Desktop's row on, for the design's look.
    desktop.shoot("launcher-for-you-lately")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: shown["keys"] and shown["held"] == ["now", "For You", "Desktop"]
    )
    says_each_thing_once(desktop)
    desktop.shoot("launcher-from-margin")
    forgets_then_brings_back(desktop)
    desktop.close_launcher()
    # Peter's was read as Desktop showed it, so another, unopened, for Mark Them Read to read.
    with desktop.told("rests Pat"):
        desktop.as_account("notify-send --expire-time=1000 'Pat' 'call me'")
    reads_them_all(desktop)
    # A critical notice put away unread waits in For You's Desktop, whose tray
    # is then the margin's one mark, strong, since it wants Joel; read, it goes.
    since = desktop.mark()
    for_you_lit(desktop, "Paula")
    desktop.reported("sets margin ", margin_sets(("forYou", "wants", "strong")), since)
    desktop.shoot("margin-for-you")
    for_you_read(desktop)


def says_each_thing_once(desktop: Desktop) -> None:
    """Say Peter's sender once, in the drawer's eyebrow, and what History keeps, or "none".

    The cursor row adds nothing to its age, as a Slack message's adds nothing, and History
    says how many it keeps, or "none" as every place holding nothing does (VKCF3Z,
    2026-11-09). Which it says hangs on what the subtests before this one sent.
    """
    desktop.launcher_until(lambda shown: shown["drawer"]["out"] and shown["footer"] == "")
    things = desktop.showing()["things"]
    kept = [
        thing["reads"].split(" \u00b7 ") for thing in things if thing["reads"].startswith("History")
    ]
    assert kept, things
    assert re.fullmatch(r"none|[1-9]\d*\skept", kept[0][1]), things


def reads_them_all(desktop: Desktop) -> None:
    """Mark Them Read reads every notice, and Super+N then says nothing's unread over History.

    With no cards, Super+N goes to For You's Desktop, as every way to the notices does. With
    none unread, the empty line says so where any empty place says it, and History is its one
    row, the cursor on it (VKCF3Z, 2026-11-09: a row Return does nothing on is a fake one).
    """
    desktop.machine.send_key("meta_l-n")
    desktop.launcher_until(lambda shown: shown["held"][-1:] == ["Desktop"])
    desktop.down_to("Mark Them Read")
    # Reading them all can't be taken back, so Return writes :delete and its line asks first.
    # Desktop holds still while it's open, so they leave it once it closes.
    desktop.line_answered("delete")
    desktop.close_launcher()
    desktop.machine.send_key("meta_l-n")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["now", "For You", "Desktop"]
            and shown["nothing"] == "Nothing unread from the desktop."
            and shown["rows"] == ["History"]
            and (shown["selected"] or {}).get("name") == "History"
            # Its side says what it counts (VKCF3Z, 2026-11-09).
            and re.fullmatch(r"\d+ kept", shown["selected"].get("side", "")) is not None
        )
    )
    desktop.shoot("launcher-desktop-none-unread")
    desktop.close_launcher()


def forgets_then_brings_back(desktop: Desktop) -> None:
    """Dd forgets Peter's notice, which offers no Read, and u brings it back.

    The foot names d by what it does, forget, since notify-send's notice has no Read to mark;
    once it's done the foot says how to take it back, and u does, until the launcher closes.
    """
    desktop.normal()
    assert "d forget" in desktop.showing()["selected"]["told"], desktop.showing()["selected"]
    with desktop.told("ended 1"):
        desktop.machine.send_chars("dd")
    desktop.launcher_until(
        lambda shown: shown["operated"].endswith("1 done \u00b7 u brings it back")
    )
    desktop.shoot("launcher-notice-done")
    assert "d forget" not in desktop.showing()["selected"]["told"], desktop.showing()
    with desktop.told("undid 1"):
        desktop.machine.send_chars("u")
    desktop.launcher_until(
        lambda shown: (
            shown["operated"].endswith("1 brought back") and "d forget" in shown["selected"]["told"]
        )
    )


def test_waiting_that_lasts_a_minute_shows_in_the_margin_quietly(desktop: Desktop) -> None:
    """Waiting that lasts a minute shows in the margin, quietly."""
    machine = desktop.machine
    # lag-watch would write its own window over a stand-in's, so it rests.
    desktop.as_account("systemctl --user stop lag-watch.service")
    lagged = "$XDG_RUNTIME_DIR/lag-watch/status.json"

    def lag_says(worst: str, who: str, ago: int) -> None:
        """Write a window as lag-watch does, renamed in, its run begun so long ago."""
        now = int(machine.succeed("date +%s").strip())
        window = {
            "ended": now,
            "trouble": [f"{who} waited on {worst}"] if worst else [],
            "worst": worst,
            "who": who,
            "since": now - ago if worst else None,
        }
        desktop.as_account(
            f"mkdir -p $(dirname {lagged}) && echo '{json.dumps(window)}' > {lagged}.new"
            f" && mv {lagged}.new {lagged}"
        )

    # A moment's wait says nothing.
    since = desktop.mark()
    with desktop.told("heard lag memory"):
        lag_says("memory", "apps", 10)
    said = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)}"
    )
    assert not re.search(r"holds margin [^\n]*\blag\b", said), "a moment's wait showed"
    # As long as the VM's Lag waits for (tests/desktop.nix's hour) it
    # does, left on, so muted. A run that began that long ago is another
    # run, so the last one ends first.
    with desktop.told("heard lag nothing"):
        lag_says("", "", 0)
    lag_says("memory", "apps", LAG_LASTS + 10)
    desktop.reported("sets margin ", margin_sets(("lag", "leftOn", "muted")), since)
    desktop.shoot("margin-lag")
    # Over, it goes.
    lag_says("", "", 0)
    desktop.reported("holds margin", lambda rest: "lag" not in rest.split(), since)
    desktop.as_account("systemctl --user start lag-watch.service")


def test_the_microphone_muted_shows_in_the_margin_and_return_unmutes_it(desktop: Desktop) -> None:
    """The microphone muted shows in the margin, and Return unmutes it."""
    machine = desktop.machine
    try:
        with desktop.told("drawn margin"):
            desktop.as_account("wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 1")
    except AssertionError:
        # Which source Pipewire calls the default, whether it's muted,
        # and what Pipewire holds for it, beside what Quickshell heard.
        print(desktop.as_account("wpctl status; wpctl get-volume @DEFAULT_AUDIO_SOURCE@"))
        print(desktop.as_account("pw-dump stand-in-microphone"))
        raise
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q microphone"), timeout=timedelta(seconds=10)
    )
    desktop.shoot("margin-muted")
    # In Now it's the Muted switch, named for whose and saying yes
    # (docs/design.md: "A switch is one row that keeps its name as it
    # flips"), never an Unmute It act.
    now_down_to(desktop, "Microphone Muted")
    desktop.launcher_until(
        lambda shown: any(
            thing["reads"].startswith("Microphone Muted \u00b7 yes") for thing in shown["things"]
        )
    )
    # A switch changes how things stand, so from the find Return lands on it, and the
    # second flips it.
    desktop.land()
    machine.send_key("ret")
    muted = "wpctl get-volume @DEFAULT_AUDIO_SOURCE@ | grep -q MUTED"
    machine.wait_until_fails(desktop.in_session(muted), timeout=timedelta(seconds=10))
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    # Unmuting stays in the launcher, which shows it done.
    stays_showing_it_gone(desktop, "Microphone Muted")
    # Unmuting shows the meter a moment, which the next pictures
    # would hold for as long as the host takes to reach them.
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q microphone"), timeout=timedelta(seconds=10)
    )


# Dictation shows at the bottom centre, never in the margin, and says what
# it heard, from a stand-in for its service (desktop.nix).


def test_dictation_says_what_its_doing_and_when_it_heard_nothing(desktop: Desktop) -> None:
    """Dictation says what it's doing, and when it heard nothing."""
    machine = desktop.machine
    dictation = "niri msg layers | grep -q dictation"
    # The volume's and the microphone's meters share the bottom centre, and
    # show for a moment as the shell first hears them, so they're let go.
    meters = "niri msg layers | grep -Eq 'Namespace: \"(volume|microphone)\"'"
    machine.wait_until_fails(desktop.in_session(meters), timeout=timedelta(seconds=15))
    since = desktop.mark()
    desktop.as_account("dictation toggle")
    desktop.reported("says dictation listening", lambda rest: rest == "", since)
    desktop.wait_for_surface("dictation")
    # While it listens, its wave takes each peak the microphone hears and its clock reads
    # each second, so it never stands still by design.
    desktop.shoot("dictation-listening", moving="dictation's wave and clock, while it listens")
    # Typing shows once it lasts, and a press meanwhile says it's still
    # writing; the stand-in types for five seconds.
    desktop.as_account("dictation toggle")
    desktop.reported("says dictation typing", lambda rest: rest == "", since)
    desktop.as_account("dictation toggle")
    desktop.reported("says dictation still writing", lambda rest: rest == "", since)
    desktop.shoot("dictation-still-writing")
    desktop.reported("says dictation heard nothing", lambda rest: rest == "", since)
    desktop.shoot("dictation-heard-nothing")
    machine.wait_until_fails(desktop.in_session(dictation), timeout=timedelta(seconds=10))


# The laptop's Copilot key sends Super, Shift and F23 together, which the
# keyboard's layout reads as F23, and niri's bind for it starts
# dictation. A stand-in key sends the same, through the kernel as the
# laptop's does, since QEMU's keyboard has no F23.


def test_the_copilot_key_starts_dictation(desktop: Desktop) -> None:
    """The Copilot key starts dictation."""
    machine = desktop.machine
    dictation = "niri msg layers | grep -q dictation"
    machine.wait_until_fails(desktop.in_session(dictation), timeout=timedelta(seconds=15))
    machine.succeed(f"systemd-run --unit=copilot-key {desktop.values['copilot_key']}")
    machine.wait_for_file("/run/copilot-key", timeout=timedelta(seconds=30))
    # A press before the key is open would be lost, so the test waits until
    # it is. Here niri runs nested in Cage, which reads the machine's input
    # devices and hands their keys on to niri, so it's Cage that opens it.
    node = machine.succeed("cat /run/copilot-key.device").strip()
    # Its process isn't named cage exactly, so it's found by its command.
    held = f"find /proc/$(pgrep -o -f bin/cage)/fd -lname {node} | grep -q ."
    try:
        machine.wait_until_succeeds(held, timeout=30)
    except AssertionError:
        # What udev made of the key says why Cage passed it by.
        seen = machine.execute(f"udevadm info {node}", check_return=False)[1]
        pytest.fail(f"Cage never opened the Copilot key; udev says:\n{seen}")
    since = desktop.mark()
    machine.succeed("echo press > /run/copilot-key")
    try:
        desktop.reported("says dictation listening", lambda rest: rest == "", since)
    finally:
        machine.succeed("systemctl stop copilot-key")
    # It's put back as it was: stopped, it types what it heard, nothing here.
    desktop.as_account("dictation toggle")
    desktop.reported("says dictation heard nothing", lambda rest: rest == "", since)
    machine.wait_until_fails(desktop.in_session(dictation), timeout=timedelta(seconds=10))


# Any player that tells the desktop what it plays shows in the margin
# while it plays, and the media keys' command shows what it is.


def test_whats_playing_shows_in_the_margin_and_a_media_key_shows_what_it_is(
    desktop: Desktop,
) -> None:
    """What's playing shows in the margin, and a media key shows what it is."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    # A track that starts by itself shows its card a moment, which
    # goes before the margin is shot.
    with desktop.told("drawn notifications"), desktop.told("drawn margin"):
        desktop.as_account(
            "systemd-run --user --unit=player mpv --ao=null --no-video --loop"
            f" {desktop.values['track']}"
        )
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"), timeout=timedelta(seconds=30)
    )
    desktop.shoot("margin-playing")
    # The play key pauses it, and the card says what was paused.
    with desktop.told("drawn notifications"):
        desktop.as_account("qs ipc call music toggle")
    desktop.shoot("music-card")
    # Paused, nothing's playing, so the margin goes; the card goes by itself.
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=15))
    desktop.as_account("systemctl --user stop player")


def test_for_yous_tray_is_shot_alone_calm_and_beside_a_timer_and_music(desktop: Desktop) -> None:
    """For You's tray is shot alone, then out and lit beside a running timer and music.

    At Joel's two scales, for its weight and size against the densest marks lit beside it
    (VKCF3Z, 2026-11-09). Only the gallery shoots them.
    """
    if not desktop.values["shots"]:
        return
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))
    for scale in JOELS_SCALES:
        desktop.as_account(f"niri msg output winit scale {scale}")
        for_you_lit(desktop, f"Alone at {scale}")
        desktop.shoot(f"margin-for-you-at-{scale.replace('.', '-')}")
        for_you_read(desktop)
    desktop.as_account("niri msg output winit scale 1")
    with desktop.told("drawn notifications"):
        track = desktop.values["track"]
        desktop.as_account(f"systemd-run --user -u player mpv --ao=null --no-video --loop {track}")
    machine.wait_until_fails(desktop.in_session(NOTICES), timeout=timedelta(seconds=30))
    desktop.answer("timer 25m bread")
    desktop.land()
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: str((shown["selected"] or {}).get("name")).startswith("Bread \u00b7 ")
    )
    desktop.close_launcher()
    try:
        for scale in JOELS_SCALES:
            desktop.as_account(f"niri msg output winit scale {scale}")
            desktop.shoot(f"margin-calm-beside-a-timer-at-{scale.replace('.', '-')}")
            for_you_lit(desktop, f"Beside at {scale}")
            desktop.shoot(f"margin-for-you-beside-a-timer-at-{scale.replace('.', '-')}")
            for_you_read(desktop, "timer", "playing")
    finally:
        desktop.as_account("niri msg output winit scale 1")
        desktop.as_account("systemctl --user stop player timers.service")
        # The timers the daemon keeps go with it, in the account's state.
        desktop.as_account("rm -f ~/.local/state/timers/held.json")
        machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=30))


def test_caps_lock_shows_in_the_margin_while_its_on(desktop: Desktop) -> None:
    """Caps lock shows in the margin while it's on."""
    machine = desktop.machine
    machine.send_key("caps_lock")
    machine.wait_until_succeeds(f"grep -qx 1 {CAPITALS}", timeout=timedelta(seconds=10))
    desktop.wait_for_surface("margin")
    desktop.shoot("margin-capitals")
    machine.send_key("caps_lock")
    machine.wait_until_succeeds(f"grep -qx 0 {CAPITALS}", timeout=timedelta(seconds=10))
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))


def test_caps_lock_still_shows_after_its_watcher_restarts(desktop: Desktop) -> None:
    """Caps lock still shows after its watcher restarts."""
    machine = desktop.machine
    # Quickshell keeps running, so it must still hear the file.
    machine.succeed("systemctl restart keyboard-lights")
    machine.wait_for_unit("keyboard-lights.service")
    machine.send_key("caps_lock")
    machine.wait_until_succeeds(f"grep -qx 1 {CAPITALS}", timeout=timedelta(seconds=10))
    desktop.wait_for_surface("margin")
    machine.send_key("caps_lock")
    machine.wait_until_fails(desktop.in_session(MARGIN), timeout=timedelta(seconds=10))


# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.


def test_quickshell_warned_of_nothing_it_drew_all_along(desktop: Desktop) -> None:
    """Quickshell warned of nothing it drew all along."""
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = desktop.as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"
