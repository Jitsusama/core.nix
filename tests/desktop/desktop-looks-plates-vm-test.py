"""The desktop test's looks area for the drawer's plates.

Here the launcher is laid out at Joel's two scales, on a screen as large as the laptop's, which the
other areas' screen is too small to hold the card and its drawer at: 1.5, as the HP draws it, and 2,
as the laptop does. Its drawer is a plate whose every line lies on one of the card's
(docs/design.md, the drawer), measured from what the launcher reports of both to a hundredth of a
pixel: a notice's title, eyebrow and words, a time server's and a device's facts, a person's page,
and Now's head and groups, each shot in the gallery for a person to measure too. It's an area of its
own, rather than more of looks, so the looks areas run side by side rather than one after another;
what they share is in looks_session.py. tests/desktop.nix runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. Every test asks for the
desktop, started once a run (desktop_session.py), and leaves it as it found it.
"""

from __future__ import annotations

import json
import shlex
from datetime import timedelta
from functools import partial
from itertools import pairwise
from typing import TYPE_CHECKING, Any

import desktop_session
import pytest

# pytest finds a fixture by its name in the test's module, so the stand-in Google's is
# brought here.
from desktop_session import LAUNCHER_OPEN, google  # noqa: F401

# pytest finds a fixture by its name in the test's module, so the looks areas' desktop and
# scales are brought here.
from looks_session import (  # noqa: F401
    BEAT,
    NEAR,
    PITCH,
    TITLE_LEADING,
    at_each_scale,
    at_scale,
    body_line,
    bottom_set,
    desktop,
    on_the_beat,
    plate_shown,
    room_beside_the_card,
    things_apart,
)

if TYPE_CHECKING:
    from desktop_session import Desktop, Json

# A Display's pitch: how far Now's time sits under its eyebrow, as a title sits a Title's.
DISPLAY_PITCH = 56


def after_the_title(line: float, lines: Json, card: Json) -> bool:
    """Say whether a line is the body's first, on the card's beat under the title's head."""
    body = body_line(lines, card)
    return body is not None and abs(line - body) <= NEAR


def says_what_it_hides(drawer: Json, lines: list[float]) -> bool:
    """Say whether its last line counts the things hidden below and how to go in to them.

    A thing is hidden when its first baseline lies past the foot; a heading isn't one. The
    chord is drawn key by key, each + its own part, as the rows and the foot draw one.
    """
    hidden = sum(
        1 for at, line in enumerate(lines) if line >= drawer["foot"] and drawer["facts"][at] != ""
    )
    if hidden == 0:
        return bool(drawer["moreSays"] == "")
    return bool(
        drawer["moreSays"] == f"{hidden} more \u00b7 Ctrl+W l"
        and drawer.get("moreKeys") == ["Ctrl", "+", "W", "l"]
    )


def facts_laid(desktop: Desktop, title: str) -> Json:
    """Wait for the plate titled so to lay each fact under the last; give that report.

    One read while its rows are still being laid holds them all on one line, and the title
    breaks again as its face takes a new scale, laying the facts again after it, so what
    the launcher said last may be mid-way: the plate is held to the report that passed.
    """
    laid: list[Json] = []

    def placed(shown: Json) -> bool:
        """Say the plate's facts each lie under the last, keeping the report that did."""
        facts = shown["drawer"]["lines"]["facts"]
        if (
            shown["drawer"]["out"] is True
            and shown["drawer"]["title"] == title
            and len(facts) > 1
            and all(above < below for above, below in pairwise(facts))
        ):
            laid.append(shown)
        return bool(laid)

    desktop.launcher_until(placed)
    return laid[-1]


