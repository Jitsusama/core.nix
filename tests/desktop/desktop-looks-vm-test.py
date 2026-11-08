"""The desktop test's looks area.

Here the launcher is laid out at Joel's two scales, on a screen as large as the laptop's, which
the other areas' screen is too small to hold the card and its drawer at: 1.5, as the HP draws it,
and 2, as the laptop does. Its drawer is a plate whose every line lies on one of the card's
(docs/design.md, the drawer), measured from what the launcher reports of both to a hundredth of a
pixel, and shot in the gallery for a person to measure too; the file picker's plate is
held to the same lines for each kind of file it previews. An ErgoDox unplugged goes from the
keyboard's layout and the launcher, and comes back plugged in. Bluetooth's Nearby hears a
device of BlueZ's emulator while it's open, and only then. tests/desktop.nix runs this
through pytest inside the driver (tests/lib/vm_pytest.py), and its opening comment says what the
test proves. Every test asks for the desktop, started once a run (desktop_session.py), and leaves
it as it found it.
"""

from __future__ import annotations

import base64
import io
import json
import shlex
import tarfile
import time
from datetime import timedelta
from functools import partial
from itertools import pairwise
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest

# pytest finds a fixture by its name in the test's module, so the stand-in Google's is
# brought here.
from desktop_session import LAUNCHER_OPEN, google  # noqa: F401
from PIL import Image, ImageChops, ImageDraw, ImageStat

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from desktop_session import Desktop, Json
    from PIL import Image
    from test_driver.machine import BaseMachine

# Joel's scales.
JOELS_SCALES = ("1.5", "2")
# A folder kitty has visited, so kitty found holds something under it.
VISITED = "~/src/tidepool"
# The drawer takes what the screen leaves beside the centred card, less the gap between them
# and the margin at the screen's edge, up to its most (Launcher.qml's drawerRoom): the frames
# rung and the screen rung. On the laptop's 2880 that's 520 at 1.5 and 300 at 2; niri gives
# the VM's screen a few logical pixels less, so it's worked out from the screen niri says it
# has.
CARD = 680
GAP = 32
MARGIN = 48
MOST = 520
# The drawer's line pitch, Figure's, two of the card's beats, and how near a line is to where
# it should be: a hundredth, and the half a hundredth each of two lengths the launcher reports
# rounded may take.
PITCH = 24
BEAT = 12
NEAR = 0.02
# A Title's pitch: how far the title sits under the eyebrow, and its lines apart when its name
# wraps, tighter than two of the plate's, so the name reads as one heading. One line's pitch,
# since a Title stands apart by weight rather than size (VKCF3Z, 2026-11-06).
TITLE_LEADING = 24
# A Display's pitch: how far Now's time sits under its eyebrow, as a title sits a Title's.
DISPLAY_PITCH = 56
# How far the text keeps from the plate's frame on every side, its foot included: the inset rung,
# from the frame's inside edge, its hairline border left out, to the ink.
INSET = 24
BORDER = 1
# A row's words stand a step from its name (Theme.space.m), in the theme's serif, and on its
# baseline to half a device pixel.
WORDS_GAP = 12
HALF_A_PIXEL = 0.5
# How long QEMU holds a key down, and how often and how long a sequence that waits for its
# second key is pressed before it counts as never doing what it does.
HELD_MS = 20
TRIES = 3
TRIED = timedelta(seconds=10)
# y's flash lasts Joel's 200 ms (language.toml's yank-flash), so the screen is taken this many
# times at once after it, and a picture is the flash where it differs, by this many pixels
# changed by more than this much light, from both the choice before it and the page after.
# Enough pictures after the screen first changes, one after the other, to span the flash's
# 200 ms.
FLASH_SHOTS = 5
FLASHED = 200
CHANGED = 40
NEVER_FLASHED = "no picture caught y's flash"


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture
def at_each_scale(desktop: Desktop) -> Iterator[list[str]]:
    """Give Joel's scales to set in turn, and put the screen back at 1 afterwards."""
    yield list(JOELS_SCALES)
    # A test that failed with the launcher open leaves it closed for the next.
    if desktop.machine.execute(desktop.in_session(LAUNCHER_OPEN))[0] == 0:
        desktop.close_launcher()
    desktop.as_account("niri msg output winit scale 1")


def at_scale(desktop: Desktop, scale: str) -> None:
    """Set the screen's scale, with the launcher closed, so it opens laid out for it."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account(f"niri msg output winit scale {scale}")


def body_line(lines: Json, card: Json) -> float | None:
    """Say where the plate's body should start, or None where the title's lines are off.

    The title's lines sit its leading apart, and the body starts on the first of the card's
    beats, counted from its first row's baseline, two Title pitches or more under the title's
    last line, so the head reads apart from what follows and the rest lies on the card's beats.
    """
    title, last = float(lines["title"]), float(lines["titleLast"])
    later = round((last - title) / TITLE_LEADING)
    if later < 0 or abs(last - title - later * TITLE_LEADING) > NEAR:
        return None
    first = float(card["rows"][0])
    beats = -(-(last + 2 * TITLE_LEADING - first - NEAR) // BEAT)
    return first + beats * BEAT


def after_the_title(line: float, lines: Json, card: Json) -> bool:
    """Say whether a line is the body's first, on the card's beat under the title's head."""
    body = body_line(lines, card)
    return body is not None and abs(line - body) <= NEAR


def on_the_pitch(line: float, from_line: float) -> bool:
    """Say whether a line lies a whole number of the plate's lines below another."""
    steps = round((line - from_line) / PITCH)
    return steps >= 0 and abs(line - from_line - steps * PITCH) <= NEAR


def on_the_beat(line: float, from_line: float) -> bool:
    """Say whether a line lies a whole number of the card's beats below another."""
    steps = round((line - from_line) / BEAT)
    return steps >= 0 and abs(line - from_line - steps * BEAT) <= NEAR


def things_apart(firsts: list[float], wrapped: bool) -> bool:
    """Say whether a list's things lie apart as they should, given each one's first line.

    Its lines are on the pitch, and where any thing wraps, each thing stands a beat further
    from the one before, so a wrapped line sits nearer its own thing than the next.
    """
    apart = BEAT if wrapped else 0
    return all(
        later - earlier > apart and on_the_pitch(later - apart, earlier)
        for earlier, later in pairwise(firsts)
    )


def bottom_set(drawer: Json, lines: list[float]) -> bool:
    """Say whether the last of these lines shown keeps the inset above the frame's bottom.

    Its baseline stands at least the inset above the frame's inside edge, and where more waits
    below, the next thing, with its heading if it opens a group, would not have fit whole there.
    These are each thing's first baselines, so a thing's last stands a step above the next's
    first: a pitch, a beat more where any thing in the list wraps, and two pitches before a
    group's heading, a fact with no name. Past the last thing left, the test can't tell. Where
    it hides any, the line saying so takes the lowest beat the inset above the frame's inside
    edge, and the things end a step above it.
    """
    shown = [line for line in lines if line < drawer["foot"]]
    if not shown:
        return False
    headings = {at for at, name in enumerate(drawer["facts"]) if name == ""}
    step = PITCH + BEAT if drawer["factsWrapped"] else PITCH
    floor = drawer["height"] - BORDER - INSET
    if drawer["moreSays"]:
        if not floor - BEAT < drawer["moreBase"] <= floor + NEAR:
            return False
        floor = drawer["moreBase"] - step
    if shown[-1] > floor + NEAR:
        return False
    thing = len(shown) + (len(shown) in headings)
    if not drawer["more"] or thing + 1 >= len(lines):
        return True
    last = lines[thing + 1] - (2 * PITCH if thing + 1 in headings else step)
    return bool(last > floor + NEAR)


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


def room_beside_the_card(desktop: Desktop) -> int:
    """Say how wide the drawer should be on the screen as niri lays it out now."""
    output = next(iter(json.loads(desktop.as_account("niri msg --json outputs")).values()))
    width = int(output["logical"]["width"])
    return round(min(MOST, (width - CARD) / 2 - GAP - MARGIN))


def plate_shown(desktop: Desktop, scale: str) -> Json:
    """Say what the drawer shows, once it's out, holding it to the card's lines and its width."""
    drawer = desktop.showing()["drawer"]
    room = room_beside_the_card(desktop)
    assert drawer["width"] == room, f"at {scale} the plate is {drawer['width']} wide, not {room}"
    assert drawer["height"] == drawer["cardHeight"], f"the plate isn't the card's height: {drawer}"
    lines, card = drawer["lines"], drawer["cardLines"]
    # The head is one unit: the eyebrow on the field's baseline, the title a Title pitch under.
    if lines["eyebrow"] is not None:
        assert abs(lines["eyebrow"] - card["fieldBase"]) <= NEAR, (
            f"at {scale} the eyebrow is off the field's baseline: {drawer}"
        )
    assert abs(lines["title"] - card["fieldBase"] - TITLE_LEADING) <= NEAR, (
        f"at {scale} the title isn't a Title pitch under the eyebrow: {drawer}"
    )
    return cast("Json", drawer)


def words_on_their_names(card: Json, scale: str, serif: str) -> int:
    """Hold each drawn row's words to its name's baseline, the gap and the serif; say how many."""
    drawn = [
        (name, words)
        for name, words in zip(card["rows"], card["words"], strict=True)
        if name is not None and words is not None
    ]
    for name, words in drawn:
        off = abs(words["baseline"] - name) * float(scale)
        assert off <= HALF_A_PIXEL, (
            f"at {scale} a row's words are {off:.2f} device pixels off its name's baseline: {card}"
        )
        assert abs(words["gap"] - WORDS_GAP) <= NEAR, (
            f"at {scale} a row's words stand {words['gap']} from its name: {card}"
        )
        assert words["face"] == serif, f"at {scale} a row's words are in {words['face']}"
    return len(drawn)


def test_a_rows_words_are_in_the_serif_on_its_names_baseline(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Home's lists and a found row's note are in the serif, on the name's baseline, a step on."""
    # The serif is the theme's, read as the theme exports it, so a theme with another face is
    # held to that face.
    theme = json.loads(desktop.as_account("cat ~/.config/jitsusama/theme.json"))
    serif = theme["font"]["serif"]["family"]
    for scale in at_each_scale:
        at_scale(desktop, scale)
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: shown["drawer"]["out"] is True)
        card = desktop.showing()["drawer"]["cardLines"]
        assert words_on_their_names(card, scale, serif) > 1, (
            f"at {scale} home's rows say nothing: {card}"
        )
        desktop.shoot(f"launcher-row-words-at-{scale.replace('.', '-')}")
        # No key is named in a mode where it does nothing: on the find the arrows move the
        # caret, so home's first row, a place, says Return alone, and in normal its arrow too.
        foot = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
        arrowed = [word["text"] for word in foot if set(word["text"]) & {"\u2192", "\u2190"}]
        assert not arrowed, f"at {scale} the find's foot names an arrow: {arrowed}"
        # Normal's foot names its keys by letter, as Vim's do, so no arrow either, and always
        # the command line's and the key list's.
        desktop.normal()
        foot = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
        arrowed = [word["text"] for word in foot if set(word["text"]) & {"\u2192", "\u2190"}]
        assert not arrowed, f"at {scale} normal's foot names an arrow: {arrowed}"
        told = desktop.showing()["selected"]["told"]
        assert told[-2:] == [": commands", "Space ? keys"], f"at {scale} normal's foot: {told}"
        # Home's head in normal: no / and no question, the column kept (Joel, 2026-11-06).
        desktop.shoot(f"launcher-home-normal-at-{scale.replace('.', '-')}")
        finds_kitty(desktop, scale)
        card = desktop.showing()["drawer"]["cardLines"]
        assert words_on_their_names(card, scale, serif) > 0, (
            f"at {scale} kitty's row says nothing: {card}"
        )
        desktop.shoot(f"launcher-found-words-at-{scale.replace('.', '-')}")
        # The find is a line, no mode of Vim's, so the foot names none; a prompt's insert,
        # which does, is seen whole in finds-sessions' Talk.
        foot = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
        named = [word for word in foot if word["text"] in ("insert", "find", "normal")]
        assert not named, f"at {scale} the find's foot names a mode: {named}"
        teaches_in_the_foot(desktop, scale)
        # In normal the field draws no caret and the foot names no mode, the
        # row's words where the find left them.
        desktop.normal()
        card = desktop.showing()["drawer"]["cardLines"]
        assert words_on_their_names(card, scale, serif) > 0, (
            f"at {scale} kitty's row says nothing in normal: {card}"
        )
        # Normal numbers the rows in its gutter, the cursor row's absolute, and says the find's
        # count after it, whole.
        shown = desktop.showing()
        assert shown["gutter"]["shown"], f"at {scale} normal's gutter: {shown['gutter']}"
        assert shown["gutter"]["cursor"] == str(shown["current"] + 1), shown["gutter"]
        field = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["field"]
        narrowing = [word for word in field if word["text"].startswith("\u00b7 ")]
        assert narrowing, f"at {scale} the narrowed find says no count: {field}"
        assert not narrowing[0]["cut"], f"at {scale} the narrowed count is cut: {narrowing}"
        desktop.shoot(f"launcher-found-normal-at-{scale.replace('.', '-')}")
        says_until_the_next_key(desktop, scale)
        # A count typed stands where insert's word was until its move, as showcmd shows it.
        stands_in_the_mode_words_room(desktop, "12", f"launcher-count-at-{scale}")
        shows_the_command_line(desktop, scale)
        lands_on_a_setting(desktop, scale)
        lines_what_ends(desktop, scale)
        lists_every_key(desktop, scale)
        leaves_kitty(desktop)
        washes_what_it_finds(desktop, scale)
        numbers_programs(desktop, scale)
        counts_before_its_chord(desktop, scale)
        says_on_its_row(desktop, scale)


def says_on_its_row(desktop: Desktop, scale: str) -> None:
    """Shoot an answer whose cursor row says how it stands, at its far end, in normal.

    What the selection says is a fact about the row, so it's said on the row, before its value,
    rather than in the head (VKCF3Z, 2026-11-06). Location says it here, since GeoClue never
    finds a guest.
    """
    desktop.answer("location")
    desktop.normal()
    desktop.launcher_until(lambda shown: shown["footer"] != "")
    desktop.shoot(f"launcher-row-says-at-{scale.replace('.', '-')}")
    desktop.close_launcher()


def numbers_programs(desktop: Desktop, scale: str) -> None:
    """Step into Programs, and shoot its normal and a find in it, the gutter numbering both.

    Normal counts from the cursor and the find numbers each row as it is, in one room, as wide
    as Programs' own rows need (VKCF3Z, 2026-11-05). Normal's keys give way rather than run
    over the glance.
    """
    at = scale.replace(".", "-")
    desktop.answer("programs")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Programs")
    desktop.step_in()
    desktop.launcher_until(lambda shown: len(shown["held"]) == 1 and shown["gutter"]["shown"])
    gutter = desktop.showing()["gutter"]
    assert gutter["relative"], f"at {scale} normal in Programs: {gutter}"
    overlaps = desktop_session.foot_overlaps(desktop)
    assert not overlaps, f"at {scale} normal in Programs: {overlaps}"
    desktop.shoot(f"launcher-programs-normal-at-{at}")
    desktop.type_in("k")
    desktop.launcher_until(
        lambda shown: shown["gutter"]["shown"] and not shown["gutter"]["relative"]
    )
    assert desktop.showing()["gutter"]["room"] == gutter["room"], f"at {scale} {gutter}"
    desktop.shoot(f"launcher-programs-found-at-{at}")
    desktop.close_launcher()


def washes_what_it_finds(desktop: Desktop, scale: str) -> None:
    """Type a find that several of home's rows hold, and shoot the letters it washed.

    Each row's matched letters take the Search wash, the selected row's on the ground a shade
    past the selection, as his Neovim's hlsearch washes a match.
    """
    desktop.answer("s")
    desktop.launcher_until(lambda shown: shown["current"] == 0 and shown["count"] > 1)
    rows = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["rows"]
    washed = [row for row in rows if row["washed"]]
    assert len(washed) > 1, f"at {scale} few rows drawn washed a letter of s: {rows}"
    # Home's collections and a program's parts are found by their names too, Shortcuts and
    # Slack Theme among them, so every row found by its name washes what it matched.
    unwashed = [row["reads"] for row in rows if row["byName"] and not row["washed"]]
    assert not unwashed, f"at {scale} rows found by name washed nothing: {unwashed}"
    desktop.shoot(f"launcher-find-washed-at-{scale.replace('.', '-')}")
    desktop.close_launcher()


def counts_before_its_chord(desktop: Desktop, scale: str) -> None:
    """Step into Desktop, whose collections each open by a chord, and shoot their counts.

    A collection listed whole outside home says how many it holds at its right, a muted dot
    from its chord, as the status line joins its facts (VKCF3Z, 2026-11-05). A find's rows
    say no count, which would read all of each at every key.
    """
    desktop.launcher_until(lambda shown: not shown["open"])
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.down_to("Desktop")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["desktop"])
    selected = desktop.showing()["selected"]
    assert selected["side"], f"at {scale} Desktop's first has no count: {selected}"
    assert selected["chord"], f"at {scale} Desktop's first has no chord: {selected}"
    desktop.shoot(f"launcher-desktop-counts-at-{scale.replace('.', '-')}")
    desktop.close_launcher()


def finds_kitty(desktop: Desktop, scale: str) -> None:
    """Find kitty from its first letter, shooting the foot at k, then the folder under it.

    The first letter tells the foot more keys than it holds, so the completion, whose ghost the
    field shows, gives way, and Tab in stays, furthest from Return (VKCF3Z, 2026-11-06).
    Found, kitty lists the folder it visited under it and its note counts it, so the shots show
    a program holding what it does, at Joel's scales.
    """
    desktop.as_account(f"mkdir -p {VISITED} && zoxide add {VISITED}")
    desktop.on_find()
    desktop.type_in("k")
    desktop.launcher_until(lambda shown: shown["typed"] == "k")
    overlaps = desktop_session.foot_overlaps(desktop)
    assert not overlaps, f"at {scale} on k: {overlaps}"
    desktop.shoot(f"launcher-finds-k-at-{scale.replace('.', '-')}")
    desktop.type_in("itty")

    def holds_the_folder(shown: Json) -> bool:
        """Say whether kitty is selected, counting the folder listed just under it."""
        selected = shown["selected"] or {}
        if selected.get("is") != "program:kitty":
            return False
        at = shown["rows"].index("kitty")
        return "tidepool" in shown["rows"][at + 1 : at + 4] and "1 recent" in selected["note"]

    desktop.launcher_until(holds_the_folder)
    overlaps = desktop_session.foot_overlaps(desktop)
    assert not overlaps, f"at {scale} on kitty: {overlaps}"


