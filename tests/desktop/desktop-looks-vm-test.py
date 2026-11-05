"""The desktop test's looks area.

Here the launcher is laid out at Joel's two scales, on a screen as large as the laptop's, which
the other areas' screen is too small to hold the card and its drawer at: 1.5, as the HP draws it,
and 2, as the laptop does. Its drawer is a plate whose every line lies on one of the card's
(docs/design.md, the drawer), measured from what the launcher reports of both to a hundredth of a
pixel, and photographed in the gallery for a person to measure too; the file picker's plate is
held to the same lines for each kind of file it previews. tests/desktop.nix runs this
through pytest inside the driver (tests/lib/vm_pytest.py), and its opening comment says what the
test proves. Every test asks for the desktop, started once a run (desktop_session.py), and leaves
it as it found it.
"""

from __future__ import annotations

import json
from datetime import timedelta
from functools import partial
from itertools import pairwise
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# Joel's scales.
JOELS_SCALES = ("1.5", "2")
# The drawer takes what the screen leaves beside the centred card, less the gap between them
# and the margin at the screen's edge, up to its most (Launcher.qml's drawerRoom). On the
# laptop's 2880 that's 520 at 1.5 and 344 at 2; niri gives the VM's screen a few logical
# pixels less, so it's worked out from the screen niri says it has.
CARD = 680
GAP = 12
MARGIN = 24
MOST = 520
# The plate's pitch, half a row, and how near a line is to where it should be: a hundredth,
# and the half a hundredth each of two lengths the launcher reports rounded may take.
PITCH = 24
NEAR = 0.02
# How far the text keeps from the plate's frame on every side, its foot included: an inset and a
# step, 4 and 16, inside the border.
INSIDE = 20
# A row's words stand a step from its name (Theme.space.m), in the theme's serif, and on its
# baseline to half a device pixel.
WORDS_GAP = 12
HALF_A_PIXEL = 0.5


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


def on_the_pitch(line: float, from_line: float) -> bool:
    """Say whether a line lies a whole number of the plate's lines below another."""
    steps = round((line - from_line) / PITCH)
    return steps >= 0 and abs(line - from_line - steps * PITCH) <= NEAR


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
    assert abs(lines["title"] - card["rows"][0]) <= NEAR, (
        f"at {scale} the title is off the first row's baseline: {drawer}"
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
        desktop.photograph(f"launcher-row-words-at-{scale.replace('.', '-')}")
        desktop.type_in("kitty")
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "program:kitty")
        card = desktop.showing()["drawer"]["cardLines"]
        assert words_on_their_names(card, scale, serif) > 0, (
            f"at {scale} kitty's row says nothing: {card}"
        )
        desktop.photograph(f"launcher-found-words-at-{scale.replace('.', '-')}")
        # In normal the caret is a block on the last letter and the foot names no mode, the
        # row's words where insert left them.
        desktop.normal()
        card = desktop.showing()["drawer"]["cardLines"]
        assert words_on_their_names(card, scale, serif) > 0, (
            f"at {scale} kitty's row says nothing in normal: {card}"
        )
        desktop.photograph(f"launcher-found-normal-at-{scale.replace('.', '-')}")
        desktop.close_launcher()