def test_a_plates_title_eyebrow_and_words_lie_on_the_cards_lines(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A notice's plate has its eyebrow on the field's baseline and its words on the beat.

    Its facts, how it came and left and its urgency, stand first under the title, as every
    plate's do (Drawer.qml), and its words an empty line under them.
    """
    desktop.as_account(
        "notify-send 'Nightly build finished'"
        " 'All 412 checks passed on the laptop, in 38 minutes. The slowest was the desktop.'"
    )
    machine = desktop.machine
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"),
        timeout=timedelta(seconds=60),
    )
    for scale in at_each_scale:
        at_scale(desktop, scale)
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("nightly build")
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"] is True
                and shown["drawer"]["title"] == "Nightly build finished"
            )
        )
        drawer = plate_shown(desktop, scale)
        lines, card = drawer["lines"], drawer["cardLines"]
        assert lines["eyebrow"] is not None, f"at {scale} the notice's plate has no eyebrow"
        # Its facts start on the card's beat, two Title pitches or more under the title, on
        # the pitch, and its words follow an empty line under the last, on the beat too.
        facts, wrapped = lines["facts"], drawer["factsWrapped"] > 0
        assert facts, f"at {scale} the notice's plate tells no facts: {drawer}"
        assert after_the_title(facts[0], lines, card), drawer
        assert things_apart(facts, wrapped=wrapped), f"at {scale} a fact is off the pitch: {drawer}"
        assert on_the_beat(lines["words"], float(card["rows"][0])), drawer
        under = lines["words"] - facts[-1]
        apart = under >= 2 * PITCH - NEAR if wrapped else abs(under - 2 * PITCH) <= NEAR
        assert apart, f"at {scale} the notice's words aren't a line under its facts: {drawer}"
        # From the program it came through, the desktop's own being what For You's path
        # names, and a notice, filing no kind; never a time, which a fact says (VKCF3Z).
        assert drawer["eyebrow"] == "from notify-send \u00b7 notice", drawer
        desktop.shoot(f"launcher-plate-at-{scale.replace('.', '-')}")
        desktop.close_launcher()


def test_a_plates_facts_lie_on_the_rows_lines_and_halfway(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A time server's plate, and a device's, has its facts from the beat under the title."""
    # chrony keeps the clock by the VM's own time server (tests/desktop.nix), whose plate
    # holds every figure chronyd has of it.
    desktop.machine.wait_until_succeeds(
        "chronyc -c -N sources | grep -q '^^,[*],time.harbour.test,'",
        timeout=timedelta(seconds=90),
    )
    for scale in at_each_scale:
        at_scale(desktop, scale)
        # Ctrl+] goes into what keeps the time, as Return does.
        desktop.answer("time")
        desktop.follow()
        desktop.launcher_until(lambda shown: "Servers" in shown["rows"])
        desktop.down_to("Servers")
        desktop.follow()
        desktop.launcher_until(lambda shown: "time.harbour.test" in shown["rows"])
        desktop.down_to("time.harbour.test")
        drawer = plate_shown(desktop, scale, facts_laid(desktop, "time.harbour.test"))
        # The title, at Title, may take two lines of the plate's width, its leading apart;
        # the facts start on the card's beat under it, on the pitch.
        facts = drawer["lines"]["facts"]
        assert after_the_title(facts[0], drawer["lines"], drawer["cardLines"]), drawer
        assert things_apart(facts, wrapped=drawer["factsWrapped"] > 0), (
            f"at {scale} a fact is off the pitch: {drawer}"
        )
        # Its name is the title, so it isn't said again under it; every other name is whole;
        # and what doesn't fit above the foot's inset waits for the drawer's keys.
        assert "Name" not in drawer["drawnFacts"], drawer
        assert drawer["labelsWhole"], f"at {scale} a fact's name is broken: {drawer}"
        # Each fact is its name and its value on one line, or its name with its value under
        # it, so a value too long beside its name, as an address or a reach, never wraps
        # there or breaks a word.
        assert drawer["besideOnOneLine"], f"at {scale} a value wraps beside its name: {drawer}"
        desktop.launcher_until(lambda shown: any(v is not None for v in shown["drawer"]["values"]))
        drawer = desktop.showing()["drawer"]
        assert drawer["valuesWhole"], f"at {scale} a fact's value breaks a word: {drawer}"
        # One that must stand under its name, as an address, lays every one under: a table or
        # a list, never a mix (VKCF3Z, 2026-11-07).
        assert drawer["laidOneWay"], f"at {scale} its facts mix beside and under: {drawer}"
        assert drawer["more"] or facts[-1] < drawer["foot"], (
            f"at {scale} a fact past the foot can't be scrolled to: {drawer}"
        )
        assert bottom_set(drawer, facts), (
            f"at {scale} the last line isn't the inset above the plate's foot: {drawer}"
        )
        # What it hides below, its last line counts, as Vim marks a line cut off.
        assert says_what_it_hides(drawer, facts), (
            f"at {scale} the last line doesn't count what's hidden: {drawer}"
        )
        desktop.shoot(f"launcher-plate-facts-at-{scale.replace('.', '-')}")
        desktop.close_launcher()
        # The zone's plate, under its eyebrow: a value that won't sit beside its name on one
        # line, as "none before 2046", takes the plate's width under it instead, so no fact
        # stands taller than two lines and none reads a word a line; where one takes two, each
        # stands a beat further from the one before.
        desktop.answer("time")
        desktop.follow()
        desktop.launcher_until(lambda shown: "Zone" in shown["rows"])
        desktop.down_to("Zone")
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"] is True
                and shown["drawer"]["title"] != "time.harbour.test"
                and len(shown["drawer"]["lines"]["facts"]) > 1
            )
        )
        drawer = plate_shown(desktop, scale)
        facts = drawer["lines"]["facts"]
        assert drawer["eyebrow"] == "Time Zone", f"at {scale} the zone's plate: {drawer}"
        assert all(
            later - earlier <= 2 * PITCH + BEAT + NEAR for earlier, later in pairwise(facts)
        ), f"at {scale} a value stands a word a line beside its name: {drawer}"
        assert things_apart(facts, wrapped=drawer["factsWrapped"] > 0), (
            f"at {scale} the zone's facts are off the pitch: {drawer}"
        )
        assert drawer["besideOnOneLine"], f"at {scale} a value wraps beside its name: {drawer}"
        desktop.shoot(f"launcher-plate-zone-at-{scale.replace('.', '-')}")
        desktop.close_launcher()
        warmth_shown(desktop, scale)
        clock_plate_shown(desktop, scale)