def leaves_kitty(desktop: Desktop) -> None:
    """Close the launcher found on kitty, and forget the folder it visited."""
    desktop.close_launcher()
    desktop.as_account(f"zoxide remove {VISITED} && rmdir {VISITED}")


def teaches_in_the_foot(desktop: Desktop, scale: str) -> None:
    """See kitty's chord taught in the foot, by its keys and lowercase words, never on the line.

    The find's line holds only the find and how far it narrows (VKCF3Z, 2026-11-05).
    """
    field = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["field"]
    taught = [word["text"] for word in field if "next time" in word["text"]]
    assert not taught, f"at {scale} the find's line teaches a chord: {taught}"
    told = desktop.showing()["selected"]["told"]
    assert any(text.endswith(" next time") for text in told), f"at {scale} the foot: {told}"


def says_until_the_next_key(desktop: Desktop, scale: str) -> None:
    """Press a key with no meaning, l, h, q, n past the end and f to no row, shooting each.

    Each stands in the keys' place, the mark in text and its words muted, until the next key.
    kitty opens on Return and holds its windows, so Tab goes in, furthest from Return.
    Vim's wrapscan words and f's miss are the longest it says, so they're seen whole too.
    """
    machine = desktop.machine
    at = scale.replace(".", "-")
    told = desktop.showing()["selected"]["told"]
    assert told[1:2] == ["Tab in"], f"at {scale} kitty's foot: {told}"
    for chars, said, shot in [
        ("z", ["z not here"], "not-here"),
        ("l", ["l not here", "\u21b5 open"], "l-not-here"),
        ("h", ["h not here", "- up"], "h-not-here"),
        ("q", ["q does nothing", ":q closes"], "q-does-nothing"),
        ("Gn", ["search hit BOTTOM, continuing at TOP"], "n-wraps"),
        ("fz", ["no row below begins with z"], "f-misses"),
    ]:
        machine.send_chars(chars)
        desktop.launcher_until(lambda shown: shown["saying"] == said)  # noqa: B023
        foot = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
        cut = [word["text"] for word in foot if word["cut"]]
        assert not cut, f"at {scale} the foot cuts what it says: {cut}"
        desktop.shoot(f"launcher-{shot}-at-{at}")
    # gg stays on the first row and lets the words go.
    machine.send_chars("gg")
    desktop.launcher_until(lambda shown: shown["saying"] == [] and shown["open"])
    # Tab opens kitty's page, as Joel goes in; inside, the foot says - goes up, and - comes
    # back to kitty.
    desktop.into_program()
    desktop.launcher_until(lambda shown: "- up" in (shown["selected"] or {}).get("told", []))
    desktop.shoot(f"launcher-inside-at-{at}")
    desktop.step_out()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "program:kitty")


def lists_every_key(desktop: Desktop, scale: str) -> None:
    """Show every key normal takes with Space ?, as a plate in the drawer, and shoot it.

    Each key's name is whole, and what won't fit above the foot waits for the drawer's keys.
    """
    desktop.machine.send_chars(" ?")
    desktop.launcher_until(
        lambda shown: (
            shown["listing"]
            and shown["drawer"]["out"] is True
            and shown["drawer"]["title"] == "The Launcher's Keys"
        )
    )
    drawer = plate_shown(desktop, scale)
    assert drawer["labelsWhole"], f"at {scale} a key's name is broken: {drawer}"
    # Each key and what it does are at Note, as the reader's list sets them, so at 1.5 each
    # fits on one line. At 2 the plate leaves about 17 letters beside the widest key, so a
    # longer saying may take a second line, but it always stands beside its key.
    assert drawer["factsUnder"] == 0, f"at {scale} {drawer['factsUnder']} under their keys"
    if scale == "1.5":
        assert drawer["factsWrapped"] == 0, f"at 1.5 {drawer['factsWrapped']} keys wrap: {drawer}"
    assert drawer["more"] or drawer["lines"]["facts"][-1] < drawer["foot"], drawer
    assert bottom_set(drawer, drawer["lines"]["facts"]), (
        f"at {scale} the keys' last line isn't the inset above the foot: {drawer}"
    )
    desktop.shoot(f"launcher-keys-at-{scale.replace('.', '-')}")
    desktop.machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["open"] and not shown["listing"])


def shows_the_command_line(desktop: Desktop, scale: str) -> None:
    """Open the command line on kitty's row, shoot it and its menu, and let it go.

    The menu lists kitty's acts, Return's first, before :q, :help, :set and :{n}; the foot
    names the line's own keys, Tab until a line is chosen and Return once one is, never the
    rows'; a line chosen says in the drawer what it acts on, and Escape puts the find line
    back in normal.
    """

    def foot() -> list[str]:
        laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
        return [word["text"] for word in laid["foot"]]

    machine = desktop.machine
    everywhere = [
        "quit Close the Launcher",
        "help List the Keys",
        "set Set an Option",
        "setlocal Set an Option of This",
        "history List Earlier Lines",
        "{n} Go to a Row",
    ]
    machine.send_chars(":")
    desktop.launcher_until(
        lambda shown: (
            (shown["command"] or {}).get("line") == ""
            and shown["command"]["menu"][-len(everywhere) :] == everywhere
            and len(shown["command"]["menu"]) > len(everywhere)
        )
    )
    desktop.shoot(f"launcher-command-at-{scale.replace('.', '-')}")
    said = foot()
    assert any("choose" in text for text in said), said
    assert not any("below" in text or "\u21b5" in text for text in said), said
    machine.send_key("tab")
    desktop.launcher_until(
        lambda shown: (
            (shown["command"] or {}).get("chosen") == 0 and shown["drawer"]["body"] == "on kitty"
        )
    )
    said = foot()
    assert any("\u21b5" in text for text in said), said
    assert not any("choose" in text or "below" in text for text in said), said
    desktop.shoot(f"launcher-command-chosen-at-{scale.replace('.', '-')}")
    machine.send_key("esc")
    desktop.launcher_until(
        lambda shown: shown["open"] and shown["command"] is None and shown["mode"] == "normal"
    )


def lines_what_ends(desktop: Desktop, scale: str) -> None:
    """Find Lock, whose Return writes :lockscreen on the command line, and shoot both.

    Locking ends what's in use, so no Return does it: the row names :lockscreen, as d carries
    none of it, and the foot's Return says the line it writes, at the right end where Return
    always is (VKCF3Z, 2026-11-05). Typed down to lo, it lists lockscreen first, and it's
    shot there too. Escape lets the line go, back in normal.
    """
    machine = desktop.machine
    at = scale.replace(".", "-")
    desktop.on_find()
    machine.send_key("ctrl-u")
    desktop.launcher_until(lambda shown: shown["typed"] == "")
    desktop.type_in("lock", selecting="Lock")
    desktop.normal()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("carried") == ":lockscreen")
    told = desktop.showing()["selected"]["told"]
    assert told[:1] == ["\u21b5 :lockscreen"], f"at {scale} an ending row's foot says {told}"
    assert told[-2:] == [": commands", "Space ? keys"], f"at {scale} its foot: {told}"
    desktop.shoot(f"launcher-ends-at-{at}")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "lockscreen")
    desktop.shoot(f"launcher-ends-lined-at-{at}")
    machine.send_key("ctrl-u")
    machine.send_chars("lo")
    desktop.launcher_until(
        lambda shown: (
            (shown["command"] or {}).get("line") == "lo"
            and shown["command"]["menu"][:1] == ["lockscreen Lock"]
        )
    )
    desktop.shoot(f"launcher-command-lo-at-{at}")
    takes_the_head(desktop, scale)
    machine.send_key("esc")
    desktop.launcher_until(
        lambda shown: shown["open"] and shown["command"] is None and shown["mode"] == "normal"
    )


def takes_the_head(desktop: Desktop, scale: str) -> None:
    """Say the : line stands in the head in the find line's place, its completions below it.

    The : line and the find's / line are one line, as Vim's are, so the find line goes while it's
    open; its completions are the list, each named on the rows' names' column, so what's typed
    stands over the name it completes rather than indented under it (Joel, 2026-11-06).
    """
    laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
    line, lines = laid["line"], laid["lines"]
    mark = next((word for word in line if word["text"] == ":"), None)
    typed = next((word for word in line if word["text"] == "lo"), None)
    assert mark, f"at {scale} the : line has no colon: {line}"
    assert typed, f"at {scale} the : line doesn't read lo: {line}"
    assert laid["field"] == [], f"at {scale} the find line stays beside the : line: {laid['field']}"
    over = [word for word in lines if word["y"] <= mark["y"]]
    assert not over, f"at {scale} the : line isn't above its completions: {mark}, {over}"
    names = [word for word in lines if word["text"].startswith("lo")]
    assert names, f"at {scale} no completion is named lo: {lines}"
    assert names[0]["starts"] == typed["starts"], (
        f"at {scale} lockscreen is listed at {names[0]['starts']}, typed at {typed['starts']}"
    )


def lands_on_a_setting(desktop: Desktop, scale: str) -> None:
    """Find Keep Awake and Return, which lands on it in normal, and shoot it there.

    Its side and the foot say what a second Return does, in the accent, and nothing is done. A
    landing is normal, so its foot keeps normal's command line and key list before Return, its
    shape never changing; only the accent marks it (VKCF3Z, 2026-11-05).
    """
    machine = desktop.machine
    desktop.on_find()
    machine.send_key("ctrl-u")
    desktop.launcher_until(lambda shown: shown["typed"] == "")
    desktop.type_in("keep awake", selecting="Keep Awake")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["mode"] == "normal" and shown["landed"] != "")
    shown = desktop.showing()
    told = shown["selected"]["told"]
    assert told[:1] == [f"\u21b5 {shown['landed']}"], f"at {scale} the landed foot says {told}"
    assert told[-2:] == [": commands", "Space ? keys"], f"at {scale} the landed foot: {told}"
    desktop.shoot(f"launcher-landed-at-{scale.replace('.', '-')}")


def stands_in_the_mode_words_room(desktop: Desktop, typed: str, shot: str) -> None:
    """Type keys normal holds in the mode word's room, shoot them there, and let them go.

    They move nothing else in the foot, and Escape lets them go without a step back.
    """
    laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
    desktop.machine.send_chars(typed)
    desktop.launcher_until(lambda shown: shown["counted"] == typed)
    held = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
    rest = sorted(word["x"] for word in held if word["text"] != typed)
    assert rest == sorted(word["x"] for word in laid), (laid, held)
    desktop.shoot(shot.replace(".", "-"))
    desktop.machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["open"] and shown["counted"] == "")


# y flashes the rows it copies for a moment (his Neovim's on_yank), and
# d's ask stands on the line in the find's place, tinting the rows it
# covers (VKCF3Z, 2026-11-05). The flash is gone in a fifth of a second, too soon to wait
# for the screen to hold still, so the screen is taken as fast as QEMU
# gives it, and the picture kept is the one furthest from the screen once
# the flash has gone. Whether y flashed is held by what the launcher says;
# the picture is for a person. QEMU's screendumps, each a whole screen at
# 2, came too slowly to land inside the fifth of a second: on 2026-11-05
# none of four caught it in three tries each. So the screen is recorded
# through niri, as Joel's recorder does, which keeps every frame niri
# draws, and the frame kept is the one furthest from the last.
RECORDING = "/run/user/1000/flash.mkv"
RECORDED = "/run/user/1000/flash.log"
FRAMES = "/run/user/1000/flash"