def test_a_plates_title_eyebrow_and_words_lie_on_the_cards_lines(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A notice's plate has its eyebrow on the field's centre and its words on the pitch."""
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
        assert abs(lines["eyebrowMiddle"] - card["field"]) <= NEAR, (
            f"at {scale} the eyebrow is off the field's centre: {drawer}"
        )
        # One empty line below the title.
        assert abs(lines["words"] - (lines["title"] + 2 * PITCH)) <= NEAR, drawer
        assert drawer["eyebrow"].startswith("Desktop \u00b7 "), drawer
        desktop.photograph(f"launcher-plate-at-{scale.replace('.', '-')}")
        desktop.close_launcher()


def test_a_plates_facts_lie_on_the_rows_lines_and_halfway(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A time server's plate has its facts from one empty line below the title, on the pitch."""
    kept = "$XDG_RUNTIME_DIR/timekeeping.json"
    machine = desktop.machine
    machine.wait_until_succeeds(
        desktop.in_session("systemctl --user is-active timekeeping"),
        timeout=timedelta(seconds=60),
    )
    # An exchange made, written as timekeeping writes it, with it stopped so nothing writes
    # over it.
    desktop.as_account("systemctl --user stop timekeeping")
    last = int(machine.succeed("date +%s%3N").strip()) - 120_000
    said = {
        "version": 2, "synchronised": True, "frequency": 6.267,
        "poll": {"now": 2048, "least": 32, "most": 2048},
        "servers": {
            "system": [], "link": [], "runtime": [],
            "fallback": ["0.harbour.pool.ntp.org", "1.harbour.pool.ntp.org"],
        },
        "exchange": {
            "server": "0.harbour.pool.ntp.org", "address": "192.0.2.7", "stratum": 1,
            "reference": "GNSS", "offset": 388, "delay": 36112, "jitter": 11888,
            "rootDelay": 15, "rootDispersion": 457, "precision": -25, "leap": "normal",
            "ntpVersion": 4, "last": last, "packets": 13,
        },
    }  # fmt: skip
    try:
        desktop.as_account(f"echo '{json.dumps(said)}' > {kept}.new && mv {kept}.new {kept}")
        for scale in at_each_scale:
            at_scale(desktop, scale)
            desktop.answer("time")
            desktop.step_in()
            desktop.down_to("0.harbour.pool.ntp.org")
            desktop.launcher_until(
                lambda shown: (
                    shown["drawer"]["out"] is True
                    and shown["drawer"]["title"] == "0.harbour.pool.ntp.org"
                    and len(shown["drawer"]["lines"]["facts"]) > 1
                )
            )
            drawer = plate_shown(desktop, scale)
            title, facts = drawer["lines"]["title"], drawer["lines"]["facts"]
            assert abs(facts[0] - (title + 2 * PITCH)) <= NEAR, drawer
            assert all(
                later > earlier and on_the_pitch(later, earlier)
                for earlier, later in pairwise(facts)
            ), f"at {scale} a fact is off the pitch: {drawer}"
            # Its name is the title, so it isn't said again under it; every other name is
            # whole; and what doesn't fit above the foot's inset waits for Shift and the arrows.
            assert "Name" not in drawer["drawnFacts"], drawer
            assert drawer["labelsWhole"], f"at {scale} a fact's name is broken: {drawer}"
            assert drawer["more"] or facts[-1] < drawer["foot"], (
                f"at {scale} a fact past the foot can't be scrolled to: {drawer}"
            )
            assert drawer["height"] - drawer["foot"] >= INSIDE - NEAR, (
                f"at {scale} the last line runs into the plate's foot: {drawer}"
            )
            desktop.photograph(f"launcher-plate-facts-at-{scale.replace('.', '-')}")
            desktop.close_launcher()
    finally:
        desktop.as_account("systemctl --user start timekeeping")


def test_nows_plate_sets_the_time_and_the_facts_on_homes_rows(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Now's plate centres the time on Now's row and sets the rest on home's rows' lines."""
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
        # The rows' middles the plate sets on are worked out apart from the
        # list, so each lies the same way above the list's own baseline.
        middles = card["middles"]
        drawn = zip(middles, card["rows"][: len(middles)], strict=True)
        drop = [middle - row for middle, row in drawn if row]
        assert all(abs(each - drop[0]) <= NEAR for each in drop), (
            f"at {scale} the plate's row middles drift from the list's rows: {drawer}"
        )
        assert abs(lines["timeMiddle"] - card["middles"][0]) <= NEAR, (
            f"at {scale} the time's capitals aren't centred on Now's row: {drawer}"
        )
        assert abs(lines["seconds"] - lines["time"]) <= NEAR, drawer
        if moment["sayOn"] >= 0:
            assert abs(lines["say"] - card["rows"][moment["sayOn"]]) <= NEAR, (
                f"at {scale} what needs Joel is off its row's line: {drawer}"
            )
        # A group with nothing to show gives up its place, so the first
        # shown starts on Programs' line, whichever it is: the VM has no
        # weather, meeting or place, so there it's the clock's.
        assert moment["groupsOn"][:1] == [shown["rows"].index("Programs")], (
            f"at {scale} the first group of facts isn't on Programs' line: {drawer}"
        )
        for on, group in zip(moment["groupsOn"], lines["groups"], strict=True):
            if on >= 0:
                assert abs(group[0] - card["rows"][on]) <= NEAR, (
                    f"at {scale} a group of facts is off its row's line: {drawer}"
                )
            assert all(
                later > earlier and on_the_pitch(later, earlier)
                for earlier, later in pairwise(group)
            ), f"at {scale} a fact is off the pitch: {drawer}"
        desktop.photograph(f"launcher-now-plate-at-{scale.replace('.', '-')}")
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
                title, facts = drawer["lines"]["title"], drawer["lines"]["facts"]
                shown = drawer["lines"]["previewLines"]
                assert all(on_the_pitch(line, title) for line in [*shown, *facts]), (
                    f"at {scale} {name}'s plate is off the pitch: {drawer}"
                )
                if preview is not None:
                    assert facts[0] > drawer["lines"]["preview"], f"{name}'s facts lead: {drawer}"
                assert facts[-1] < drawer["foot"], f"at {scale} {name}'s facts are cut: {drawer}"
                said = name.replace(".", "-")
                desktop.photograph(f"file-picker-{said}-at-{scale.replace('.', '-')}")
            # Escape leaves insert, then refuses the ask from the folder it opened in.
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


def marked_is_in_sight(shown: Json, scale: str) -> None:
    """Hold Marked, b.md left in it, to its rows' order and its crumbs."""
    rows = shown["rows"]
    assert rows[-3:] == ["b.md", "Unmark All", "Copy the Paths"], rows
    # Its crumb is the place, and the program's ask and the place are said
    # whole, a folder's path giving way first where they don't fit.
    assert shown["path"] == ["Open the Notes", "~/marking/more", "Marked"], shown["path"]
    whole = shown["pathWhole"]
    assert whole[0], f"at {scale} the program's ask is cut: {whole}"
    assert whole[-1], f"at {scale} the place is cut: {whole}"


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
            assert "Open the Marked" not in shown["rows"], shown["rows"]
            desktop.down_to("a.md")
            desktop.launcher_until(partial(selected_is, name="a.md", told="\u21b5 open"))
            desktop.photograph(f"file-picker-one-file-at-{scale.replace('.', '-')}")
            # Escape leaves insert, then refuses the ask from the folder it opened in.
            desktop.close_launcher()
            machine.wait_until_succeeds(f"grep -qx 1 {chose}", timeout=timedelta(seconds=60))
    finally:
        desktop.as_account(f"rm -rf {folder}")


def test_the_file_picker_says_whats_marked_and_what_return_does(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Marking in the file picker says what Return does, and Open the Marked counts and lists."""
    machine = desktop.machine
    folder = f"/home/{desktop.account}/marking"
    chose = "/run/user/1000/portal-chose"
    desktop.as_account(
        f"mkdir -p {folder}/more && printf 'one\\n' > {folder}/a.md"
        f" && printf 'two\\n' > {folder}/b.md"
        f" && touch {folder}/c.md {folder}/more/d.md {folder}/more/e.md"
    )
    try:
        for scale in at_each_scale:
            at_scale(desktop, scale)
            said = scale.replace(".", "-")
            machine.succeed(f"rm -f {chose}")
            desktop.as_account(
                "systemd-run --user --collect --unit=choose-to-mark"
                f" {desktop.values['choose_a_file']} open-several {folder}"
            )
            asked_in(desktop, "~/marking", {"a.md", "b.md", "more"})
            assert desktop.showing()["hint"] == "open which files?", desktop.showing()
            desktop.down_to("Open the Marked")
            desktop.launcher_until(partial(selected_is, name="Open the Marked", side="none"))
            assert not [key for key in desktop.showing()["selected"]["told"] if "\u21b5" in key], (
                "with none marked, Open the Marked says a Return"
            )
            desktop.down_to("a.md")
            desktop.launcher_until(partial(selected_is, name="a.md", told="\u21b5 mark"))
            desktop.photograph(f"file-picker-nothing-marked-at-{said}")
            for name in ["a.md", "b.md"]:
                desktop.down_to(name)
                machine.send_key("ret")
                desktop.launcher_until(partial(selected_is, name=name, told="\u21b5 unmark"))
            desktop.down_to("Open the Marked")
            desktop.launcher_until(partial(selected_is, name="Open the Marked", side="2"))
            desktop.launcher_until(
                lambda shown: (shown["drawer"] or {}).get("eyebrow") == "2\u00a0marked"
            )
            desktop.photograph(f"file-picker-two-marked-at-{said}")
            desktop.down_to("more")
            # The right arrow steps in from normal, where the walk stays from here.
            desktop.step_in()
            desktop.launcher_until(lambda shown: shown["path"][-1:] == ["~/marking/more"])
            desktop.down_to("Open the Marked")
            desktop.launcher_until(partial(selected_is, name="Open the Marked", side="2"))
            desktop.photograph(f"file-picker-marked-elsewhere-at-{said}")
            desktop.step_in()
            desktop.launcher_until(lambda shown: "Unmark All" in shown["rows"])
            desktop.down_to("a.md")
            machine.send_key("ret")
            desktop.launcher_until(partial(selected_is, name="a.md", told="\u21b5 mark"))
            marked_is_in_sight(desktop.showing(), scale)
            desktop.photograph(f"file-picker-inside-marked-at-{said}")
            machine.send_key("left")
            desktop.launcher_until(partial(selected_is, name="Open the Marked", side="1"))
            desktop.launcher_until(lambda shown: shown["rows"][-1:] == ["Copy the Path"])
            desktop.down_to("Copy the Path")
            desktop.photograph(f"file-picker-copy-last-at-{said}")
            # Escape climbs back to the folder it opened in, then refuses the ask there.
            desktop.close_launcher()
            machine.wait_until_succeeds(f"grep -qx 1 {chose}", timeout=timedelta(seconds=60))
    finally:
        desktop.as_account(f"rm -rf {folder}")


# Each order the file picker lists in, what its Order row says of it, and
# the files it lists so: a.md is the smallest and changed between, b.md the
# largest and oldest, c.md between and newest, and the folder first in each.
ORDERS = {
    "Name": ("name", ["more", "a.md", "b.md", "c.md"]),
    "Size": ("size, largest first", ["more", "b.md", "c.md", "a.md"]),
    "Changed": ("changed, newest first", ["more", "c.md", "a.md", "b.md"]),
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
    """Order steps into Name, Changed and Size, and each lists the folder so, folders first."""
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
            asked_in(desktop, "~/ordered", {"Order"})
            for name in ["Size", "Changed", "Name"]:
                side, listed = ORDERS[name]
                desktop.down_to("Order")
                # The right arrow steps in from normal, where the walk stays after the first.
                desktop.step_in()
                desktop.launcher_until(
                    lambda shown: sorted(shown["rows"]) == ["Changed", "Name", "Size"]
                )
                if name == "Size":
                    assert desktop.showing()["rows"] == ["Name", "Changed", "Size"], (
                        "Order doesn't start on Name, the one in use first"
                    )
                    desktop.photograph(f"file-picker-inside-order-at-{said}")
                desktop.down_to(name)
                machine.send_key("ret")
                desktop.launcher_until(partial(listed_from_the_top, listed=listed))
                desktop.down_to("Order")
                desktop.launcher_until(partial(selected_is, name="Order", side=side))
                desktop.photograph(f"file-picker-by-{name.lower()}-at-{said}")
            desktop.close_launcher()
            machine.wait_until_succeeds(f"grep -qx 1 {chose}", timeout=timedelta(seconds=60))
    finally:
        desktop.as_account(f"rm -rf {folder}")


# A session sits under the one orchestrating it, a step in, its glyph where its orchestrator's
# name starts, with no line, box or fold; typed, every row is at no depth, whom it's under
# leading its note; and an orchestrator says what its whole orchestration spent today
# (QEST-20261026-15J7N3's agents-tree-look.md). The orchestrator listens while an owner under it
# runs a builder that works, so the orchestration stands with those working, the owner and its
# builder before the other, listening, session under it.
ORCHESTRATION = [
    # address, title, whom it's under, whether it works
    ("TESTQA", "A quest of its own & whole", None, False),
    ("TESTQB", "The Other One Under It", "TESTQA", False),
    ("TESTQC", "An Owner Under It", "TESTQA", False),
    ("TESTQD", "A Builder Under the Owner", "TESTQC", True),
]
TREE = {"TESTQA": 0, "TESTQC": 1, "TESTQD": 2, "TESTQB": 1}
SPENT = {
    "v": 2,
    "sessions": {"01a1face-test": "$3.10", "01a1face-qd": "$5.10"},
    "helpers": {},
    "families": {
        "QEST-20261022-TESTQA": {"all": "$14.20", "own": "$3.10", "under": "$11.10"},
    },
}


def window_of(address: str, title: str) -> str:
    """Give the title of a session's window, as its label: its quest's name and its folder."""
    return f"\u03c0 - {title} - {address.lower()}-tree"


def orchestrated(desktop: Desktop) -> None:
    """Start the orchestration's sessions, each in a window of its own, saying whom it's under."""
    desktop.pi_session()
    for address, title, under, works in ORCHESTRATION:
        if address != "TESTQA":
            desktop.pi_window(address, window_of(address, title))
        desktop.presence_says(
            {
                "session": (
                    "01a1face-test" if address == "TESTQA" else f"01a1face-{address[-2:].lower()}"
                ),
                "addresses": [address],
                "busy": works,
                "owns": [f"QEST-20261022-{address}"],
                "orchestratedBy": under,
            },
            address,
        )
    desktop.as_account(
        f"mkdir -p $XDG_RUNTIME_DIR/spend-watch && echo '{json.dumps(SPENT)}'"
        " > $XDG_RUNTIME_DIR/spend-watch/.c"
        " && mv $XDG_RUNTIME_DIR/spend-watch/.c $XDG_RUNTIME_DIR/spend-watch/collection.json"
    )


def unorchestrated(desktop: Desktop) -> None:
    """Close the sessions the orchestration started, and forget what they spent."""
    for address, title, _, _ in ORCHESTRATION[1:]:
        window = desktop.window_titled(window_of(address, title))
        desktop.as_account(f"niri msg action close-window --id {window['id']}")
        desktop.as_account(f"rm -f {desktop_session.PI_PRESENCE}/{address.lower()}.json")
    desktop.as_account("rm -f $XDG_RUNTIME_DIR/spend-watch/collection.json")
    desktop.presence_says({"addresses": ["TESTQA"], "busy": True})


def address_of(row: Json) -> str:
    """Say which of the orchestration's sessions a drawn row is."""
    return next(address for address in TREE if address in row["reads"])


def orchestration_drawn(desktop: Desktop) -> list[Json]:
    """Give the orchestration's rows as they're drawn, once two reads in a row agree."""
    was = None
    for _ in range(20):
        laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
        now = [row for row in laid["rows"] if any(address in row["reads"] for address in TREE)]
        if now == was:
            return now
        was = now
    return cast("list[Json]", was)


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
            and sum(any(address in thing["reads"] for address in TREE) for thing in shown["things"])
            == len(TREE)
        )
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
            # Its glyph stands where the name above it starts: the first word a row draws is
            # its glyph.
            words = {address_of(row): row["words"] for row in rows}
            titles = {address: title for address, title, _, _ in ORCHESTRATION}
            for one, above in [("TESTQC", "TESTQA"), ("TESTQB", "TESTQA"), ("TESTQD", "TESTQC")]:
                glyph = words[one][0]["starts"]
                name = next(w["starts"] for w in words[above] if w["text"] == titles[above])
                assert abs(glyph - name) <= 1, (
                    f"at {scale} {one}'s glyph is at {glyph}, not on {above}'s name at {name}"
                )
            # The orchestrator says what all of it spent; one under it, its own. One idle
            # is listening, never waiting.
            reads = {address_of(row): row["reads"] for row in rows}
            assert "$14.20 today in all" in reads["TESTQA"], reads
            assert "$5.10 today" in reads["TESTQD"], reads
            assert "in all" not in reads["TESTQD"], reads
            assert "listening" in reads["TESTQB"], reads
            assert "waiting" not in " ".join(reads.values()), reads
            desktop.photograph(f"launcher-agents-tree-at-{scale.replace('.', '-')}")
            desktop.close_launcher()
        # Inside the orchestrator, what all of it spent, then its own and its sessions'.
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("testqa", selecting="A quest of its own & whole")
        desktop.step_in()
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Go To")
        for name, side in [
            ("Spent Today", "$14.20 in all"),
            ("Its Own", "$3.10"),
            ("Its Sessions", "$11.10"),
        ]:
            desktop.down_to(name)
            assert desktop.showing()["selected"]["side"] == side, desktop.showing()["selected"]
        desktop.close_launcher()
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
        desktop.photograph("launcher-agents-typed-flat")
        desktop.close_launcher()
    finally:
        unorchestrated(desktop)