@pytest.mark.usefixtures("google")
def test_a_persons_page_names_what_its_rows_hold_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A person's Unread and Recent say nothing after their names, how many at their right.

    Ada wrote the stand-in Google's Lunch on Friday, so her page holds it in Unread, shot at
    both of Joel's scales for a person to judge.
    """
    for scale in at_each_scale:
        at_scale(desktop, scale)
        desktop.answer("ada lovelace")
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("is") == "person:ada@example.com"
        )
        desktop.step_in()
        desktop.launcher_until(lambda shown: "Unread" in shown["rows"])
        desktop.down_to("Unread")
        unread = desktop.showing()["selected"]
        # What it holds is named in its drawer, a place's, as on the nested niri's Agents'
        # Recent (VKCF3Z, dafc2199).
        assert (unread["count"], unread["side"]) == ("", "1"), unread
        desktop.launcher_until(
            lambda shown: (
                (shown["drawer"]["title"], shown["drawer"]["eyebrow"], shown["drawer"]["body"])
                == ("Unread", "place", "Lunch on Friday")
            )
        )
        desktop.shoot(f"launcher-person-at-{scale.replace('.', '-')}")
        desktop.close_launcher()


def warmth_shown(desktop: Desktop, scale: str) -> None:
    """See the screen's Warmth say what holds it by night and by day, inside and on its plate."""
    note = "/run/user/1000/backlight-warmth"
    now = int(desktop.machine.succeed("date +%s").strip()) * 1000
    hour = 3_600_000
    # As the backlight daemon says it, whole by a rename: a night under way, then the same
    # night still to come, as at noon.
    night = {
        "kelvin": 4000,
        "followsTheRoom": True,
        "seesColour": True,
        "roomWhite": 3200,
        "nightLight": True,
        "nightWhite": 4000,
        "sunset": now - hour,
        "sunrise": now + 10 * hour,
    }
    day = night | {"kelvin": 6500, "sunset": now + 5 * hour, "sunrise": now + 19 * hour}

    def warmth_plated(shown: Json, kelvin: str) -> bool:
        """Say whether the Warmth is selected at this kelvin, its plate out."""
        return bool(
            (shown["selected"] or {}).get("side") == kelvin
            and shown["drawer"]["out"] is True
            and shown["drawer"]["title"] == "Warmth"
        )

    def said(warmth: dict[str, Any]) -> None:
        """Say how the warmth stands as the backlight daemon does, whole, by a rename."""
        text = shlex.quote(json.dumps(warmth))
        desktop.as_account(f"printf %s {text} > {note}.writing && mv {note}.writing {note}")

    named = scale.replace(".", "-")
    try:
        for when, warmth, kelvin in (("night", night, "4,000 K"), ("day", day, "6,500 K")):
            said(warmth)
            desktop.as_account("qs ipc call launcher collection displays")
            desktop.launcher_ready()
            desktop.follow()
            desktop.launcher_until(lambda shown: "Warmth" in shown["rows"])
            desktop.down_to("Warmth")
            desktop.launcher_until(partial(warmth_plated, kelvin=kelvin))
            # The plate is held to the lines, as every plate is; its facts' order is
            # answers.qml's to hold.
            drawer = plate_shown(desktop, scale)
            assert drawer["eyebrowNames"] is True, drawer
            desktop.shoot(f"launcher-warmth-{when}-at-{named}")
            desktop.follow()
            desktop.launcher_until(
                lambda shown: shown["rows"][:2] == ["Night Light", "Follow the Room"]
            )
            desktop.shoot(f"launcher-warmth-{when}-inside-at-{named}")
            desktop.close_launcher()
    finally:
        desktop.as_account(f"echo '{{}}' > {note}")