def test_y_flashes_its_rows_and_ds_ask_stands_on_the_line(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Y flashes the row it copies, and d's ask stands on the line, tinting what it covers."""
    machine = desktop.machine
    # A second window, so dj has two to close and asks once for both,
    # where one closes at once.
    wednesday = desktop.opened("Notes for Wednesday")
    desktop.notes_window()
    for scale in at_each_scale:
        at_scale(desktop, scale)
        named = scale.replace(".", "-")
        # A row flashes as it's copied, and an answer with no rows, a
        # sum, flashes its ghost in the field.
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("tuesday", selecting="Notes for Tuesday")
        flashes_as_yy_copies(
            desktop, [0], f"launcher-yank-flash-at-{named}", f"launcher-yanked-at-{named}"
        )
        desktop.answer("12*7")
        flashes_as_yy_copies(
            desktop,
            [-1],
            f"launcher-yank-ghost-flash-at-{named}",
            f"launcher-yanked-ghost-at-{named}",
        )
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("notes for")
        # The windows lead, the title yy just copied below them.
        desktop.launcher_until(
            lambda shown: sorted(shown["rows"][:2]) == ["Notes for Tuesday", "Notes for Wednesday"]
        )
        desktop.normal()
        machine.send_chars("gg")
        desktop.launcher_until(lambda shown: shown["current"] == 0)
        before = desktop.showing()
        # d waiting says at once what it will take, the row's name whole,
        # since two windows here begin "Notes": the row's keys, which no
        # key does while d waits, give way to it. It moves nothing; j then
        # takes both rows, and asks once for both.
        machine.send_chars("d")
        desktop.launcher_until(lambda shown: shown["modeWord"].startswith("d \u00b7 close"))
        shown = desktop.showing()
        assert shown["modeWord"] == "d \u00b7 close Notes for Tuesday", f"at {scale} {shown}"
        assert shown["hints"] == [], f"at {scale} {shown}"
        assert shown["tinted"] == [shown["current"]], shown
        assert len(shown["glance"]) == len(before["glance"]), f"at {scale} d cut {shown}"
        desktop.shoot(f"launcher-pending-at-{named}")
        with desktop.told("ask ready"), desktop.told("asked Close 2 windows? (y/n)"):
            machine.send_chars("j")
        # The ask stands on the line, in the find's place, as every ask
        # does, said whole, and tints the rows it covers as visual does.
        # The foot keeps how things stand, whole (a minute turning keeps
        # its length), and its keys stay put.
        shown = desktop.showing()
        assert shown["question"] == "Close 2 windows? (y/n)", shown
        assert shown["questionWhole"], f"at {scale} the ask was cut: {shown}"
        assert shown["tinted"] == [0, 1], shown
        # The rows it covers wear the tint alone, the find's wash giving
        # way on each, and the line leads with no glyph, since it isn't a
        # find while it asks.
        laid = desktop_session.laid_out(desktop)
        covered = [row for row in laid["rows"] if row["at"] in shown["tinted"]]
        assert covered, f"at {scale} no covered row was drawn: {laid['rows']}"
        assert not any(row["washed"] for row in covered), f"at {scale} washed {covered}"
        leading = min(laid["field"], key=lambda word: word["x"])
        assert leading["text"].startswith("Close"), f"at {scale} the line leads {leading}"
        assert len(shown["glance"]) == len(before["glance"]), f"at {scale} the foot gave way"
        desktop.shoot(f"launcher-asks-at-{named}")
        with desktop.told("answered no"):
            machine.send_key("esc")
        desktop.launcher_until(lambda shown: shown["open"] and shown["question"] == "")
        after_ask = desktop.showing()
        assert after_ask["keysAt"] == before["keysAt"], f"at {scale} the keys moved"
        assert after_ask["tinted"] == [], f"at {scale} {after_ask}"
        desktop.close_launcher()
    desktop.as_account(f"niri msg action close-window --id {wednesday['id']}")


def flashes_as_yy_copies(desktop: Desktop, rows: list[int], flash: str, settled: str) -> None:
    """Copy what's selected with yy, see `rows` flash, and keep the frame furthest from after."""
    machine = desktop.machine
    desktop.normal()
    shots = desktop.values["shots"]
    if shots:
        # The output named, as Joel's recorder names it (Recorder.qml), since
        # with more than one wf-recorder asks which; and every frame taken,
        # damaged or not (-D), since on a still screen it waited for a frame
        # to end on once told to stop, and never ended.
        output = (
            "niri msg --json focused-output"
            ' | grep -o \'"name":"[^"]*"\' | head -1 | cut -d\'"\' -f4'
        )
        desktop.as_account(
            f'rm -f {RECORDING}; setsid -f wf-recorder -y -D --output "$({output})"'
            f" --file {RECORDING} </dev/null >{RECORDED} 2>&1"
        )
        # It says the region it records once it has begun; the file stays
        # empty a while, the encoder holding frames back on a still screen.
        began, _ = machine.execute(
            f"timeout 10 sh -c 'until grep -q \"selected region\" {RECORDED}; do sleep 0.1; done'"
        )
        assert began == 0, f"wf-recorder never began: {machine.execute(f'cat {RECORDED}')[1]}"
    since = desktop.mark()
    machine.send_chars("yy")
    desktop.reported("showing ", lambda said: json.loads(said).get("flashing") == rows, since)
    desktop.launcher_until(lambda shown: shown["flashing"] == [])
    desktop.settled_screen(settled)
    if shots:
        machine.succeed("pkill -INT -x wf-recorder")
        machine.wait_until_fails("pgrep -x wf-recorder", timeout=timedelta(seconds=10))
        flashed_frame(desktop).save(machine.out_dir / f"{flash}.png")
    desktop.close_launcher()


def flashed_frame(desktop: Desktop) -> Image.Image:
    """Give the recording's frame furthest from its last, the flash at its height."""
    machine = desktop.machine
    ffmpeg = f"{desktop.values['ffmpeg_headless']}/bin/ffmpeg -v error"
    machine.succeed(
        f"rm -rf {FRAMES} && mkdir -p {FRAMES}"
        f" && {ffmpeg} -i {RECORDING} -fps_mode passthrough {FRAMES}/%03d.png"
        f" && tar -C {FRAMES} -cf {FRAMES}.tar ."
    )
    machine.copy_from_machine(f"{FRAMES}.tar")
    frames = []
    with tarfile.open(machine.out_dir / "flash.tar") as kept:
        for member in sorted(kept.getmembers(), key=lambda member: member.name):
            read = kept.extractfile(member)
            if read is not None:
                frames.append(Image.open(read).convert("RGB"))
    (machine.out_dir / "flash.tar").unlink()
    return max(frames, key=partial(changed_from, frames[-1]))


def changed_from(after: Image.Image, frame: Image.Image) -> float:
    """Say how far a picture is from another, as the mean of its differences' light."""
    return float(ImageStat.Stat(ImageChops.difference(frame, after).convert("L")).mean[0])


# Visual line counts the rows it holds in the mode word, and what arrives
# while they're chosen waits, counted last among the foot's facts, beside
# the mode holding them still (VKCF3Z, 2026-11-05), shot at each
# of Joel's scales for a person to judge.


def test_visual_counts_its_rows_and_what_arrives_waits_in_the_foot(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """V j counts two in the mode word, and a window arriving waits as "1 new" in the foot.

    It's shown in Super+Tab's windows, which is all of them, since a typed search said again
    finds only what it found. The foot names y, one of what visual chooses rows for, with "1
    new" beside it or not: holding "999 chosen" and Return's room for verbs no window says,
    it gave y way (VKCF3Z, 2026-11-06).
    """
    machine = desktop.machine
    desktop.notes_window()
    others = [desktop.opened(title) for title in ("Notes for Thursday", "Notes for Friday")]
    for scale in at_each_scale:
        at_scale(desktop, scale)
        named = scale.replace(".", "-")
        machine.send_key("meta_l-tab")
        desktop.launcher_ready()
        desktop.launcher_until(
            lambda shown: shown["held"] == ["windows"] and len(shown["rows"]) > 1
        )
        desktop.normal()
        machine.send_chars("ggVj")
        desktop.launcher_until(lambda shown: shown["modeWord"] == "2 chosen")
        hints = desktop.showing()["hints"]
        assert "y copy" in hints, f"at {scale} the foot drew {hints}"
        arriving = desktop.opened(f"Notes for the shot at {named}")
        desktop.launcher_until(
            lambda shown: shown["waiting"] == 1 and shown["glance"].endswith("1 new")
        )
        hints = desktop.showing()["hints"]
        assert "y copy" in hints, f"at {scale} with one new the foot drew {hints}"
        overlaps = desktop_session.foot_overlaps(desktop)
        assert not overlaps, f"at {scale} with one new: {overlaps}"
        desktop.shoot(f"launcher-visual-waits-at-{named}")
        machine.send_key("esc")
        desktop.launcher_until(
            lambda shown: shown["waiting"] == 0 and not shown["glance"].endswith("new")
        )
        desktop.close_launcher()
        desktop.close(arriving)
    for other in others:
        desktop.close(other)


def test_space_right_goes_into_the_drawer_its_rule_on_the_cursors_line(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Go into the key list's drawer at each of Joel's scales, move there, and shoot it.

    Gone into, the drawer holds the keys: its frame takes the accent and its cursor's line the
    rule in its left margin. Five lines down and four letters along, it's shot with the
    foot at the laptop's size, for a person to judge, then with x's E21 and q's words where the
    line and the foot were, and Ctrl+W h comes back.
    """

    def relisted() -> None:
        closed_from_the_list(desktop)
        listed_keys(desktop)

    for scale in at_each_scale:
        at_scale(desktop, scale)
        listed_keys(desktop)
        # The l alone steps into the list's row, so the list is shown again.
        pressed_until(
            desktop, "ctrl-w l", lambda shown: shown["drawer"]["cursor"] is not None, relisted
        )
        line = int(desktop.showing()["drawer"]["cursor"]["line"])
        pressed(desktop, "5 j l l l l")
        desktop.launcher_until(partial(moved_to, line + 5))
        drawn = desktop.showing()["drawer"]
        assert drawn["keys"], f"at {scale} the drawer doesn't hold the keys: {drawn}"
        assert drawn["cursor"]["rule"]["width"] > 0, f"at {scale} no rule: {drawn['cursor']}"
        desktop.shoot(f"launcher-drawer-in-at-{scale.replace('.', '-')}")
        # A page can't be changed, so x says Vim's E21 where the line was, and q how to close.
        pressed(desktop, "x")
        desktop.launcher_until(lambda shown: shown["commandSaid"] == "E21: Cannot make changes")
        desktop.shoot(f"launcher-drawer-refused-at-{scale.replace('.', '-')}")
        pressed(desktop, "q")
        desktop.launcher_until(lambda shown: shown["saying"] == ["q does nothing", ":q closes"])
        desktop.shoot(f"launcher-drawer-quit-at-{scale.replace('.', '-')}")
        # Space ? in it lists the drawer's own keys there; the pair come apart leaves ? alone,
        # a search back, which Escape lets go.
        pressed_until(
            desktop,
            "spc shift-0x35",
            lambda shown: shown["drawer"]["title"] == "The Drawer's Keys",
            lambda: pressed(desktop, "esc"),
        )
        desktop.shoot(f"launcher-drawer-listed-at-{scale.replace('.', '-')}")
        # The h alone moves the cursor a letter, which leaves nothing to put back.
        pressed_until(
            desktop,
            "ctrl-w h",
            lambda shown: shown["mode"] == "normal" and not shown["drawer"]["keys"],
            lambda: None,
        )
        closed_from_the_list(desktop)


def test_the_drawers_search_and_visual_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Search the key list's drawer, choose in it and copy, shooting each at both scales.

    What's typed after / is washed as it's typed; v chooses letters and V lines, the mode word
    counting them, "3 chosen"; and y flashes what it took. Each is kept for a person
    to judge: the wash and the choice change fill and colour, never weight.
    """

    def relisted() -> None:
        closed_from_the_list(desktop)
        listed_keys(desktop)

    for scale in at_each_scale:
        named = scale.replace(".", "-")
        at_scale(desktop, scale)
        listed_keys(desktop)
        pressed_until(
            desktop, "ctrl-w l", lambda shown: shown["drawer"]["cursor"] is not None, relisted
        )
        pressed(desktop, "slash d o w n")
        desktop.launcher_until(lambda shown: shown["drawer"]["search"]["typing"] == "down")
        desktop.shoot(f"launcher-drawer-washed-at-{named}")
        pressed(desktop, "ret")
        desktop.launcher_until(partial(landed_on, "Down"))
        pressed(desktop, "v l l l l")
        desktop.launcher_until(partial(chose, "letters", 5))
        desktop.shoot(f"launcher-drawer-letters-at-{named}")
        pressed(desktop, "esc shift-v j j")
        desktop.launcher_until(partial(chose, "lines", 3))
        desktop.shoot(f"launcher-drawer-lines-at-{named}")
        caught_flash(desktop, f"launcher-drawer-flash-at-{named}")
        pressed_until(
            desktop,
            "ctrl-w h",
            lambda shown: shown["mode"] == "normal" and not shown["drawer"]["keys"],
            lambda: None,
        )
        closed_from_the_list(desktop)


def landed_on(word: str, shown: Json) -> bool:
    """Say whether the drawer's cursor stands on the word's first letter.

    The report's words join a line's parts with a space its letters don't count, so the cursor's
    letter is read from what it's on rather than as a place in them.
    """
    cursor = shown["drawer"]["cursor"]
    return cursor is not None and cursor["on"] == word[0] and word in str(cursor["says"])


def chose(by: str, count: int, shown: Json) -> bool:
    """Say whether the drawer's visual has chosen so many, by letters or by lines."""
    chosen = shown["drawer"]["chosen"]
    return bool(chosen["by"] == by and chosen["count"] == count)


def caught_flash(desktop: Desktop, name: str) -> None:
    """Copy what's chosen with y, keeping a picture of its flash in the gallery.

    The flash is shorter than a picture held still, and a loaded builder has taken over a third
    of a second to show a key at all, so the screen is taken over and over from y until it first
    changes from the choice, then at once a few times more. A shot that differs from both the
    choice before and the page after is the flash. One that missed it chooses the same three
    lines again and copies them again.
    """
    seen = []
    for tried in range(TRIES):
        if tried:
            pressed(desktop, "shift-v j j")
            desktop.launcher_until(partial(chose, "lines", 3))
        desktop.held_still()
        choice = desktop.screen_now()
        pressed(desktop, "y")
        pressed_at = time.monotonic()
        first = desktop.screen_now()
        while (
            changed(first, choice) < FLASHED
            and time.monotonic() - pressed_at < TRIED.total_seconds()
        ):
            first = desktop.screen_now()
        shots = [(first, time.monotonic() - pressed_at)] + [
            (desktop.screen_now(), time.monotonic() - pressed_at) for _ in range(FLASH_SHOTS)
        ]
        desktop.launcher_until(
            lambda shown: shown["mode"] == "page" and not shown["drawer"]["chosen"]["flashing"]
        )
        # The screen follows the shell late, so the page is taken once it's done drawing.
        desktop.held_still()
        page = desktop.screen_now()
        for shot, after in shots:
            before, since = changed(shot, choice), changed(shot, page)
            if before >= FLASHED and since >= FLASHED:
                if desktop.values["shots"]:
                    shot.save(desktop.machine.out_dir / f"{name}.png")
                return
            seen.append(f"{after * 1000:.0f} ms: {before} from the choice, {since} from the page")
    missed = f"{NEVER_FLASHED}; each picture after y: {'; '.join(seen)}"
    raise AssertionError(missed)


def changed(one: Image.Image, other: Image.Image) -> int:
    """Count the pixels two pictures of the screen differ at by more than a little light."""
    difference = ImageChops.difference(one, other).convert("L")
    return difference.point(lambda light: 255 if light > CHANGED else 0).histogram()[255]


def pressed(desktop: Desktop, keys: str) -> None:
    """Press each key QEMU names, held a moment, the next following at once."""
    for key in keys.split():
        desktop.machine.send_monitor_command(f"sendkey {key} {HELD_MS}")


def pressed_until(
    desktop: Desktop, keys: str, test: Callable[[Json], bool], again: Callable[[], None]
) -> None:
    """Press the keys and wait until the launcher passes the test, pressing them again if not.

    Space waits Joel's 300 ms for the arrow, which a loaded builder has kept from the shell
    longer, so a pair that came apart is pressed again once `again` puts back what the arrow
    did alone.
    """
    for _ in range(TRIES - 1):
        pressed(desktop, keys)
        try:
            desktop.launcher_until(test, TRIED)
        except AssertionError:
            print(f"{keys} came apart, the wait letting go between them: pressed again")
            again()
        else:
            return
    pressed(desktop, keys)
    desktop.launcher_until(test)


def closed_from_the_list(desktop: Desktop) -> None:
    """Close the launcher with the key list shown, or not.

    Escape puts the list away first, which changes nothing close_launcher waits on.
    """
    if desktop.showing()["listing"]:
        desktop.machine.send_key("esc")
        desktop.launcher_until(lambda shown: shown["open"] and not shown["listing"])
    desktop.close_launcher()


def listed_keys(desktop: Desktop) -> None:
    """Open the launcher, and list its keys with Space ? in the drawer.

    Space waits for ? as it does for an arrow, so a pair that came apart is pressed again in a
    launcher opened afresh.
    """

    def opened() -> None:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.normal()

    def reopened() -> None:
        desktop.close_launcher()
        opened()

    opened()
    pressed_until(
        desktop,
        "spc shift-0x35",
        lambda shown: shown["listing"] and shown["drawer"]["out"],
        reopened,
    )


def moved_to(below: int, shown: Json) -> bool:
    """Say whether the drawer's cursor is on a line, four letters along or at its last."""
    cursor = shown["drawer"]["cursor"]
    return bool(
        cursor is not None
        and cursor["line"] == below
        and cursor["letter"] == min(4, len(cursor["says"]) - 1)
    )


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
        assert things_apart(facts, wrapped), f"at {scale} a fact is off the pitch: {drawer}"
        assert on_the_beat(lines["words"], float(card["rows"][0])), drawer
        under = lines["words"] - facts[-1]
        apart = under >= 2 * PITCH - NEAR if wrapped else abs(under - 2 * PITCH) <= NEAR
        assert apart, f"at {scale} the notice's words aren't a line under its facts: {drawer}"
        # Signed as its card signs it, from its sender, and never a time, which a fact says
        # (VKCF3Z, 2026-11-08); notify-send files no kind to follow it.
        assert drawer["eyebrow"] == "from Desktop", drawer
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
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"] is True
                and shown["drawer"]["title"] == "time.harbour.test"
                and len(shown["drawer"]["lines"]["facts"]) > 1
            )
        )
        drawer = plate_shown(desktop, scale)
        # The title, at Title, may take two lines of the plate's width, its leading apart;
        # the facts start on the card's beat under it, on the pitch.
        facts = drawer["lines"]["facts"]
        assert after_the_title(facts[0], drawer["lines"], drawer["cardLines"]), drawer
        assert things_apart(facts, drawer["factsWrapped"] > 0), (
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
        assert things_apart(facts, drawer["factsWrapped"] > 0), (
            f"at {scale} the zone's facts are off the pitch: {drawer}"
        )
        assert drawer["besideOnOneLine"], f"at {scale} a value wraps beside its name: {drawer}"
        desktop.shoot(f"launcher-plate-zone-at-{scale.replace('.', '-')}")
        desktop.close_launcher()


def home_network_plate(desktop: Desktop, scale: str, title: str) -> None:
    """Wait for a thing of the home network's plate, and hold its facts whole and one way."""
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and shown["drawer"]["title"] == title
            and len(shown["drawer"]["lines"]["facts"]) > 1
        )
    )
    drawer = plate_shown(desktop, scale)
    assert drawer["labelsWhole"], f"at {scale} a fact's name is broken: {drawer}"
    assert drawer["valuesWhole"], f"at {scale} a fact's value breaks a word: {drawer}"
    assert drawer["laidOneWay"], f"at {scale} its facts mix beside and under: {drawer}"


def home_network_laid_out(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Hold the Home Network to the eero daemon's record, and each thing's plate whole.

    The record the stand-in writes (tests/desktop.nix) reaches the place: the internet first,
    then the gateway node with its devices under it, and each node after the one it hangs from.
    A node's, a device's and a profile's plates lay their facts out one way, every name and value
    whole, at both of Joel's scales.
    """
    desktop.machine.wait_until_succeeds(
        desktop.in_session('test -s "$XDG_RUNTIME_DIR/eero/home-network.json"'),
        timeout=timedelta(seconds=30),
    )
    for scale in at_each_scale:
        at_scale(desktop, scale)
        at = scale.replace(".", "-")
        desktop.answer("home network")
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name") == "Home Network"
        )
        desktop.step_in()
        desktop.launcher_until(
            lambda shown: (
                shown["rows"][:4] == ["Internet", "Hall", "generic 3", "phone 1"]
                and "Den" in shown["rows"]
                and shown["rows"].index("Den") < shown["rows"].index("Study")
            )
        )
        home_network_plate(desktop, scale, "Internet")
        desktop.shoot(f"launcher-home-network-at-{at}")
        desktop.down_to("Study")
        home_network_plate(desktop, scale, "Study")
        desktop.shoot(f"launcher-home-network-node-at-{at}")
        desktop.down_to("unknown computer 1")
        home_network_plate(desktop, scale, "unknown computer 1")
        desktop.shoot(f"launcher-home-network-device-at-{at}")
        desktop.down_to("profile 1")
        home_network_plate(desktop, scale, "profile 1")
        desktop.shoot(f"launcher-home-network-profile-at-{at}")
        desktop.close_launcher()
        devices_plates_lie_on_the_lines(desktop, scale)
        the_desk_lights_plate_lies_on_the_lines(desktop, scale)
        the_screens_plate_says_what_sets_it(desktop, scale)


def devices_plates_lie_on_the_lines(desktop: Desktop, scale: str) -> None:
    """Hold the speakers', the microphone's and the screen's plates to the lines, as a server's.

    Each says which way it goes or what it is over its name, then its parts, what moves most
    first, laid one way, every name whole (VKCF3Z, 2026-11-07). A sound device's State, as
    pipewire says it, leads its level (VKCF3Z, 2026-11-08); the stand-in backlight lights the
    one screen, so its Brightness leads.
    """
    for collection, typed, kind, leading in [
        ("sound", "speakers", "output", ["State", "Volume"]),
        ("sound", "microphone", "input", ["State", "Gain"]),
        ("displays", "", "screen", ["Brightness"]),
    ]:
        first = leading[-1]
        desktop.launcher_until(lambda shown: not shown["open"])
        desktop.as_account(f"qs ipc call launcher collection {collection}")
        desktop.launcher_ready()
        if typed:
            desktop.type_in(typed, selecting=typed)

        def plated(shown: Json, first: str = first) -> bool:  # As launcher_until calls it.
            """Say whether the drawer is out, holding this plate's first fact, its facts laid.

            A busy VM has reported a plate out before its facts were placed, all on one line,
            and a fact that arrives late, as Curve does when the backlight daemon writes its
            curve, lays them out again; so the plate is waited for laid, on the pitch, every
            name whole and one way, rather than held to it in a later snapshot.
            """
            drawer = shown["drawer"]
            return (
                drawer["out"] is True
                and first in drawer["facts"]
                and things_apart(drawer["lines"]["facts"], drawer["factsWrapped"] > 0)
                and drawer["labelsWhole"] is True
                and drawer["laidOneWay"] is True
            )

        desktop.launcher_until(plated)
        drawer = plate_shown(desktop, scale)
        assert str(drawer["eyebrow"]).lower().endswith(kind), f"at {scale}: {drawer}"
        assert drawer["facts"][: len(leading)] == leading, (
            f"at {scale} {typed or kind} leads wrong: {drawer}"
        )
        desktop.shoot(f"launcher-plate-{typed or 'screen'}-at-{scale.replace('.', '-')}")
        desktop.close_launcher()


@pytest.mark.usefixtures("google")
def test_a_persons_page_names_what_its_rows_hold_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A person's Unread and Recent say their names after theirs, how many at their right.

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
        assert (unread["count"], unread["side"]) == ("Lunch on Friday", "1"), unread
        desktop.shoot(f"launcher-person-at-{scale.replace('.', '-')}")
        desktop.close_launcher()


def the_desk_lights_plate_lies_on_the_lines(desktop: Desktop, scale: str) -> None:
    """Hold the desk light's plate to the lines, as a sound device's.

    A stand-in Litra Glow (desk-light-stand-in.py) is plugged in, lit as a Glow first is, and
    the desk light answer's drawer says where it is and what it is over the light's own name,
    then its output in lumens of its range, its HID device and its serial (VKCF3Z, 2026-11-08).
    It's unplugged again after, so no later shot holds a light that test order put there.
    systemd's hwdb opens a Litra to the seat by its USB parent, which a uhid stand-in hasn't, so
    its node is given to the account here, as udev gives a real one; `litra watch` tries again
    when it is.
    """
    machine = desktop.machine
    light = desktop.values["desk_light"]
    machine.succeed(
        "systemctl is-active --quiet desk-light"
        f" || systemd-run --unit=desk-light {light} litra /run/desk-light.log"
    )
    try:
        found = machine.wait_until_succeeds(
            "grep -l HID_ID=0003:0000046D:0000C900 /sys/class/hidraw/*/device/uevent",
            timeout=timedelta(seconds=30),
        )
        node = found.split("/")[4]
        machine.succeed(f"chown {desktop.account} /dev/{node}")
        desktop.answer("desk light")
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"] is True
                and "Output" in shown["drawer"]["facts"]
                and things_apart(
                    shown["drawer"]["lines"]["facts"], shown["drawer"]["factsWrapped"] > 0
                )
            )
        )
        drawer = plate_shown(desktop, scale)
        assert drawer["eyebrow"] == "on USB \u00b7 desk light", f"at {scale}: {drawer}"
        assert drawer["facts"] == ["Output", "HID Device", "Serial"], f"at {scale}: {drawer}"
        assert things_apart(drawer["lines"]["facts"], drawer["factsWrapped"] > 0), (
            f"at {scale} the desk light's facts are off the pitch: {drawer}"
        )
        assert drawer["labelsWhole"], f"at {scale} a fact's name is broken: {drawer}"
        assert drawer["laidOneWay"], f"at {scale} its facts mix beside and under: {drawer}"
        desktop.shoot(f"launcher-plate-desk-light-at-{scale.replace('.', '-')}")
        desktop.close_launcher()
    finally:
        machine.succeed("systemctl stop desk-light")


def the_screens_plate_says_what_sets_it(desktop: Desktop, scale: str) -> None:
    """Hold the screen's plate to the lines with the lid shut, then following the room.

    The backlight daemon says what it settled, whole by a rename, in backlight-now, and keeps
    its curve in learned; the VM's stand-in backlight and sensor give it no real figures, so
    both are written here as it writes them. With the lid shut, Follows is gone and the Room
    Light still says the stand-in sensor's live reading; following the room on its way to
    what it asks, with a curve, it says both, the Curve last of the light, every point
    dimmest first (VKCF3Z, 2026-11-08).
    """
    now = "/run/user/1000/backlight-now"
    learned = "~/.local/state/backlight/learned"

    def settled(said: str) -> None:
        """Say what the daemon settled, as it does."""
        desktop.as_account(f"printf '{said}' > {now}.writing && mv {now}.writing {now}")

    def shown_with(labels: list[str], missing: str) -> Callable[[Json], bool]:
        """Give a test that the plate is out and laid, saying these and not the one missing."""

        def laid(shown: Json) -> bool:  # As launcher_until calls it.
            """Say whether the plate says these facts, laid on the pitch, whole, one way."""
            drawer = shown["drawer"]
            return (
                drawer["out"] is True
                and all(label in drawer["facts"] for label in labels)
                and missing not in drawer["facts"]
                and things_apart(drawer["lines"]["facts"], drawer["factsWrapped"] > 0)
                and drawer["labelsWhole"] is True
                and drawer["laidOneWay"] is True
            )

        return laid

    at = scale.replace(".", "-")
    desktop.as_account(f"rm -f {learned}")
    settled("level\\t0.600\\nlux\\t28\\nasks\\t0.950\\nfollowing\\tno\\n")
    desktop.launcher_until(lambda shown: not shown["open"])
    desktop.as_account("qs ipc call launcher collection displays")
    desktop.launcher_ready()
    desktop.launcher_until(shown_with(["Brightness", "Room Light"], "Follows"))
    drawer = plate_shown(desktop, scale)
    room = drawer["values"][drawer["facts"].index("Room Light")]
    assert room.endswith(" lx"), f"at {scale}: {drawer}"
    desktop.shoot(f"launcher-plate-screen-lid-shut-at-{at}")
    curve = "0 0.150\\n6 0.377\\n28 0.950\\n10000 1.000\\n"
    desktop.as_account(
        f"mkdir -p $(dirname {learned}) && printf '{curve}'"
        f" > {learned}.writing && mv {learned}.writing {learned}"
    )
    settled("level\\t0.600\\nlux\\t28\\nasks\\t0.950\\nfollowing\\tyes\\n")
    desktop.launcher_until(shown_with(["Brightness", "Follows", "Curve"], ""))
    drawer = plate_shown(desktop, scale)
    facts = drawer["facts"]
    assert facts.index("Follows") < facts.index("Curve"), f"at {scale}: {drawer}"
    assert drawer["values"][facts.index("Follows")] == "the room \u00b7 on its way to 95%", (
        f"at {scale}: {drawer}"
    )
    desktop.shoot(f"launcher-plate-screen-following-at-{at}")
    desktop.close_launcher()
    desktop.as_account(f"rm -f {now} {learned}")


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
        # time a Display pitch under it with its seconds on its baseline, and what needs Joel a
        # Voice pitch under the time.
        assert abs(lines["eyebrow"] - card["fieldBase"]) <= NEAR, (
            f"at {scale} Now's eyebrow is off the field's baseline: {drawer}"
        )
        assert abs(lines["time"] - lines["eyebrow"] - DISPLAY_PITCH) <= NEAR, (
            f"at {scale} the time isn't a Display pitch under the eyebrow: {drawer}"
        )
        assert abs(lines["seconds"] - lines["time"]) <= NEAR, drawer
        head = lines["time"]
        if moment["say"]:
            assert abs(lines["say"] - lines["time"] - PITCH) <= NEAR, (
                f"at {scale} what needs Joel isn't a Voice pitch under the time: {drawer}"
            )
            head = lines["say"]
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


# The file picker's plate, for each kind of file it previews and one it doesn't, at both of
# Joel's scales: its eyebrow the kind with its numbers, its name, its preview right under it,
# and its facts after, every line on the pitch, the facts said from the folder at once and the
# preview coming once file-preview has said (VKCF3Z, 2026-11-03). Each is its kind, what its
# eyebrow says and what it previews.
def asked_in(desktop: Desktop, place: str, rows: set[str]) -> None:
    """Wait until a fresh ask holds the keys in the place given, with these rows listed.

    An ask is shown before its folder's read, so a row selected isn't its rows listed, and
    before the launcher holds the keys, so a key sent then goes nowhere: down_to reads the
    rows once and sends its keys at once, and needs both.
    """
    desktop.launcher_until(
        lambda shown: (
            shown["keys"]
            and shown["path"][-1:] == [place]
            and shown["selected"] is not None
            and rows <= set(shown["rows"])
        ),
        timeout=timedelta(seconds=60),
    )
    # The folder he's in is always named, however little room the crumbs have.
    drawn = desktop.showing()["pathDrawn"]
    assert drawn[-1]["said"].endswith(place.rsplit("/", 1)[-1]), f"{place} isn't named: {drawn}"


PICKED = {
    "drafts": ("Folder", "lines"),
    "archive.zip": ("ZIP", None),
    "clip.mp4": ("MP4 \u00b7 0:20", "picture"),
    "notes.md": ("Markdown \u00b7 5\u00a0lines", "lines"),
    "plan.pdf": ("PDF \u00b7 1\u00a0page", "picture"),
    "wide.png": ("PNG \u00b7 1600\u00a0\u00d7\u00a01000", "picture"),
}


def previewed(shown: Json, name: str, preview: str | None) -> bool:
    """Say whether the plate is out for the file named, its facts known and its preview in.

    Its row says its size, or a folder its count, and when it changed, rather than its folder,
    even though the folder was read before the ask was shown.
    """
    drawer = shown["drawer"]
    if drawer["out"] is not True or drawer["title"] != name or None in drawer["values"]:
        return False
    if "\u00b7" not in (shown["selected"] or {}).get("side", ""):
        return False
    held = drawer["preview"] or {}
    if preview == "picture":
        return bool(held.get("picture"))
    if preview == "lines":
        return bool(held.get("lines"))
    return drawer["preview"] is None


def test_the_file_pickers_plate_previews_each_kind_on_the_cards_lines(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """The file picker's plate shows each file's kind, preview and facts, on the pitch."""
    machine = desktop.machine
    values = desktop.values
    folder = f"/home/{desktop.account}/picked"
    chose = "/run/user/1000/portal-chose"
    # A note's first lines, each line break written for printf to make.
    notes = (
        "# The Plan\\n\\nPreviews, sizes and when each file changed,\\nthen marking,"
        "\\n  then a one-file picker, then sorting."
    )
    desktop.as_account(
        f"mkdir -p {folder}/drafts/older && cp {values['wide']} {folder}/wide.png"
        f" && cp {values['clip']} {folder}/clip.mp4 && cp {values['page']} {folder}/plan.pdf"
        f" && printf '{notes}\\n' > {folder}/notes.md"
        f" && head -c 48000 /dev/urandom > {folder}/archive.zip"
        f" && touch {folder}/drafts/one.md {folder}/drafts/two.md && chmod -R u+w {folder}"
    )
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            machine.succeed(f"rm -f {chose}")
            desktop.as_account(
                "systemd-run --user --collect --unit=choose-to-preview"
                f" {values['choose_a_file']} open {folder}"
            )
            asked_in(desktop, "~/picked", set(PICKED))
            for name, (eyebrow, preview) in PICKED.items():
                desktop.down_to(name)
                desktop.launcher_until(partial(previewed, name=name, preview=preview))
                drawer = plate_shown(desktop, scale)
                assert drawer["eyebrow"] == eyebrow, f"{name}'s eyebrow: {drawer}"
                assert drawer["drawnFacts"][1:] == ["Changed", "Where"], drawer
                facts = drawer["lines"]["facts"]
                shown = drawer["lines"]["previewLines"]
                body = body_line(drawer["lines"], drawer["cardLines"])
                assert body is not None, f"at {scale} {name}'s title is off its leading: {drawer}"
                assert all(on_the_pitch(line, body) for line in shown), (
                    f"at {scale} {name}'s preview is off the pitch: {drawer}"
                )
                assert all(on_the_beat(line, body) for line in facts), (
                    f"at {scale} {name}'s facts are off the beat: {drawer}"
                )
                assert things_apart(facts, drawer["factsWrapped"] > 0), (
                    f"at {scale} {name}'s facts are off the pitch: {drawer}"
                )
                if preview is not None:
                    assert facts[0] > drawer["lines"]["preview"], f"{name}'s facts lead: {drawer}"
                assert facts[-1] < drawer["foot"], f"at {scale} {name}'s facts are cut: {drawer}"
                said = name.replace(".", "-")
                desktop.shoot(f"file-picker-{said}-at-{scale.replace('.', '-')}")
            # :q refuses the ask from any folder, Escape closing nothing.
            desktop.close_launcher()
            machine.wait_until_succeeds(f"grep -qx 1 {chose}", timeout=timedelta(seconds=60))
    finally:
        desktop.as_account(f"rm -rf {folder}")


def selected_is(shown: Json, name: str, side: str | None = None, told: str | None = None) -> bool:
    """Say whether the row selected is the one named, saying the side and the keys given."""
    row = shown["selected"]
    return (
        row is not None
        and row["name"] == name
        and (side is None or row["side"] == side)
        and (told is None or told in row["told"])
    )


def gathered_is_in_sight(shown: Json, scale: str) -> None:
    """Hold Gathered, a.md and d.md in it, to its rows' order and its crumbs."""
    rows = shown["rows"]
    assert rows == ["a.md", "d.md", "Forget Them", "Copy the Paths"], rows
    # Its crumb is the place, and the program's ask and the place are said
    # whole, a folder's path giving way first where they don't fit.
    assert shown["path"][-1:] == ["Gathered"], shown["path"]
    whole = shown["pathWhole"]
    assert whole[0], f"at {scale} the program's ask is cut: {whole}"
    assert whole[-1], f"at {scale} the place is cut: {whole}"
    # A step gives way at whole names, never cut inside one, and its mark
    # keeps its own spacing, with no room left past the words before it
    # (VKCF3Z, 2026-11-05: "~\u2026e" and a wide gap after it).
    drawn = shown["pathDrawn"]
    assert not any(step["cut"] for step in drawn), f"at {scale} a name is cut: {drawn}"
    assert all(step["spare"] <= 1 for step in drawn), f"at {scale} a step leaves a gap: {drawn}"


def ran(desktop: Desktop, line: str) -> None:
    """Type a command on the launcher's line from normal, and wait until it's run."""
    desktop.normal()
    desktop.machine.send_chars(f":{line}")
    desktop.launcher_until(lambda shown: (shown["command"] or {}).get("line") == line)
    desktop.machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["command"] is None and shown["commandSaid"] == "")