def clock_plate_shown(desktop: Desktop, scale: str) -> None:
    """See the clock's plate: via the server it's kept by, titled Clock, its state left out.

    Its eyebrow is the server it's kept via, set as written, and its title its name; its state
    stays its row's side (VKCF3Z, 2026-11-07).
    """
    desktop.answer("time")
    desktop.follow()
    desktop.launcher_until(lambda shown: "Clock" in shown["rows"])
    desktop.down_to("Clock")
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and shown["drawer"]["title"] == "Clock"
            and len(shown["drawer"]["lines"]["facts"]) > 1
        )
    )
    drawer = plate_shown(desktop, scale)
    assert drawer["eyebrow"] == "via time.harbour.test", f"at {scale} the clock: {drawer}"
    assert drawer["eyebrowNames"] is True, f"at {scale} the server is re-cased: {drawer}"
    assert drawer["laidOneWay"], f"at {scale} its facts mix beside and under: {drawer}"
    desktop.shoot(f"launcher-plate-clock-at-{scale.replace('.', '-')}")
    desktop.close_launcher()


def test_nows_plate_sets_its_head_and_groups_as_every_plate(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Now's plate sets its head as one unit and its groups as a body under it, apart."""
    for scale in at_each_scale:
        at_scale(desktop, scale)
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"] is True
                and shown["drawer"]["moment"] is not None
                and shown["drawer"]["lines"]["time"] is not None
            )
        )
        shown = desktop.showing()
        drawer = shown["drawer"]
        room = room_beside_the_card(desktop)
        assert drawer["width"] == room, f"at {scale} the plate is {drawer['width']} wide: {drawer}"
        lines, card, moment = drawer["lines"], drawer["cardLines"], drawer["moment"]
        # Its head is one unit, as every plate's is: the eyebrow on the field's baseline, the
        # time a Display pitch under it with its seconds on its baseline; what waits is home's
        # row's to say, never the plate's (VKCF3Z, 25c253d3).
        assert abs(lines["eyebrow"] - card["fieldBase"]) <= NEAR, (
            f"at {scale} Now's eyebrow is off the field's baseline: {drawer}"
        )
        assert abs(lines["time"] - lines["eyebrow"] - DISPLAY_PITCH) <= NEAR, (
            f"at {scale} the time isn't a Display pitch under the eyebrow: {drawer}"
        )
        assert abs(lines["seconds"] - lines["time"]) <= NEAR, drawer
        assert "say" not in moment, f"at {scale} Now's plate says what waits: {drawer}"
        head = lines["time"]
        # Its groups are a body under the head: the first on the card's first beat two Title
        # pitches or more under it, and each after two pitches from the last line before.
        groups = lines["groups"]
        assert groups, f"at {scale} Now's plate shows no facts: {drawer}"
        first = float(card["rows"][0])
        body = first + -(-(head + 2 * TITLE_LEADING - first - NEAR) // BEAT) * BEAT
        assert abs(groups[0][0] - body) <= NEAR, (
            f"at {scale} Now's facts don't start on the beat under its head: {drawer}"
        )
        for earlier, later in pairwise(groups):
            assert abs(later[0] - earlier[-1] - 2 * PITCH) <= NEAR, (
                f"at {scale} Now's groups aren't two pitches apart: {drawer}"
            )
        for group in groups:
            # A group whose facts all take a line has them on the pitch; one where any wraps
            # has each a beat further from the one before, so either way on the card's beat.
            assert things_apart(group, wrapped=False) or things_apart(group, wrapped=True), (
                f"at {scale} a fact is off the pitch: {drawer}"
            )
        # The status line's last ink ends on the inset, as its first starts, Return's held
        # room to the left of its word (Joel, 2026-11-05: "a right padding for text most of
        # the time which looks fucking janky"; VKCF3Z's rule).
        foot = desktop_session.laid_out(desktop)["foot"]
        ends = max(word["starts"] + word["inked"] for word in foot)
        assert abs(ends - shown["footEnd"]) <= 1, (
            f"at {scale} the status line ends at {ends}, not the inset at {shown['footEnd']}: "
            f"{foot}"
        )
        desktop.shoot(f"launcher-now-plate-at-{scale.replace('.', '-')}")
        desktop.close_launcher()
        location_plate_shown(desktop, scale)


# Location's plate as each kind of source leaves it: Wi-Fi, which gives no altitude, speed or
# heading, and GPS, which gives all three after its coordinates (VKCF3Z, 2026-11-09). GPS's
# fix is as wide as Wi-Fi's, so the clock follows neither and the tests after keep their zone.
LOCATION_PLATES: tuple[tuple[str, Json, str, list[str]], ...] = (
    ("", {"source": "wifi"}, "via Wi-Fi · location", []),
    (
        "gps-",
        {"source": "gps", "altitude": 212.4, "speed": 4.26, "heading": 274.6},
        "via GPS · location",
        ["Altitude", "Speed", "Heading"],
    ),
)


def plate_out(shown: Json, eyebrow: str) -> bool:
    """Say whether Location's plate is out, titled by its town, with this eyebrow."""
    drawer = shown["drawer"]
    return bool(
        drawer["out"] is True and drawer["title"] == "Kelpwater" and drawer["eyebrow"] == eyebrow
    )


def location_plate_shown(desktop: Desktop, scale: str) -> None:
    """See the Location's plate at a scale for each source: what found it, its town, its figures."""
    when = int(desktop.machine.succeed("date +%s").strip()) - 720
    for shot, found, eyebrow, measured in LOCATION_PLATES:
        here = {
            "v": 2,
            "latitude": 12.3457,
            "longitude": -98.7654,
            "accuracy": 26000,
            "when": when,
            "place": "Kelpwater, Westmark",
            "zone": "America/Toronto",
            "country": "Canada",
        } | found
        desktop.found_at(here)
        desktop.as_account("qs ipc call launcher collection on")
        desktop.launcher_ready()
        desktop.down_to("Location")
        desktop.launcher_until(partial(plate_out, eyebrow=eyebrow))
        drawer = plate_shown(desktop, scale)
        facts = ["Located", "Accuracy", "Coordinates", *measured, "Country", "Zone"]
        assert drawer["facts"] == facts, f"at {scale} the plate: {drawer}"
        desktop.launcher_until(lambda shown: any(v is not None for v in shown["drawer"]["values"]))
        drawer = desktop.showing()["drawer"]
        assert drawer["valuesWhole"], f"at {scale} a fact's value breaks a word: {drawer}"
        desktop.shoot(f"launcher-location-{shot}plate-at-{scale.replace('.', '-')}")
        desktop.close_launcher()
    # Nowhere again, for the tests after.
    desktop.found_at(None)