def test_a_one_file_picker_asks_for_one_and_says_return_opens_it(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A picker that takes one file asks for one, shows no mark, and says Return opens it."""
    machine = desktop.machine
    folder = f"/home/{desktop.account}/one"
    chose = "/run/user/1000/portal-chose"
    desktop.as_account(
        f"mkdir -p {folder}/more && printf 'one\\n' > {folder}/a.md && touch {folder}/b.md"
    )
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            machine.succeed(f"rm -f {chose}")
            desktop.as_account(
                "systemd-run --user --collect --unit=choose-one"
                f" {desktop.values['choose_a_file']} open {folder}"
            )
            asked_in(desktop, "~/one", {"a.md"})
            shown = desktop.showing()
            assert shown["hint"] == "open which file?", shown
            assert "Open the Gathered" not in shown["rows"], shown["rows"]
            desktop.down_to("a.md")
            desktop.launcher_until(partial(selected_is, name="a.md", told="\u21b5 open"))
            desktop.shoot(f"file-picker-one-file-at-{scale.replace('.', '-')}")
            at_the_top(desktop, scale)
            # :q refuses the ask from any folder, Escape closing nothing.
            desktop.close_launcher()
            machine.wait_until_succeeds(f"grep -qx 1 {chose}", timeout=timedelta(seconds=60))
    finally:
        desktop.as_account(f"rm -rf {folder}")


def at_the_top(desktop: Desktop, scale: str) -> None:
    """Go up by - to /, a dialog's top, where - says it has nowhere to go; shoot it.

    Each folder's foot offers - up, and / offers none, since - there would leave the ask.
    The head still says where he is, "Open the Notes \u203a /", as it does a folder down.
    """
    machine = desktop.machine
    desktop.normal()
    while len(desktop.showing()["held"]) > 1:
        depth = len(desktop.showing()["held"])
        assert "- up" in desktop.showing()["selected"]["told"], desktop.showing()["selected"]
        machine.send_chars("-")

        def up(shown: Json, depth: int = depth) -> bool:
            """Say whether the picker went up one folder from this depth, a row selected."""
            return bool(len(shown["held"]) == depth - 1 and shown["selected"] is not None)

        desktop.launcher_until(up)
    assert "- up" not in desktop.showing()["selected"]["told"], desktop.showing()["selected"]
    machine.send_chars("-")
    desktop.launcher_until(
        lambda shown: (
            shown["saying"] == ["- not here"]
            and len(shown["held"]) == 1
            and shown["path"][-1:] == ["/"]
        )
    )
    desktop.shoot(f"file-picker-at-the-top-at-{scale.replace('.', '-')}")


def test_a_save_is_named_on_the_line_and_a_name_taken_asks_once(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A save's name is the line's words, in insert and in normal, and a name taken asks once."""
    machine = desktop.machine
    folder = f"/home/{desktop.account}/saving"
    chose = "/run/user/1000/portal-chose"
    desktop.as_account(f"mkdir -p {folder}/drafts && printf 'one\\n' > {folder}/a.md")
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            shot = scale.replace(".", "-")
            machine.succeed(f"rm -f {chose}")
            desktop.as_account(
                "systemd-run --user --collect --unit=choose-to-save"
                f" {desktop.values['choose_a_file']} save {folder} a.md"
            )
            asked_in(desktop, "~/saving", {"drafts"})
            # The caret before the extension, so typing renames it. The line is
            # the row Return stands on, above the rows, the name never said
            # again among them (VKCF3Z, #1812).
            desktop.launcher_until(
                lambda shown: (
                    shown["typed"] == "a.md"
                    and shown["caret"] == 1
                    and shown["mode"] == "insert"
                    and shown["modeMark"] == "INS"
                    and shown["current"] == -1
                    and (shown["selected"] or {}).get("name") == "a.md"
                    and "a.md" not in shown["rows"]
                )
            )
            desktop.shoot(f"file-picker-save-name-at-{shot}")
            desktop.normal()
            desktop.shoot(f"file-picker-save-name-normal-at-{shot}")
            machine.send_key("ret")
            desktop.launcher_until(
                lambda shown: (
                    shown["question"] == "Replace a.md? (y/n)"
                    and shown["takesAnswer"]
                    # Read whole, the path giving way, never cut.
                    and shown["questionWhole"]
                )
            )
            desktop.shoot(f"file-picker-save-replace-at-{shot}")
            machine.send_chars("n")
            desktop.launcher_until(lambda shown: shown["question"] == "" and shown["open"])
            # :q, from normal on the name, refuses the ask, Escape closing nothing.
            desktop.close_launcher()
            machine.wait_until_succeeds(f"grep -qx 1 {chose}", timeout=timedelta(seconds=60))
    finally:
        desktop.as_account(f"rm -rf {folder}")


def test_the_file_picker_gathers_files_and_says_what_return_opens(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """:arga gathers files across folders, the foot and mode word say how many, :args lists."""
    machine = desktop.machine
    folder = f"/home/{desktop.account}/gathering"
    chose = "/run/user/1000/portal-chose"
    desktop.as_account(
        f"mkdir -p {folder}/more && printf 'one\\n' > {folder}/a.md"
        f" && printf 'two\\n' > {folder}/b.md && touch {folder}/more/d.md"
    )
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            said = scale.replace(".", "-")
            machine.succeed(f"rm -f {chose}")
            desktop.as_account(
                "systemd-run --user --collect --unit=choose-to-gather"
                f" {desktop.values['choose_a_file']} open-several {folder}"
            )
            asked_in(desktop, "~/gathering", {"a.md", "b.md", "more"})
            assert desktop.showing()["hint"] == "open which files?", desktop.showing()
            desktop.down_to("a.md")
            desktop.launcher_until(partial(selected_is, name="a.md", told="\u21b5 open"))
            desktop.shoot(f"file-picker-nothing-gathered-at-{said}")
            # V takes rows, tinted, and the mode word and the foot say how many.
            desktop.normal()
            machine.send_chars("Vj")
            desktop.launcher_until(
                lambda shown: (
                    shown["mode"] == "visual"
                    and shown["modeWord"] == "2 chosen"
                    and len(shown["tinted"]) == len(["a.md", "b.md"])
                    and selected_is(shown, name="b.md", told="\u21b5 open 2")
                )
            )
            desktop.shoot(f"file-picker-visual-at-{said}")
            desktop.normal()
            desktop.down_to("a.md")
            ran(desktop, "arga")
            desktop.down_to("more")
            desktop.follow()
            desktop.launcher_until(lambda shown: shown["path"][-1:] == ["~/gathering/more"])
            desktop.down_to("d.md")
            ran(desktop, "arga")
            desktop.launcher_until(
                lambda shown: (
                    shown["placeSays"] == "2 gathered"
                    and selected_is(shown, name="d.md", told="\u21b5 open 2")
                )
            )
            desktop.shoot(f"file-picker-two-gathered-at-{said}")
            ran(desktop, "args")
            desktop.launcher_until(lambda shown: "Forget Them" in shown["rows"])
            # The cursor stands on the first gathered, and Return still answers both.
            desktop.launcher_until(partial(selected_is, name="a.md", told="\u21b5 open 2"))
            gathered_is_in_sight(desktop.showing(), scale)
            desktop.shoot(f"file-picker-inside-gathered-at-{said}")
            # Typed past the hint, the middle folder gives way to what's typed, and the
            # last step and what's typed stay whole (VKCF3Z, 2026-11-06).
            desktop.type_in("abcdefghijklmno")
            desktop.shoot(f"file-picker-typed-gathered-at-{said}")
            machine.send_key("ctrl-u")
            desktop.launcher_until(lambda shown: shown["typed"] == "")
            desktop.step_out()
            desktop.launcher_until(lambda shown: shown["rows"][-1:] == ["Copy the Path"])
            desktop.down_to("Copy the Path")
            desktop.shoot(f"file-picker-copy-last-at-{said}")
            # :q refuses the ask from any folder, Escape closing nothing.
            desktop.close_launcher()
            machine.wait_until_succeeds(f"grep -qx 1 {chose}", timeout=timedelta(seconds=60))
    finally:
        desktop.as_account(f"rm -rf {folder}")


# Each :sort the file picker takes and the files it lists so: a.md is the
# smallest and changed between, b.md the largest and oldest, c.md between
# and newest, and the folder first in each, :sort! turning the files round.
ORDERS = {
    "sort size": ["more", "b.md", "c.md", "a.md"],
    "sort!": ["more", "a.md", "c.md", "b.md"],
    "sort changed": ["more", "c.md", "a.md", "b.md"],
    "sort name": ["more", "a.md", "b.md", "c.md"],
}


def listed_from_the_top(shown: Json, listed: list[str]) -> bool:
    """Say whether the ordered folder is shown listing these first, on its first row."""
    return (
        shown["path"][-1:] == ["~/ordered"]
        and shown["rows"][: len(listed)] == listed
        and shown["selected"] is not None
        and shown["selected"]["name"] == listed[0]
    )


def test_the_file_picker_lists_a_folder_in_the_order_chosen(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """:sort name, changed and size list the folder so, :sort! turned round, folders first."""
    machine = desktop.machine
    folder = f"/home/{desktop.account}/ordered"
    chose = "/run/user/1000/portal-chose"
    desktop.as_account(
        f"mkdir -p {folder}/more && head -c 3 /dev/zero > {folder}/a.md"
        f" && head -c 3000 /dev/zero > {folder}/b.md && head -c 300 /dev/zero > {folder}/c.md"
        f" && touch -d '2 days ago' {folder}/b.md && touch -d '1 hour ago' {folder}/a.md"
    )
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            said = scale.replace(".", "-")
            machine.succeed(f"rm -f {chose}")
            desktop.as_account(
                "systemd-run --user --collect --unit=choose-in-order"
                f" {desktop.values['choose_a_file']} open {folder}"
            )
            asked_in(desktop, "~/ordered", {"a.md", "b.md", "c.md"})
            assert "Order" not in desktop.showing()["rows"], desktop.showing()["rows"]
            desktop.normal()
            machine.send_chars(":sort ")
            desktop.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "sort ")
            desktop.shoot(f"file-picker-sort-menu-at-{said}")
            machine.send_key("ctrl-u")
            machine.send_key("esc")
            desktop.launcher_until(lambda shown: shown["command"] is None)
            for line, listed in ORDERS.items():
                ran(desktop, line)
                desktop.launcher_until(partial(listed_from_the_top, listed=listed))
                shot = line.replace(" ", "-").replace("!", "-turned")
                desktop.shoot(f"file-picker-{shot}-at-{said}")
            desktop.close_launcher()
            machine.wait_until_succeeds(f"grep -qx 1 {chose}", timeout=timedelta(seconds=60))
    finally:
        desktop.as_account(f"rm -rf {folder}")


# A session sits under the one orchestrating it, a step in, its glyph where its orchestrator's
# name starts, with no line, box or fold; typed, every row is at no depth, whom it's under
# leading its note; and an orchestrator says what its whole orchestration spent today
# (QEST-20261026-15J7N3's agents-tree-look.md). The orchestrator listens while an owner under it
# runs a builder that works, so the orchestration stands with those working: the owner, its
# builder, and one paused beside the builder. The two others under the orchestrator only listen,
# so they fold into one row after its own (VKCF3Z, 2026-11-06), while the owner's one paused
# stays a row, since folding one saves none.
ORCHESTRATION = [
    # address, title, whom it's under, how it stands
    ("TESTQA", "A quest of its own & whole", None, "listening"),
    ("TESTQB", "The Other One Under It", "TESTQA", "listening"),
    ("TESTQC", "An Owner Under It", "TESTQA", "listening"),
    ("TESTQD", "A Builder Under the Owner", "TESTQC", "working"),
    ("TESTQE", "One Put Down Under It", "TESTQC", "paused"),
    ("TESTQF", "One More Under It", "TESTQA", "listening"),
]
# The fold's row, by what it says, and the sessions it holds.
FOLD = "2 listening"
FOLDED = ["The Other One Under It", "One More Under It"]
TREE = {"TESTQA": 0, "TESTQC": 1, "TESTQD": 2, "TESTQE": 2, FOLD: 1}
SPENT = {
    "v": 2,
    "sessions": {"01a1face-test": "$3.10", "01a1face-qd": "$5.10"},
    # How the builder's day went, its hours from midnight on the day's largest one.
    "sessionsToday": {
        "01a1face-qd": {
            "graph": {
                "shape": "spark",
                "values": [0, 0, 0, 0, 0, 0, 0, 0, 0.1, 0.4, 0.2, 0.7, 1, 0.5, 0.3],
            },
            "rebuilt": "$1.20",
        }
    },
    "helpers": {},
    "families": {
        "QEST-20261022-TESTQA": {"all": "$14.20", "own": "$3.10", "under": "$11.10"},
        "QEST-20261022-LAMPLT": {"all": "$2.40", "own": "$2.40", "under": "$0.00"},
    },
}
# Orchestrators no session runs, as the harness lists them, each with when it was last live:
# they stand dimmed at the tree's end, the one live last first, and only one that spent today
# says what all of it spent (VKCF3Z, 2026-11-07 [9a4b1dd0]).
CLOSED = [
    # address, its short name, its quest's title, when it was last live
    ("BEACON", "Beacon", "Light the beacon", "2026-11-06T22:00:00Z"),
    ("LAMPLT", "Lamplight", "Keep the lamp lit", "2026-11-07T04:00:00Z"),
]
# What the owner under the orchestrator landed, newest first, as the harness keeps it: a change
# with its page, a plan item ticked alone, which Landed leaves out, and a change saying what to
# try (VKCF3Z's design: Landed in Now, tier 2).
STOPPED = "Stop Waits for a Second Return"
CLOSING = "A Session Closes From the Launcher"
LANDINGS = [
    {"at": "2026-11-07T11:31:00Z", "ref": "core.nix#2140", "what": STOPPED,
     "url": "https://example.org/core.nix/pull/2140"},
    {"at": "2026-11-07T11:25:00Z", "ref": "PLAN-20261022-TESTQC#2", "what": "Ticked alone"},
    {"at": "2026-11-07T11:19:00Z", "ref": "core.nix#2138", "what": CLOSING,
     "person": "Press dd on a session to close it"},
]  # fmt: skip
# How full the builder's context is and what its turns took, as the harness's usage keeps them
# by gen_ai's names; today's counts show only on the day they're of.
FULL = {
    "gen_ai.response.model": "claude-opus-5-5",
    "last": {"gen_ai.usage.output_tokens": 4100},
    "context": {"used": 842000, "limit": 1000000},
    "turns": {"last": {"duration_ms": 38000, "first_token_ms": 900}},
}
# What the builder's record says of its work, as pi says it: its step, its quest, and what it
# works on, the quest's title and then a summary as long as every real session's.
BUILDER = {
    "stepDetail": {"kind": "tool", "name": "edit", "detail": "/home/jitsusama/tree/Drawer.qml"},
    "questInfo": {"id": "QEST-20261022-TESTQD", "title": "A Builder Under the Owner",
                  "short": "TESTQD", "status": "active"},
    "worksOn": "A Builder Under the Owner: A builder under the owner's plan, which keeps the"
    " launcher's agents quiet, told and easy to read: every session says how it stands, what it"
    " does, what it spent and what it needs from Joel, from the node, with nothing guessed and"
    " nothing said twice.",
}  # fmt: skip
# A quest nothing orchestrates and no session has, whose step in offers to orchestrate it.
UNSTARTED = "Chart the harbour"
WORKFLOW = "~/.local/state/pi/agentic-harness.pi/quest-workflow"


def window_of(address: str, title: str) -> str:
    """Give the title of a session's window, as its label: its quest's name and its folder."""
    return f"\u03c0 - {title} - {address.lower()}-tree"


def orchestrated(desktop: Desktop) -> None:
    """Start the orchestration's sessions, each in a window of its own, saying whom it's under."""
    desktop.pi_session()
    for address, title, under, stands in ORCHESTRATION:
        if address != "TESTQA":
            desktop.pi_window(address, window_of(address, title))
        desktop.presence_says(
            {
                "session": (
                    "01a1face-test" if address == "TESTQA" else f"01a1face-{address[-2:].lower()}"
                ),
                "addresses": [address],
                "busy": stands == "working",
                "state": "busy" if stands == "working" else stands,
                "owns": [f"QEST-20261022-{address}"],
                "orchestratedBy": under,
                # The orchestrator's record names its repository, which its drawer's eyebrow
                # says as it's written.
                **({"repo": "core.nix"} if address == "TESTQA" else {}),
                **({"landings": LANDINGS} if address == "TESTQC" else {}),
                **({"usage": FULL, **BUILDER} if address == "TESTQD" else {}),
            },
            address,
        )
    desktop.as_account(
        f"mkdir -p $XDG_RUNTIME_DIR/spend-watch && echo '{json.dumps(SPENT)}'"
        " > $XDG_RUNTIME_DIR/spend-watch/.c"
        " && mv $XDG_RUNTIME_DIR/spend-watch/.c $XDG_RUNTIME_DIR/spend-watch/collection.json"
    )
    closed_listed(desktop)


def closed_listed(desktop: Desktop) -> None:
    """List the closed orchestrators, as the harness does, and a quest nothing has started."""
    listed = {
        "orchestrators": [
            {"quest": f"QEST-20261022-{address}", "address": address, "short": short,
             "title": title}
            for address, short, title, _ in CLOSED
        ]
    }  # fmt: skip
    quests = [
        desktop_session.listed_quest(
            f"QEST-20261022-{address}",
            title,
            priority="active",
            resume={"id": f"01a1-{address.lower()}", "cwd": "/home/jitsusama", "at": at},
        )
        for address, _, title, at in CLOSED
    ]
    unstarted = desktop_session.listed_quest("QEST-20261022-HARBOR", UNSTARTED, priority="queued")
    desktop_session.quests_listed(desktop, [*quests, unstarted])
    desktop.as_account(
        f"echo '{json.dumps(listed)}' > {WORKFLOW}/.o"
        f" && mv {WORKFLOW}/.o {WORKFLOW}/orchestrators.json"
    )


def unorchestrated(desktop: Desktop) -> None:
    """Close the sessions the orchestration started, and forget what they spent."""
    for address, title, _, _ in ORCHESTRATION[1:]:
        window = desktop.window_titled(window_of(address, title))
        desktop.as_account(f"niri msg action close-window --id {window['id']}")
        desktop.as_account(f"rm -f {desktop_session.PI_PRESENCE}/{address.lower()}.json")
    desktop.as_account(
        "rm -f $XDG_RUNTIME_DIR/spend-watch/collection.json"
        f" {WORKFLOW}/orchestrators.json {WORKFLOW}/quests.json {WORKFLOW}/ended.json"
    )
    desktop.presence_says({"addresses": ["TESTQA"], "busy": True})


def address_of(row: Json) -> str:
    """Say which of the orchestration's sessions a drawn row is, by its title.

    A session's row says its title, how it stands and what it spent; its address is its
    drawer's. The fold's row is named by what it says.
    """
    if FOLD in row["reads"]:
        return FOLD
    return next(address for address, title, _, _ in ORCHESTRATION if title in row["reads"])


def in_orchestration(row: Json) -> bool:
    """Say whether a drawn row is one of the orchestration's sessions, or its fold."""
    reads = row["reads"]
    return FOLD in reads or any(title in reads for _, title, _, _ in ORCHESTRATION)


def orchestration_drawn(desktop: Desktop) -> list[Json]:
    """Give the orchestration's rows as they're drawn, once two reads in a row agree."""
    was = None
    for _ in range(20):
        laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
        now = [row for row in laid["rows"] if in_orchestration(row)]
        if now == was:
            return now
        was = now
    return cast("list[Json]", was)


def side_gives_way_whole(row: Json, title: str, said: str, scale: str) -> None:
    """Hold a row's side to whole items, its name whole while the side says more than one."""
    name = next(word for word in row["words"] if word["text"] == title)
    # What a row says at its right is the word drawn furthest right.
    side = max(row["words"], key=lambda word: word["x"])
    assert not side["cut"], f"at {scale} {title}'s side is cut: {side}"
    assert said in side["text"], f"at {scale} {title}'s side dropped {said!r}: {side}"
    assert not name["cut"] or " \u00b7 " not in side["text"], (
        f"at {scale} {title}'s name is cut while its side says {side['text']!r}"
    )


def into_agents(desktop: Desktop) -> None:
    """Open the launcher into Agents, once it lists every session of the orchestration."""
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("agents")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Agents")
    desktop.machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["agents"]
            and sum(in_orchestration(thing) for thing in shown["things"]) == len(TREE)
        )
    )


def orchestrator_says_whose_spend_is_whose(desktop: Desktop) -> None:
    """Inside the orchestrator, what all of it spent, then its own and its sessions'."""
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("testqa", selecting="A quest of its own & whole")
    desktop.follow()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Go To")
    for name, side in [
        ("Spent Today", "$14.20 in all"),
        ("Its Own", "$3.10"),
        ("Its Sessions", "$11.10"),
    ]:
        desktop.down_to(name)
        assert desktop.showing()["selected"]["side"] == side, desktop.showing()["selected"]
    desktop.close_launcher()


def orchestrators_drawer_names_its_repo(desktop: Desktop, title: str, shot: str) -> None:
    """Shoot the orchestrator's drawer, its eyebrow naming its repository as written.

    The eyebrow says its address, its repository and when it started, set as they're written,
    never in capitals, since they're names from the world (VKCF3Z, 2026-11-06). Short of room,
    as the drawer is at its least width, it gives way by whole items from its end, never cut
    mid-fact (VKCF3Z, 2026-11-07).
    """
    desktop.down_to(title)
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["shown"]
            and shown["drawer"]["eyebrow"].startswith("TESTQA \u00b7 core.nix \u00b7 ")
            and shown["drawer"]["eyebrowNames"]
        )
    )
    drawer = desktop.showing()["drawer"]
    whole = drawer["eyebrow"]
    assert drawer["eyebrowDrawn"] in {whole, "TESTQA \u00b7 core.nix", "TESTQA"}, (
        f"at {shot} the eyebrow is cut mid-fact: {drawer}"
    )
    desktop.shoot(f"launcher-agents-drawer-at-{shot}")


def figures_in_the_drawer(desktop: Desktop, title: str, shot: str) -> None:
    """Shoot a session past 80% full, its drawer giving its figures by gen_ai's names."""
    desktop.down_to(title)
    # Its step leads, its figures follow what it spent, then its quest and machine, and what
    # its quest is about, long, comes last, so the first screen holds the rest (Joel and
    # VKCF3Z, 2026-11-07).
    first = ["Doing", "Spent Today", "Context", "Last Turn", "Model"]
    later = ["Context Rebuilt", "Quest", "Machine", "Working On"]
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["shown"]
            and shown["drawer"]["facts"][: len(first)] == first
            and shown["drawer"]["facts"][-len(later) :] == later
        )
    )
    side = desktop.showing()["selected"]["side"]
    assert side.startswith("84% full \u00b7 working"), f"at {shot}: {side}"
    # Its note says what its quest is about, never its title, the row's own name, again.
    note = desktop.showing()["selected"]["note"]
    assert note.startswith("A builder under"), f"at {shot}: {note}"
    # Context draws Meter's stroke after its value, at Joel's scales both.
    drawer = desktop.showing()["drawer"]
    drawn = dict(zip(drawer["drawnFacts"], drawer["figures"], strict=True))
    assert drawn["Context"] == "stroke", f"at {shot}: {drawer}"
    # Its spend today carries its hours as Graph's spark.
    assert drawn["Spent Today"] == "spark", f"at {shot}: {drawer}"
    assert drawer["laidOneWay"], f"at {shot} its facts mix beside and under: {drawer}"
    # How it stands is its row's, so Doing says its step alone, and its quest, active, its id.
    assert drawer["values"][0] == "edit Drawer.qml", f"at {shot}: {drawer}"
    assert drawer["values"][-3] == "QEST-20261022-TESTQD", f"at {shot}: {drawer}"
    # What it's doing, its spend and how full it is are on its first screen, even this
    # short one's, before any it says are more.
    said = str(drawer.get("moreSays") or "")
    hidden = int(said.split(maxsplit=1)[0]) if said else 0
    led = first[: first.index("Context") + 1]
    assert len(drawer["facts"]) - hidden >= len(led), f"at {shot} it hides {said}: {drawer}"
    desktop.shoot(f"launcher-agents-figures-at-{shot}")


def returns_said_in_the_foot(desktop: Desktop, titles: dict[str, str], shot: str) -> None:
    """Shoot the foot on a working, a listening and a paused row, then one's acts.

    Return does each state's harmless act, which the foot says before it's pressed
    (VKCF3Z, 2026-11-06): talk to one listening, resume one paused. A step in holds every
    act in its fixed order, whatever Return does. From the find, a Return that changes
    something only lands first, and its foot names the act it leads to all the same
    (VKCF3Z, 2026-11-07), so the paused row's foot is kept there too.
    """
    desktop.down_to(titles["TESTQE"])
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("told", [])[:1] == ["\u21b5 resume"]
    )
    desktop.shoot(f"launcher-agents-returns-paused-found-at-{shot}")
    desktop.normal()
    feet = [("TESTQD", "working", "\u21b5 go to"), ("TESTQC", "listening", "\u21b5 talk")]
    for address, stands, said in [*feet, ("TESTQE", "paused", "\u21b5 resume")]:
        desktop.down_to(titles[address])

        def foot_says(shown: Json, said: str = said) -> bool:
            """Say whether the selected row's foot says Return's act."""
            told = (shown["selected"] or {}).get("told", [])
            return any(str(one).startswith(said) for one in told)

        desktop.launcher_until(foot_says)
        desktop.shoot(f"launcher-agents-returns-{stands}-at-{shot}")
    desktop.down_to(titles["TESTQC"])
    desktop.follow()
    desktop.launcher_until(lambda shown: "Talk" in shown["rows"][:2])
    desktop.shoot(f"launcher-agents-acts-at-{shot}")
    desktop.step_out()


def fold_steps_in(desktop: Desktop, shot: str) -> None:
    """Shoot the fold's row, then what Return steps in to: each session it holds."""
    desktop.down_to(FOLD)
    desktop.shoot(f"launcher-agents-fold-at-{shot}")
    desktop.machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"][-1:] == [FOLD] and all(title in shown["rows"] for title in FOLDED)
        )
    )
    desktop.shoot(f"launcher-agents-fold-inside-at-{shot}")
    desktop.step_out()


def closed_stand_last(desktop: Desktop, shot: str) -> None:
    """Shoot the closed orchestrators at the tree's end, one on the cursor, one at rest.

    Each is dimmed, says it's closed, and what all of it spent only where it spent today;
    Return reopens it.
    """
    desktop.down_to("Lamplight")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("side") == "closed \u00b7 $2.40 in all"
            and shown["selected"]["hue"] == "muted"
        )
    )
    rows = desktop.showing()["rows"]
    assert rows.index("Lamplight") + 1 == rows.index("Beacon"), f"at {shot}: {rows}"
    desktop.shoot(f"launcher-agents-closed-at-{shot}")


def close_asks_what_stays(desktop: Desktop, title: str, address: str, shot: str) -> None:
    """Shoot a session's Close on the cursor, then its ask, which says what stays.

    Close ends the session, so Return puts it on the command line and a fresh Return asks,
    naming the session by its address and what of it stays (VKCF3Z, 2026-11-06); n leaves
    it running.
    """
    desktop.down_to(title)
    desktop.follow()
    desktop.launcher_until(lambda shown: "Close" in shown["rows"])
    desktop.down_to("Close")
    desktop.shoot(f"launcher-agents-close-at-{shot}")
    # The second Return waits out the first's, which the launcher takes for the same press
    # doubled while it's fresh, and so runs nothing.
    desktop.line("close")
    asked = f"Close {address}? Its folder stays. (y/n)"
    with desktop.told("ask ready"), desktop.told(f"asked {asked}"):
        desktop.machine.send_key("ret")
    # The ask takes the whole head, so what it closes is read, its path given way.
    shown = desktop.showing()
    drawn = shown["questionShown"]
    assert drawn.startswith(f"Close {address}?"), f"at {shot} the ask lost its subject: {drawn}"
    assert drawn.endswith("(y/n)"), f"at {shot} the ask lost its keys: {drawn}"
    assert shown["questionWhole"], f"at {shot} the ask was cut: {drawn}"
    desktop.shoot(f"launcher-agents-close-asks-at-{shot}")
    with desktop.told("answered no"):
        desktop.machine.send_chars("n")
    desktop.launcher_until(lambda shown: shown["question"] == "")
    desktop.step_out()


def landed_lists_what_sessions_landed(desktop: Desktop, shot: str) -> None:
    """Shoot Landed, its count at its side, then inside it, a landing's drawer out.

    Each says what landed as its name and its ref in the mono at its right; its drawer says
    who landed it and when, and what to try, a sentence the row has no room for. Nothing is
    read, so the walk can come back to it.
    """
    desktop.down_to("Landed")
    assert desktop.showing()["selected"]["side"] == "2", f"at {shot}: {desktop.showing()}"
    desktop.shoot(f"launcher-agents-landed-at-{shot}")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"] == [STOPPED, CLOSING])
    desktop.down_to(CLOSING)
    selected = desktop.showing()["selected"]
    assert selected["side"] == "core.nix#2138", f"at {shot}: {selected}"
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True and shown["drawer"]["body"] == LANDINGS[2]["person"]
        )
    )
    desktop.shoot(f"launcher-agents-landed-in-at-{shot}")
    desktop.step_out()


def after_the_sessions(desktop: Desktop, shot: str) -> None:
    """Shoot what Agents holds after its sessions: the closed, Landed, Recent and Start."""
    closed_stand_last(desktop, shot)
    landed_lists_what_sessions_landed(desktop, shot)
    desktop.down_to("Recent")
    desktop.shoot(f"launcher-agents-recent-at-{shot}")
    a_quest_offers_to_orchestrate(desktop, shot)


def a_quest_offers_to_orchestrate(desktop: Desktop, shot: str) -> None:
    """Shoot a quest's step in from Start a Session: Start Here, then Orchestrate.

    Normal's foot keeps : and Space ?, since the mode's word holds room only for what it
    says in normal, never for visual's count (VKCF3Z, 2026-11-07: held for "N chosen", it
    cost normal its :); visual says its count where normal's word stood.
    """
    desktop.down_to("Start a Session")
    desktop.machine.send_key("ret")
    desktop.launcher_until(lambda shown: UNSTARTED in shown["rows"])
    desktop.down_to(UNSTARTED)
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Start Here", "Orchestrate"])
    hints = desktop.showing()["hints"]
    assert {": commands", "Space ? keys"} <= set(hints), f"at {shot}: {hints}"
    desktop.shoot(f"launcher-agents-start-quest-at-{shot}")
    desktop.machine.send_chars("v")
    desktop.launcher_until(lambda shown: shown["modeWord"] == "1 chosen")
    desktop.shoot(f"launcher-agents-start-quest-visual-at-{shot}")
    desktop.machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["modeWord"] == "")
    desktop.step_out()
    desktop.step_out()


def glyphs_stand_on_names(rows: list[Json], titles: dict[str, str], scale: str) -> None:
    """Hold each row's glyph where the name above it starts, the first word a row draws."""
    words = {address_of(row): row["words"] for row in rows}
    under = [("TESTQC", "TESTQA"), ("TESTQD", "TESTQC"), ("TESTQE", "TESTQC"), (FOLD, "TESTQA")]
    for one, above in under:
        glyph = words[one][0]["starts"]
        name = next(w["starts"] for w in words[above] if w["text"] == titles[above])
        assert abs(glyph - name) <= 1, (
            f"at {scale} {one}'s glyph is at {glyph}, not on {above}'s name at {name}"
        )


def test_a_session_sits_a_step_in_under_its_orchestrator_which_says_what_all_of_it_spent(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A session sits a step in under its orchestrator, its glyph on its name; flat once typed."""
    orchestrated(desktop)
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            into_agents(desktop)
            rows = orchestration_drawn(desktop)
            order = [address_of(row) for row in rows]
            assert order == list(TREE), f"at {scale} the orchestration reads {order}"
            depths = {address_of(row): row["depth"] for row in rows}
            assert depths == TREE, f"at {scale} the steps in are {depths}"
            titles = {address: title for address, title, _, _ in ORCHESTRATION}
            titles[FOLD] = FOLD
            glyphs_stand_on_names(rows, titles, scale)
            # Short of room, a side gives way whole, the state first, before a name gives up
            # a letter, and what it cost stays, never slid from its left. Beside a tier 2
            # figure, as the builder's 84% full, how it stands outlasts the money instead.
            costs = {"TESTQA": "$14.20 in all", "TESTQD": "84% full \u00b7 working"}
            for row in rows:
                one = address_of(row)
                side_gives_way_whole(row, titles[one], costs.get(one, ""), scale)
            # The orchestrator says what all of it spent; one under it, its own. One idle
            # is listening, never waiting.
            reads = {address_of(row): row["reads"] for row in rows}
            assert "$14.20 in all" in reads["TESTQA"], reads
            assert "$5.10" in reads["TESTQD"], reads
            assert "in all" not in reads["TESTQD"], reads
            assert "listening" in reads["TESTQC"], reads
            assert "paused" in reads["TESTQE"], reads
            # And its glyph is muted, never the live rows' colour (#2052), its name in text.
            inks = {address_of(row): row["words"][0]["ink"] for row in rows}
            assert inks["TESTQE"] != inks["TESTQC"], f"at {scale} paused looks live: {inks}"
            assert "waiting" not in " ".join(reads.values()), reads
            shot = scale.replace(".", "-")
            desktop.shoot(f"launcher-agents-tree-at-{shot}")
            orchestrators_drawer_names_its_repo(desktop, titles["TESTQA"], shot)
            figures_in_the_drawer(desktop, titles["TESTQD"], shot)
            returns_said_in_the_foot(desktop, titles, shot)
            fold_steps_in(desktop, shot)
            close_asks_what_stays(desktop, titles["TESTQC"], "TESTQC", shot)
            after_the_sessions(desktop, shot)
            desktop.close_launcher()
        orchestrator_says_whose_spend_is_whose(desktop)
        # Typed, the builder ranks flat, at no depth, whom it's under leading its note,
        # which the foot says for the row selected.
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("testqd", selecting="A Builder Under the Owner")
        # Found from the top, it reads as the thing it is, its window, beside its kind.
        laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
        row = next(row for row in laid["rows"] if row["selected"])
        assert row["depth"] == 0, row
        note = desktop.showing()["selected"]["note"]
        assert note.startswith("under TESTQC"), note
        # And it leads the words drawn after its name, since the note is only in the foot.
        drawn = [word["text"] for word in row["words"]]
        assert any(text.startswith("under TESTQC") for text in drawn), drawn
        desktop.shoot("launcher-agents-typed-flat")
        desktop.close_launcher()
    finally:
        unorchestrated(desktop)


def test_a_meter_stands_over_dictation_rather_than_on_it_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot dictation, a meter above it, and a meter alone at its least and most.

    Both are one frame at the bottom centre, the frame's inset from the ink on every side
    (docs/design.md, The Ladder). A meter that shows while dictation listens stands the space
    between frames above it rather than over it, and a shorter figure keeps its room after it,
    so the bar's gap to it never grows. Each is kept for a person to measure.
    """
    machine = desktop.machine
    dictation = desktop.in_session("niri msg layers | grep -q dictation")
    volume = desktop.in_session("niri msg layers | grep -q 'Namespace: \"volume\"'")
    was = desktop.as_account("wpctl get-volume @DEFAULT_AUDIO_SINK@").split()[1]
    # A change is heard only once the shell has bound the speaker, which it says.
    desktop.reported("hears volume ", bool, "", timeout=timedelta(seconds=60))

    # Where on the screen, in fractions of its height, dictation's frame and
    # a meter above it fall at both scales, the middle half across.
    under, over = (0.87, 0.94), (0.78, 0.84)

    def band(rows: tuple[float, float]) -> bytes:
        """Give the screen's middle half across between two fractions of its height."""
        screen = desktop.screen_now()
        width, height = screen.size
        box = (width // 4, int(height * rows[0]), 3 * width // 4, int(height * rows[1]))
        return screen.crop(box).tobytes()

    def shot_once_in(name: str, rows: tuple[float, float], before: bytes) -> None:
        """Shoot once the band shows something it didn't, or after three seconds.

        While dictation listens, its clock and wave move, so the screen never holds still,
        and a meter goes again in a second and a half; and niri can show dictation's first
        frame a while after it's drawn. So the picture is taken once what it's of is on the
        screen. One that never comes, as a meter hidden under dictation, is kept as it is.
        """
        deadline = time.monotonic() + 3
        while band(rows) == before and time.monotonic() < deadline:
            pass
        if desktop.values["shots"]:
            machine.screenshot(name)

    def metered(level: str, name: str) -> None:
        machine.wait_until_fails(volume, timeout=timedelta(seconds=10))
        with desktop.told("drawn volume"):
            desktop.as_account(f"wpctl set-volume @DEFAULT_AUDIO_SINK@ {level}")
        desktop.shoot(name)

    try:
        for scale in at_each_scale:
            named = scale.replace(".", "-")
            at_scale(desktop, scale)
            machine.wait_until_fails(dictation, timeout=timedelta(seconds=15))
            since = desktop.mark()
            before = band(under)
            with desktop.told("drawn dictation"):
                desktop.as_account("dictation toggle")
            desktop.reported("says dictation listening", lambda rest: rest == "", since)
            shot_once_in(f"dictation-listening-at-{named}", under, before)
            machine.wait_until_fails(volume, timeout=timedelta(seconds=10))
            before = band(over)
            with desktop.told("drawn volume"):
                desktop.as_account("wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.5")
            shot_once_in(f"meter-over-dictation-at-{named}", over, before)
            # Stopped, the stand-in types a moment and says it heard nothing.
            desktop.as_account("dictation toggle")
            desktop.reported("says dictation heard nothing", lambda rest: rest == "", since)
            machine.wait_until_fails(dictation, timeout=timedelta(seconds=10))
            metered("0.05", f"meter-least-at-{named}")
            metered("1", f"meter-most-at-{named}")
    finally:
        desktop.as_account(f"wpctl set-volume @DEFAULT_AUDIO_SINK@ {was}")


def spend_line(key: str, name: str, side: str, note: str = "", role: str = "text") -> Json:
    """Give one of AI spend's rows as spend-watch writes it (its collection.rs's Line)."""
    line: Json = {"key": key, "name": name, "side": side, "copy": f"{name}: {side}", "role": role}
    if note:
        line["note"] = note
    # A budget's row and a window's name what opens their drawer in budgets.json.
    if key.startswith("budget:"):
        line["budget"] = key
    return line


def marked(line: Json, mark: str, rows: list[Json] | None = None) -> Json:
    """Give a row the theme's mark its role colours, and what it holds."""
    line = {**line, "mark": mark}
    if rows:
        line["rows"] = rows
    return line


# How things stand with three budgets, as spend-watch writes it: today, with its drawer and the
# day's cap behind it, opening its window's drawer; then Budgets, the month's whole in billed
# spend, its mark its nearest budget's colour; behind it today's whole, then each budget nearest
# its allowance first, the plan stepping into its two windows.
BUDGETED = {
    "v": 2,
    "sessions": {},
    "helpers": {},
    "families": {},
    "now": [
        marked(
            spend_line("today", "Today", "$412")
            | {
                "drawn": {
                    "eyebrow": "Sat 7 Nov \u00b7 day",
                    "facts": [
                        {"id": "ends", "value": "$520 \u00b7 likely $470 to $610"},
                        {"id": "from", "value": "the bill \u00b7 pi's logs where it hasn't come"}
                        | {"words": True},
                        {"id": "rebuilding", "value": "$25 \u00b7 6%"},
                        {"id": "most-quest", "value": "Build the shell \u00b7 31%"}
                        | {"words": True},
                        {"id": "most-model", "value": "claude-opus-5-5 \u00b7 all"},
                    ],
                }
            },
            "date",
            rows=[
                marked(
                    spend_line("cap:premium", "Premium Day", "83%", "$250 spent", "warning")
                    | {"budget": "budget:proxy:premium"},
                    "clock",
                )
            ],
        ),
        marked(
            spend_line("budgets", "Budgets", "$21,564 this month", "billed spend", "warning"),
            "agent",
            rows=[
                marked(
                    spend_line(
                        "budgets/all",
                        "All AI Spend",
                        "$412 today",
                        "billed spend \u00b7 $3,163 in 7 days",
                    ),
                    "agent",
                ),
                marked(
                    spend_line(
                        "budget:claude", "Claude Pro", "85% of 5 h", "your own plan", "warning"
                    ),
                    "agent",
                    rows=[
                        marked(
                            spend_line(
                                "budget:claude:five-hours",
                                "Five Hours",
                                "85%",
                                role="warning",
                            ),
                            "clock",
                        ),
                        marked(
                            spend_line("budget:claude:week", "Week", "40%"),
                            "clock",
                        ),
                    ],
                ),
                marked(
                    spend_line(
                        "budget:proxy",
                        "Your AI proxy token",
                        "$250 of $300",
                        "the default \u00b7 the employer's money",
                        "warning",
                    ),
                    "agent",
                    rows=[
                        marked(
                            spend_line(
                                "budget:proxy:premium",
                                "Premium Day",
                                "$250 of $300",
                                role="warning",
                            ),
                            "clock",
                        ),
                        marked(
                            spend_line("budget:proxy:weekly", "Week", "$27,570 of $40,000"),
                            "clock",
                        ),
                    ],
                ),
                marked(
                    spend_line(
                        "budget:delta", "Delta team token", "$310 this month", "a team's token"
                    ),
                    "agent",
                ),
            ],
        ),
    ],
    "quiet": [spend_line("today", "Today", "spending")],
}


def allowance(
    name: str, label: str, standing: str, nearest: float, warn: float, **said: str
) -> Json:
    """Give one of a budget's windows as budgets.json has it, its facts in words."""
    unit = "dollars" if standing.startswith("$") else "share"
    window: Json = {"name": name, "label": label, "unit": unit, "warn": warn}
    window |= {"standing": standing, "nearest": nearest}
    keys = ("window", "warns", "resets", "ends", "changed", "read", "state")
    return window | {"said": {key: said.get(key, "") for key in keys}}


# How each of BUDGETED's budgets stands, as spend-watch's budgets.json says it, for their
# drawers: a plan's share windows, one past where it warns and one not read yet; the employer's
# money of two dollar windows, the nearer past where it warns; and a team's token of none.
BUDGETS = {
    "v": 1,
    "default": "proxy",
    "sessions": {},
    "budgets": [
        {
            "name": "claude",
            "label": "Claude Pro",
            "words": "your own plan",
            "providers": ["claude-code"],
            "standing": "85% of 5 h",
            "allowances": [
                allowance(
                    "five-hours",
                    "Five Hours",
                    "85% of 5 h",
                    0.85,
                    warn=0.8,
                    window="5 hours",
                    warns="at 80%",
                    resets="16:20",
                    read="Claude at 14:20",
                ),
                allowance(
                    "week",
                    "Week",
                    "40% of the week",
                    0.4,
                    warn=0.8,
                    window="a week",
                    warns="at 80%",
                    resets="Mon 09:00",
                    read="Claude at 14:20",
                ),
            ],
            "said": {
                "spent": "",
                "worth": "$84 today \u00b7 $1,120 this month",
                "reading": "reading fine",
            },
        },
        {
            "name": "proxy",
            "label": "Your AI proxy token",
            "words": "the employer's money",
            "providers": [],
            "standing": "$250 of $300",
            "allowances": [
                allowance(
                    "premium",
                    "Premium Day",
                    "$250 of $300",
                    0.833,
                    warn=0.5,
                    window="a day",
                    warns="at $150",
                    resets="05:00 tomorrow",
                    read="the AI proxy at 14:05",
                ),
                allowance(
                    "weekly",
                    "Week",
                    "$27,570 of $40,000",
                    0.689,
                    warn=0.9,
                    window="a week",
                    warns="at $36,000",
                    read="the AI proxy at 14:05",
                ),
            ],
            "said": {
                "spent": "$402 today \u00b7 $3,060 in 7 days \u00b7 $20,144 this month",
                "worth": "",
                "reading": "reading fine",
            },
        },
        {
            "name": "delta",
            "label": "Delta team token",
            "words": "a team's token",
            "providers": [],
            "standing": "$310 this month",
            "allowances": [],
            "said": {"spent": "$10 today \u00b7 $310 this month", "worth": "", "reading": ""},
        },
    ],
}


def written(desktop: Desktop, name: str, what: Json) -> None:
    """Write one of spend-watch's files whole, as its daemon does, renaming it into place."""
    # Its words have apostrophes, so the JSON is quoted for the shell whole.
    desktop.as_account(
        f"mkdir -p $XDG_RUNTIME_DIR/spend-watch && echo {shlex.quote(json.dumps(what))}"
        f" > $XDG_RUNTIME_DIR/spend-watch/.{name}"
        f" && mv $XDG_RUNTIME_DIR/spend-watch/.{name} $XDG_RUNTIME_DIR/spend-watch/{name}"
    )


def drawer_shot(desktop: Desktop, title: str, picture: str) -> list[str]:
    """Shoot the drawer of the row selected once it's out with its title, and say its facts."""
    desktop.launcher_until(
        lambda shown: shown["drawer"]["out"] is True and shown["drawer"]["title"] == title
    )
    desktop.shoot(picture)
    return cast("list[str]", desktop.showing()["drawer"]["facts"])


def test_the_budgets_stand_behind_one_row_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot AI spend's Budgets row, the budgets behind it, and their windows' drawers.

    One row stands for every budget, the month's whole beside it; behind it the whole first,
    then each budget nearest its allowance first, each standing in its own unit, and a budget
    steps into its windows (docs/design.md; VKCF3Z, 2026-11-06). A budget's drawer and a
    window's say what budgets.json holds of each (VKCF3Z, 2026-11-07): a plan's, the employer's
    money's, a share window's and a dollar window past where it warns. A running period's
    drawer says its days and what its cuts went to most, and a cap's is its window's. Each is
    kept for the design's look at Joel's scales.
    """
    written(desktop, "budgets.json", BUDGETS)
    written(desktop, "collection.json", BUDGETED)
    try:
        for scale in at_each_scale:
            named = scale.replace(".", "-")
            at_scale(desktop, scale)
            desktop.as_account("qs ipc call launcher collection spend")
            desktop.launcher_ready()
            desktop.launcher_until(lambda shown: "Budgets" in shown["rows"])
            desktop.down_to("Today")
            facts = drawer_shot(desktop, "Today", f"launcher-spend-today-at-{named}")
            assert facts == [
                "Ends Near",
                "Read From",
                "Rebuilding Context",
                "Most by Quest",
                "Most by Model",
            ], facts
            desktop.step_in()
            desktop.down_to("Premium Day")
            facts = drawer_shot(desktop, "Premium Day", f"launcher-spend-cap-at-{named}")
            assert facts == ["Standing", "Resets", "Read By", "Warns"], facts
            desktop.step_out()
            desktop.down_to("Budgets")
            assert desktop.showing()["selected"]["side"] == "$21,564 this month"
            desktop.shoot(f"launcher-budgets-at-{named}")
            desktop.step_in()
            rows = desktop.showing()["rows"]
            assert rows[:4] == [
                "All AI Spend",
                "Claude Pro",
                "Your AI proxy token",
                "Delta team token",
            ], rows
            desktop.down_to("Your AI proxy token")
            facts = drawer_shot(
                desktop, "Your AI proxy token", f"launcher-budgets-proxy-at-{named}"
            )
            assert facts == ["Premium Day", "Week", "Spent", "Feed"], facts
            desktop.step_in()
            desktop.down_to("Premium Day")
            facts = drawer_shot(desktop, "Premium Day", f"launcher-budgets-dollars-at-{named}")
            assert facts == ["Standing", "Resets", "Read By", "Warns"], facts
            desktop.step_out()
            desktop.down_to("Claude Pro")
            facts = drawer_shot(desktop, "Claude Pro", f"launcher-budgets-each-at-{named}")
            assert facts == ["Five Hours", "Week", "Worth at List", "Feed", "Providers"], facts
            desktop.step_in()
            rows = desktop.showing()["rows"]
            assert rows[:2] == ["Five Hours", "Week"], rows
            facts = drawer_shot(desktop, "Five Hours", f"launcher-budgets-windows-at-{named}")
            assert facts == ["Resets", "Read By", "Warns"], facts
            desktop.close_launcher()
    finally:
        desktop.as_account(
            "rm -f $XDG_RUNTIME_DIR/spend-watch/collection.json"
            " $XDG_RUNTIME_DIR/spend-watch/budgets.json"
        )


# What's typed for the meaning daemon's stand-in to find by its meaning, the words in the keys
# of the things it finds for it, among those the launcher told it of, and the place it finds,
# Screens, whose Brightness stands in its stead.
MEANT = "too bright to read"
MEANT_KEYS = ("bright", "night")
MEANT_PLACE = "collection:displays"
# The most rows meaning adds below what typing found (VKCF3Z, 2026-11-07).
MEANT_MOST = 3


def test_what_is_found_by_its_meaning_sits_below_what_typing_found_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Find a thing in the launcher, then a screenshot in Recent, by their meaning.

    One subtest holds both, since both prove the one wiring to the meaning daemon, its things
    and its pictures, and the looks test is at its limit (docs/testing.md).
    """
    a_thing_found_by_its_meaning(desktop, at_each_scale)
    a_screenshot_found_by_what_it_shows(desktop, at_each_scale)


def a_thing_found_by_its_meaning(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Find things by their meaning, below what typing found, and shoot them.

    The launcher tells the meaning daemon its things as it opens and asks it what's typed, on
    a stand-in that loads no model (tests/desktop/stand-in-meaning.py): at most three of what
    it finds are added below every row typing found, none of them twice, each saying it was
    found by its meaning (VKCF3Z, 2026-11-07). Which it adds, and that nothing moves when they
    arrive late, is tests/meaning.qml's to prove; this is the wiring, and the look at Joel's
    scales.
    """
    desktop.as_account("rm -f $XDG_RUNTIME_DIR/meaning-means.json $XDG_RUNTIME_DIR/meaning-asked")
    desktop.answer(MEANT)
    desktop.machine.wait_until_succeeds(
        desktop.in_session("grep -q '^know [1-9]' $XDG_RUNTIME_DIR/meaning-asked"),
        timeout=timedelta(seconds=10),
    )
    typed = [row["is"] for row in desktop.showing()["things"]]
    desktop.close_launcher()
    known = desktop.as_account("cat $XDG_RUNTIME_DIR/meaning-known").split()
    meant = [key for key in known if key == MEANT_PLACE or any(word in key for word in MEANT_KEYS)]
    assert meant, f"the launcher told of nothing a test can mean: {known}"

    # Two keys can name one thing, a spot and its collection, so what's added is seen by
    # identity: rows after the typed ones, and none of them twice.
    def added(shown: dict[str, Any]) -> list[str]:
        found = [row["is"] for row in shown["things"]]
        return found[len(typed) :] if found[: len(typed)] == typed else []

    desktop.as_account(
        f"echo {shlex.quote(json.dumps({MEANT: meant}))} > $XDG_RUNTIME_DIR/meaning-means.json"
    )
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            desktop.answer(MEANT)
            desktop.launcher_until(lambda shown: added(shown) != [])
            meanings = added(desktop.showing())
            assert len(meanings) <= MEANT_MOST, f"meaning added too many: {meanings}"
            assert len(set(meanings)) == len(meanings), f"a meaning came twice: {meanings}"
            assert not set(meanings) & set(typed), f"a meaning repeats a typed row: {meanings}"
            assert MEANT_PLACE not in meanings, f"Screens stood beside its Brightness: {meanings}"
            desktop.down_to(desktop.showing()["rows"][len(typed)])
            note = desktop.showing()["selected"]["note"]
            assert note == "found by its meaning", f"a meant row says {note!r}"
            desktop.shoot(f"launcher-found-by-meaning-at-{scale.replace('.', '-')}")
            desktop.close_launcher()
    finally:
        desktop.as_account("rm -f $XDG_RUNTIME_DIR/meaning-means.json")


def drawn(shape: str, colour: str) -> str:
    """Give a picture of one shape on white, as base64, for the drawer to show."""
    picture = Image.new("RGB", (224, 224), "white")
    box = (40, 40, 184, 184)
    if shape == "circle":
        ImageDraw.Draw(picture).ellipse(box, fill=colour)
    else:
        ImageDraw.Draw(picture).rectangle(box, fill=colour)
    png = io.BytesIO()
    picture.save(png, "PNG")
    return base64.b64encode(png.getvalue()).decode()


def a_screenshot_found_by_what_it_shows(desktop: Desktop, at_each_scale: list[str]) -> None:
    """In Capture's Recent, a screenshot found by its look sits below those found by name and words.

    Typing in Recent finds the screenshot named for the words, then the one whose words hold
    them, its line below it, then the one the meaning daemon says looks like them, on a stand-in
    that loads no model (tests/desktop/stand-in-meaning.py), saying in the foot that it was
    found by its meaning and nothing under it (VKCF3Z, 2026-11-07). Which it adds is
    tests/meaning.qml's to prove; this is the wiring, and the look at Joel's scales.
    """
    shots = desktop.pictures
    named, said, seen = (
        f"{shots}/wombat plan.png",
        f"{shots}/2026-10-01 09-00-00.png",
        f"{shots}/2026-09-15 08-00-00.png",
    )
    texts = {said: ["Standup", "The wombat review moved to Friday"], named: ["Wombat"]}
    pictures = {named: drawn("square", "steelblue"), said: drawn("square", "grey")}
    pictures[seen] = drawn("circle", "sienna")
    desktop.as_account(
        f"mkdir -p {shlex.quote(shots)}"
        + "".join(
            f" && printf %s {picture} | base64 -d > {shlex.quote(path)}"
            for path, picture in pictures.items()
        )
        + f" && printf %s {shlex.quote(json.dumps(texts))}"
        " > $XDG_RUNTIME_DIR/screen-text-texts.json"
        + f" && printf %s {shlex.quote(json.dumps({'wombat': [seen]}))}"
        " > $XDG_RUNTIME_DIR/meaning-means.json"
    )
    found = ["wombat plan.png", "2026-10-01 09-00-00.png", "2026-09-15 08-00-00.png"]
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            desktop.answer("wombat")
            desktop.launcher_until(
                lambda shown: (shown["selected"] or {}).get("name") == "wombat plan.png"
            )
            desktop.step_in()
            desktop.launcher_until(
                lambda shown: shown["held"][-1:] == ["Recent"] and shown["rows"] == found
            )
            desktop.down_to(found[1])
            desktop.launcher_until(
                lambda shown: shown["selected"]["note"] == "The wombat review moved to Friday"
            )
            desktop.down_to(found[2])
            note = desktop.showing()["selected"]["note"]
            assert note == "found by its meaning", f"a looked-at screenshot says {note!r}"
            desktop.shoot(f"capture-recent-found-by-meaning-at-{scale.replace('.', '-')}")
            desktop.close_launcher()
        asked = desktop.as_account("cat $XDG_RUNTIME_DIR/meaning-asked").split("\n")
        assert "look wombat" in asked, f"the pictures were never asked of: {asked}"
    finally:
        desktop.as_account(
            "rm -f $XDG_RUNTIME_DIR/meaning-means.json $XDG_RUNTIME_DIR/screen-text-texts.json "
            + " ".join(shlex.quote(path) for path in pictures)
        )


# The ErgoDox as the kernel names it, the key the layout finds only on it, and what its watcher
# says while it's locked on its keypad's layer.
ERGODOX = "ZSA Technology Labs ErgoDox EZ"
ITS_KEY = "7 keypad 7"
LOCKED = "Numb locked"
# The laptop's own layout's one layer, as keyd's module describes it (modules/keyd/layout.nix).
LAPTOPS = "Colemak"
# How far, as the mean of every pixel's change in light, the screen moves once the layout's
# card stands on it: measured at 2, the card moves it by ten, and a still screen by nothing.
LAYOUT_CHANGES = 3.0
# How far it moves as one keyboard's card turns to the other's in place, which is less, since
# the two cards overlap.
CARD_TURNS = 1.0


def plug_in_the_ergodox(desktop: Desktop) -> None:
    """Plug the stand-in ErgoDox in, and say its layer as its watcher does once it's there.

    keyboard-lights hears it come, so the shell lists it, and locked, the layer's mark stands
    in the margin, where a mode Joel can forget is held.
    """
    machine = desktop.machine
    since = desktop.mark()
    desktop.as_account(f"echo {shlex.quote(LOCKED)} > $XDG_RUNTIME_DIR/keyboard-layer")
    machine.succeed(f"systemd-run --unit=ergodox {desktop.values['ergodox']}")
    machine.wait_for_file("/run/ergodox.device", timeout=timedelta(seconds=30))
    desktop.reported("holds margin ", lambda rest: "layer" in rest.split(), since)


def unplug_the_ergodox(desktop: Desktop) -> None:
    """Unplug the stand-in ErgoDox, its layer's mark going with it.

    Its watcher's last word is left standing, so only the keyboards connected now can say it's
    gone, as keyboard-lights hears it go.
    """
    since = desktop.mark()
    desktop.machine.succeed("systemctl stop ergodox")
    desktop.reported("holds margin ", lambda rest: "layer" not in rest.split(), since)


def the_card_showing(desktop: Desktop, layer: str, since: str) -> int:
    """Wait for the map to say it shows the layer, and give how tall its card stands."""
    heights: list[int] = []

    def showing(rest: str) -> bool:
        shown, _, height = rest.rpartition(" ")
        if shown == layer:
            heights.append(int(height))
        return shown == layer

    desktop.reported("keyboard layer ", showing, since)
    return heights[-1]


def the_layout_drawn(desktop: Desktop, layer: str) -> int:
    """Bring the layout up by Super+K's call, wait until its card is on the screen, its height.

    The shell says it's drawn as its surface is mapped, but in a guest drawing in software its
    76 keys reach the screen a second or more later, so its pixels are waited on instead.
    """
    bare = desktop.screen_now()
    since = desktop.mark()
    with desktop.told("drawn keyboard"):
        desktop.as_account("qs ipc call keyboard toggle")
    height = the_card_showing(desktop, layer, since)
    desktop_session.retry(
        lambda _: changed_from(bare, desktop.screen_now()) > LAYOUT_CHANGES,
        timeout=timedelta(seconds=30),
    )
    return height


def the_card_turned(desktop: Desktop, since: str, before: Image.Image, layer: str) -> int:
    """Wait for the open card to turn in place to another keyboard's layout, its height."""
    height = the_card_showing(desktop, layer, since)
    desktop_session.retry(
        lambda _: changed_from(before, desktop.screen_now()) > CARD_TURNS,
        timeout=timedelta(seconds=30),
    )
    return height


def keyboards_listed(desktop: Desktop, scale: str, plugged: str) -> list[str]:
    """Give the keyboards the launcher lists, once its search finds the ErgoDox's key or not.

    The keyboards are shot each time, for the design's look.
    """
    there = plugged != "unplugged"
    desktop.answer("keypad 7")
    desktop.launcher_until(lambda shown: (ITS_KEY in shown["rows"]) == there)
    desktop.close_launcher()
    desktop.answer("keyboards")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "The Laptop's Keyboard" in shown["rows"])
    desktop.shoot(f"launcher-keyboards-{plugged}-at-{scale.replace('.', '-')}")
    rows = cast("list[str]", desktop.showing()["rows"])
    desktop.close_launcher()
    return rows


# A keyboard that isn't plugged in is shown nowhere (Joel, 2026-11-07: "we shouldn't be showing
# keyboard shortcuts for keyboards that are not plugged in"): unplugging the ErgoDox turns its
# open layout to the laptop's own in place, the margin lets its layer's mark go, and the
# launcher neither finds its keys nor lists it among the keyboards; Super+K then shows the
# laptop's, and plugging it back in turns the open card to the board's again (VKCF3Z,
# 2026-11-07). With it back, its drawer and every other new drawer are photographed
# (each_drawer_drawn), here rather than in a subtest of their own, since the board is
# already plugged in and the file keeps to twenty.


def test_an_unplugged_ergodox_is_shown_nowhere_until_its_back(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Unplugging the ErgoDox shows the laptop's layout and leaves it out of the launcher."""
    scale = at_each_scale[-1]
    at_scale(desktop, scale)
    desktop.as_account(
        f"mkdir -p ~/.config/ergodox && cp {desktop.values['ergodox_layout']} ~/.config/ergodox/.l"
        " && mv ~/.config/ergodox/.l ~/.config/ergodox/layout.json"
    )
    named = scale.replace(".", "-")
    try:
        plug_in_the_ergodox(desktop)
        assert ERGODOX in keyboards_listed(desktop, scale, "plugged")
        boards = the_layout_drawn(desktop, "Numb")
        desktop.shoot(f"keyboard-layout-plugged-at-{named}")
        # The open card stays as the board goes, turned to the laptop's own.
        before = desktop.screen_now()
        since = desktop.mark()
        unplug_the_ergodox(desktop)
        turned = the_card_turned(desktop, since, before, LAPTOPS)
        desktop.shoot(f"keyboard-layout-unplugged-at-{named}")
        with desktop.told("gone keyboard"):
            desktop.as_account("qs ipc call keyboard toggle")
        rows = keyboards_listed(desktop, scale, "unplugged")
        assert ERGODOX not in rows, rows
        # The laptop's six rows stand shorter than the board's eight, with no band left of the
        # board's taller card, turned to or opened on.
        assert turned < boards, f"turned to the laptop's at {turned}, the board's at {boards}"
        # Super+K with the laptop alone shows the laptop's, never nothing.
        fresh = the_layout_drawn(desktop, LAPTOPS)
        assert fresh == turned, f"opened on the laptop's at {fresh}, turned to it at {turned}"
        before = desktop.screen_now()
        since = desktop.mark()
        plug_in_the_ergodox(desktop)
        the_card_turned(desktop, since, before, "Numb")
        with desktop.told("gone keyboard"):
            desktop.as_account("qs ipc call keyboard toggle")
        assert ERGODOX in keyboards_listed(desktop, scale, "back")
        each_drawer_drawn(desktop, at_each_scale)
    finally:
        desktop.machine.succeed("systemctl stop ergodox || true")
        desktop.as_account("rm -f ~/.config/ergodox/layout.json $XDG_RUNTIME_DIR/keyboard-layer")


# How long a wait lasts before Now lists Lag, as tests/desktop.nix sets it.
LAG_LASTS = 3600


def disk_filling(desktop: Desktop) -> None:
    """Write the machine's disk guard's status, /nix urgent and filling, as the guard does."""
    now = time.time()
    status = {
        "v": 1,
        "heartbeat": now + 3600,
        "filesystems": [
            {
                "path": "/nix",
                "band": "urgent",
                "free": 18 * 2**30,
                "total": 900 * 2**30,
                "fill_per_hour": 3 * 2**30,
                "full_at": now + 6 * 3600,
                "says": "The disk is nearly full, with 18.0 GiB free.",
            }
        ],
        "frozen": [],
        "holders": [
            {"path": "/nix/store", "bytes": 610 * 2**30, "grew": {"bytes": 9 * 2**30}},
            {"path": "/var/log", "bytes": 2 * 2**30, "grew": {"bytes": 2**29}},
        ],
        "holders_at": now,
        "cleared": [
            {"at": now - 5400, "kind": "cache", "path": "/nix/var/cache", "bytes": 6 * 2**30}
        ],
    }
    desktop.machine.succeed(
        f"echo '{json.dumps(status)}' > /run/disk-guard/.status"
        " && mv /run/disk-guard/.status /run/disk-guard/status.json"
    )


def lag_lasting(desktop: Desktop) -> None:
    """Write a window as lag-watch does, a wait on memory begun longer ago than Lag waits.

    A window with no wait goes first, heard before the next, so a wait lag-watch itself saw
    earlier ends and the stand-in's is a run of its own. Lag hears a window lag-watch stopped
    rewriting as silent, so it's written just before its drawer is opened, each time.
    """
    now = int(desktop.machine.succeed("date +%s").strip())
    lagged = "$XDG_RUNTIME_DIR/lag-watch/status.json"

    def write(window: Json) -> None:
        desktop.as_account(
            f"mkdir -p $(dirname {lagged}) && echo '{json.dumps(window)}' > {lagged}.new"
            f" && mv {lagged}.new {lagged}"
        )

    with desktop.told("heard lag nothing"):
        write({"ended": now, "trouble": [], "worst": "", "who": "", "since": None})
    with desktop.told("heard lag memory"):
        write(lasting_window(now))


def lasting_window(now: int) -> Json:
    """Give lag-watch's window of a wait on memory begun longer ago than Lag waits.

    It ends five minutes on, since Lag hears a window 30 seconds past its end as lag-watch
    gone, and with lag-watch resting nothing writes the next: a walk to Lag at 2 outlasted it.
    """
    return {
        "ended": now + 300,
        "seconds": 10,
        "stalls": {
            "app": {"cpu_some": 2.4, "memory_some": 12.5, "io_some": 0.04},
            "session": {"cpu_some": 0.5, "memory_some": 3.1, "io_some": 0.0},
        },
        "memory": {"app": {"swap_mib": 1144}},
        "trouble": ["apps waited on memory"],
        "worst": "memory",
        "who": "apps",
        "since": now - LAG_LASTS - 10,
    }


def now_drawer(desktop: Desktop, row: str, eyebrow: str, shot: str) -> None:
    """Open Now by Super+O, move down to a piece's row, and photograph its drawer."""
    desktop.machine.send_key("meta_l-o")
    desktop.in_now("Needs You")
    desktop.launcher_until(lambda shown: row in shown["rows"])
    desktop.down_to(row)
    desktop.launcher_until(
        lambda shown: shown["drawer"]["out"] is True and shown["drawer"]["eyebrow"] == eyebrow
    )
    desktop.shoot(shot)
    desktop.close_launcher()


# Each drawer this desktop's integrations gained says everything its source holds (Joel,
# 2026-11-07: "every integration has an insanely high bar of nerdiness and integration"), where
# the thing is and what it is above, and each is photographed at each of Joel's scales for the
# design's look: mpv's program row and its Media, playing and paused, the ErgoDox and the
# laptop's own board, a picture Capture took, and Now's Disks and Lag.
def media_drawer(desktop: Desktop, shot: str, side: str, program: str = "") -> None:
    """Open mpv's Media, its side saying how it stands, and photograph its drawer.

    On the way, mpv's own row's drawer is photographed too, where a shot is named for it.
    """
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("mpv", selecting="mpv Media Player")
    if program:
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"] is True
                and shown["drawer"]["title"] == "mpv Media Player"
                and "Playing" in shown["drawer"]["facts"]
            )
        )
        desktop.shoot(program)
    desktop.into_program()
    desktop.type_in("media", selecting="Media")
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and shown["drawer"]["title"] == "Bless The Child"
            and any(
                thing["reads"].startswith("Media") and side in thing["reads"]
                for thing in shown["things"]
            )
        )
    )
    drawer = desktop.showing()["drawer"]
    assert drawer["eyebrow"].endswith(" \u00b7 track"), drawer
    assert drawer["facts"][0] == "Position", drawer
    desktop.shoot(shot)
    desktop.close_launcher()


def board_drawer(desktop: Desktop, name: str, eyebrow: str, shot: str) -> None:
    """Open the keyboards on one, and photograph its drawer."""
    desktop.answer("keyboards")
    desktop.step_in()
    desktop.launcher_until(lambda shown: name in shown["rows"])
    desktop.down_to(name)
    desktop.launcher_until(
        lambda shown: shown["drawer"]["out"] is True and shown["drawer"]["title"] == name
    )
    drawer = desktop.showing()["drawer"]
    assert drawer["eyebrow"] == eyebrow, drawer
    desktop.shoot(shot)
    desktop.close_launcher()


def recent_drawer(desktop: Desktop, shot: str) -> None:
    """Open what Capture took lately on the newest, and photograph its drawer."""
    desktop.capture()
    desktop.type_in("recent", selecting="Recent")
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            shown["drawer"]["out"] is True
            and shown["drawer"]["eyebrow"] == "in ~/Pictures/Screenshots \u00b7 screenshot"
            and "Dimensions" in shown["drawer"]["facts"]
        )
    )
    desktop.shoot(shot)
    desktop.close_launcher()


def drawers_at(desktop: Desktop, scale: str) -> None:
    """Photograph each drawer at one of Joel's scales."""
    at_scale(desktop, scale)
    at = scale.replace(".", "-")
    media_drawer(
        desktop,
        f"launcher-media-drawer-playing-at-{at}",
        "playing",
        program=f"launcher-program-drawer-at-{at}",
    )
    desktop.as_account("qs ipc call music toggle")
    media_drawer(desktop, f"launcher-media-drawer-paused-at-{at}", "paused")
    desktop.as_account("qs ipc call music toggle")
    board_drawer(desktop, ERGODOX, "on USB \u00b7 keyboard", f"launcher-ergodox-drawer-at-{at}")
    board_drawer(
        desktop,
        "The Laptop's Keyboard",
        "built in \u00b7 keyboard",
        f"launcher-laptops-keyboard-drawer-at-{at}",
    )
    recent_drawer(desktop, f"launcher-capture-drawer-at-{at}")
    now_drawer(desktop, "Disks", "on this machine \u00b7 disk", f"launcher-disk-drawer-at-{at}")
    lag_lasting(desktop)
    now_drawer(desktop, "Lag", "on this machine \u00b7 lag", f"launcher-lag-drawer-at-{at}")


# Called from the unplugged ErgoDox subtest once the board is back, so it's plugged in here and
# that subtest's own finally unplugs it.
def each_drawer_drawn(desktop: Desktop, scales: list[str]) -> None:
    """Photograph a program's, its Media's, each keyboard's, a capture's, Disks' and Lag's."""
    machine = desktop.machine
    desktop.as_account(
        "systemd-run --user --unit=player mpv --ao=null --no-video --loop"
        f" {shlex.quote(desktop.values['track'])}"
    )
    named = "busctl --user list --acquired | grep -q '^org\\.mpris\\.MediaPlayer2\\.mpv'"
    machine.wait_until_succeeds(desktop.in_session(named), timeout=timedelta(seconds=30))
    # lag-watch would write its own window over the stand-in's, so it rests.
    desktop.as_account("systemctl --user stop lag-watch.service")
    try:
        desktop.capture()
        with desktop.captured():
            machine.send_key("ret")
        disk_filling(desktop)
        for scale in scales:
            drawers_at(desktop, scale)
    finally:
        desktop.as_account("systemctl --user stop player || true")
        machine.succeed("rm -f /run/disk-guard/status.json")
        desktop.as_account(
            "rm -f $XDG_RUNTIME_DIR/lag-watch/status.json"
            " && systemctl --user start lag-watch.service"
        )


# The device Nearby hears: btvirt's second controller, named as BlueZ names a device by what it
# calls itself, and how strong btvirt says every device is heard (emulator/btdev.c's inquiry).
HEARD = "Buds"
HEARD_AT = "\u221260 dBm \u00b7 strong"
# BlueZ's adapters, both seen as Buds: the launcher looks from the first Quickshell is told of,
# which with BlueZ starting after the shell is either, and hears the other.
ADAPTERS = ("/org/bluez/hci0", "/org/bluez/hci1")


def bluez_set(desktop: Desktop, adapter: str, name: str, kind: str, value: str) -> None:
    """Set one of an adapter's properties over BlueZ's D-Bus, as bluetoothctl would."""
    desktop.machine.succeed(
        f"busctl set-property org.bluez {adapter} org.bluez.Adapter1 {name} {kind} {value}"
    )


def discovering(adapter: str) -> str:
    """Ask BlueZ whether an adapter is looking for devices, which busctl says as "b true"."""
    return f"busctl get-property org.bluez {adapter} org.bluez.Adapter1 Discovering"


def looking(desktop: Desktop) -> bool:
    """Say whether the launcher's adapter, whichever it is, is looking for devices."""
    return any(
        desktop.machine.succeed(discovering(adapter)).strip() == "b true" for adapter in ADAPTERS
    )


def radios_up(desktop: Desktop) -> None:
    """Power both of btvirt's adapters once BlueZ lists them."""
    for adapter in ADAPTERS:
        # An adapter BlueZ lists may still be setting up, and refuses power until it's done.
        desktop.machine.wait_until_succeeds(
            f"busctl set-property org.bluez {adapter} org.bluez.Adapter1 Powered b true",
            timeout=timedelta(seconds=30),
        )


def radios_seen(desktop: Desktop, seen: bool) -> None:
    """Make both adapters visible as Buds, for good, or hide them; BlueZ keeps which."""
    for adapter in ADAPTERS:
        bluez_set(desktop, adapter, "Alias", "s", HEARD)
        bluez_set(desktop, adapter, "DiscoverableTimeout", "u", "0")
        bluez_set(desktop, adapter, "Discoverable", "b", "true" if seen else "false")


def looks_soon(desktop: Desktop) -> None:
    """Wait for the launcher's adapter, whichever it is, to look for devices."""
    desktop.machine.wait_until_succeeds(
        " || ".join(f"{discovering(adapter)} | grep -qx 'b true'" for adapter in ADAPTERS),
        timeout=timedelta(seconds=30),
    )


def buds(shown: dict[str, Any]) -> list[bool]:
    """Say, for each row of Buds the launcher lists, whether it's gone."""
    return [thing["gone"] for thing in shown["things"] if thing["reads"].startswith(HEARD)]


# Nearby lists the devices the radio hears only while it's open (VKCF3Z, 2026-11-07): two
# controllers of btvirt's, each made visible as Buds, so the launcher's radio, whichever it is,
# looks once Nearby is stepped into, and the other is listed with its signal as Quickshell's
# patch says it (modules/quickshell/bluetooth-rssi-and-failures.patch), never paired. BlueZ
# starts here after the shell, as a restarted bluetoothd does, which the shell takes up
# (bluetooth-finds-bluez-late.patch). bluetoothd restarting, as a switch restarts it, says
# nothing of its devices as it stops, and comes back knowing none it hadn't paired, so the shell
# forgets them as it goes and keeps only what BlueZ has on return: with both radios hidden
# across the restart, Buds stays in Nearby, still open, gone, as Steady keeps a row, rather than
# heard on with its last signal, while Nearby looks again; shown once more, Buds is heard afresh,
# by whichever radio the shell now takes for its own, so as the same row or beside the one gone.
# Run last of the area's, since BlueZ starting moves what the launcher holds.


def nearby_heard(desktop: Desktop, at_each_scale: list[str], values: dict[str, Any]) -> None:
    """Hold Nearby to looking once open, listing Buds with its signal, no longer than BlueZ."""
    machine = desktop.machine
    machine.succeed("modprobe bluetooth && modprobe hci_vhci")
    machine.succeed(f"systemd-run --unit=btvirt {values['btvirt']} -l2")
    machine.succeed("systemctl start bluetooth.service")
    try:
        radios_up(desktop)
        radios_seen(desktop, seen=True)
        for scale in at_each_scale:
            named = scale.replace(".", "-")
            at_scale(desktop, scale)
            desktop.answer("bluetooth")
            # The find answers a moment after the letters, with nothing selected until then.
            desktop.launcher_until(
                lambda shown: (shown["selected"] or {}).get("name") == "Bluetooth"
            )
            desktop.step_in()
            desktop.launcher_until(lambda shown: "Nearby" in shown["rows"])
            # Not looking yet: only Nearby looks.
            assert not looking(desktop)
            desktop.down_to("Nearby")
            desktop.step_in()
            desktop.launcher_until(
                lambda shown: HEARD in shown["rows"], timeout=timedelta(seconds=60)
            )
            assert looking(desktop)
            desktop.down_to(HEARD)
            selected = desktop.showing()["selected"]
            assert selected["side"] == HEARD_AT, selected
            desktop.shoot(f"launcher-nearby-at-{named}")
            desktop.step_out()
            # Out of Nearby, the radio stops looking, and Bluetooth lists only what's paired.
            machine.wait_until_succeeds(
                " && ".join(f"{discovering(adapter)} | grep -qx 'b false'" for adapter in ADAPTERS),
                timeout=timedelta(seconds=30),
            )
            assert HEARD not in desktop.showing()["rows"]
            desktop.shoot(f"launcher-bluetooth-at-{named}")
            desktop.close_launcher()
        desktop.answer("bluetooth")
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Bluetooth")
        desktop.step_in()
        desktop.down_to("Nearby")
        desktop.step_in()
        desktop.launcher_until(lambda shown: buds(shown) == [False], timeout=timedelta(seconds=60))
        # Hidden at once, then stopped well before BlueZ would forget Buds for going unheard.
        radios_seen(desktop, seen=False)
        # Restarted, not stopped then started: a BlueZ client on the bus (WirePlumber's) may ask
        # for it the moment it's gone, which has the bus start it again and cancel a stop.
        machine.succeed("systemctl restart bluetooth.service")
        radios_up(desktop)
        desktop.launcher_until(lambda shown: buds(shown) == [True], timeout=timedelta(seconds=30))
        looks_soon(desktop)
        radios_seen(desktop, seen=True)
        desktop.launcher_until(lambda shown: False in buds(shown), timeout=timedelta(seconds=60))
        desktop.close_launcher()
    finally:
        machine.succeed("systemctl stop bluetooth.service btvirt || true")


# What's around the machine, on its network and in the air, as the places that list it lay it
# out: one subtest, since the area holds the pyramid's twenty (docs/testing.md), and last, for
# Nearby's sake.


def test_the_home_network_and_nearby_list_whats_around_at_each_scale(
    desktop: Desktop, at_each_scale: list[str], values: dict[str, Any]
) -> None:
    """The Home Network lays out the eero daemon's record, and Nearby what the radio hears."""
    home_network_laid_out(desktop, at_each_scale)
    nearby_heard(desktop, at_each_scale, values)
