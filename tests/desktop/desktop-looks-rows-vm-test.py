"""The desktop test's looks area for a row's words.

Here the launcher is laid out at Joel's two scales, on a screen as large as the laptop's, which the
other areas' screen is too small to hold the card and its drawer at: 1.5, as the HP draws it, and 2,
as the laptop does. A found row's note is in the serif, on its name's baseline, a step on, and
home's own foot and the empty places' cat are held here too, each shot for a person to measure.
It's an area of its own, rather than more of looks, so the looks areas run side by side rather
than one after another; what they share is in looks_session.py. tests/desktop.nix runs this
through pytest inside the driver (tests/lib/vm_pytest.py), and its opening comment says what the
test proves. Every test asks for the desktop, started once a run (desktop_session.py), and leaves
it as it found it.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING

import desktop_session

# pytest finds a fixture by its name in the test's module, so the looks areas' desktop and
# scales are brought here.
from looks_session import NEAR, at_each_scale, at_scale, bottom_set, desktop, drawn, plate_shown  # noqa: F401

if TYPE_CHECKING:
    from desktop_session import Desktop, Json

# A folder kitty has visited, so kitty found holds something under it.
VISITED = "~/src/tidepool"
# A row's words stand a step from its name (Theme.space.m), in the theme's serif, and on its
# baseline to half a device pixel.
WORDS_GAP = 12
HALF_A_PIXEL = 0.5


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
    """A found row's note is in the serif, on the name's baseline, a step on.

    Home's rows say nothing after their names (VKCF3Z, dae7fb67), which place-row.qml holds,
    so the words are held where a row still has them, and home's own foot is held here.
    """
    # The serif is the theme's, read as the theme exports it, so a theme with another face is
    # held to that face.
    theme = json.loads(desktop.as_account("cat ~/.config/jitsusama/theme.json"))
    serif = theme["font"]["serif"]["family"]
    for scale in at_each_scale:
        at_scale(desktop, scale)
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: shown["drawer"]["out"] is True)
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
        counts_with_its_chord_in_the_foot(desktop, scale)
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
    # The program's drawer holds its identifiers, a desktop entry's id among them, whole in its
    # frame, broken at their own marks (VKCF3Z, 2026-11-09).
    desktop.launcher_until(lambda shown: shown["drawer"]["out"] is True)
    # Its values are known before they're judged, since a drawer with none known says no.
    desktop.launcher_until(lambda shown: any(v is not None for v in shown["drawer"]["values"]))
    drawer = desktop.showing()["drawer"]
    assert drawer["valuesWhole"], f"at {scale} a program's value breaks a word: {drawer}"
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


def counts_with_its_chord_in_the_foot(desktop: Desktop, scale: str) -> None:
    """Step into Desktop, whose collections each open by a chord, and shoot their counts.

    A collection's row reads in Desktop as it does at home: what it holds, by name after its
    name or by how many at its right, or its word for none once where it holds nothing, and
    its chord taught in the foot as the next time's way, never drawn on the row (VKCF3Z,
    2a429f67). A find's rows say no count, which would read all of each at every key.
    """
    desktop.launcher_until(lambda shown: not shown["open"])
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.down_to("Desktop")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["desktop"])
    selected = desktop.showing()["selected"]
    assert selected["side"] or selected["count"], (
        f"at {scale} Desktop's first says nothing of what it holds: {selected}"
    )
    assert selected["chord"] == "", f"at {scale} Desktop's first draws its chord: {selected}"
    taught = " ".join(desktop.showing()["wanted"])
    assert "next time" in taught, f"at {scale} the foot doesn't teach its chord: {taught}"
    desktop.shoot(f"launcher-desktop-counts-at-{scale.replace('.', '-')}")
    desktop.close_launcher()


# A cat's cell in the screen's pixels at each scale: whole, and 16 logical pixels across at both.
CAT_CELL = {"1.5": 3, "2": 4}


def cat_drawn(desktop: Desktop, scale: str, mark: str, shot: str) -> None:
    """Wait for the empty line's cat in the pose for `mark`, hold it to whole pixels, shoot it."""
    desktop.launcher_until(lambda shown: (shown["nothingDrawn"] or {}).get("mark") == mark)
    drawn = desktop.showing()["nothingDrawn"]
    assert drawn["cell"] == CAT_CELL[scale], f"at {scale} the cat's cell: {drawn}"
    off = [abs(at - round(at)) for at in drawn["at"]]
    assert max(off) < HALF_A_PIXEL / 10, f"at {scale} the cat splits a pixel: {drawn}"
    # Its foot on the sentence's baseline, to the pixel snapping may move it (VKCF3Z, e5239c50).
    assert abs(drawn["fromBaseline"]) <= 1, f"at {scale} the cat stands off the line: {drawn}"
    desktop.shoot(f"launcher-cat-{shot}-at-{scale.replace('.', '-')}")


def test_the_empty_places_cat_stands_in_whole_pixels(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Where a list is empty the cat stands in its mark column, in the pose its nothing means.

    It reads by a find that found nothing, sleeps in For You's empty Desktop, and sits by an
    empty Recent; each cell is whole pixels of the screen, starting on one, at 1.5 as at 2,
    and its foot stands on the sentence's baseline.
    """
    machine = desktop.machine
    for scale in at_each_scale:
        at_scale(desktop, scale)
        desktop.answer("zqxjvw")
        cat_drawn(desktop, scale, "nothingFound", "reading")
        desktop.close_launcher()
        machine.send_key("meta_l-n")
        desktop.launcher_until(lambda shown: shown["held"][-1:] == ["Desktop"])
        cat_drawn(desktop, scale, "forYouEmpty", "asleep")
        desktop.close_launcher()
        desktop.answer("capture")
        desktop.down_to("Capture")
        desktop.step_in()
        desktop.launcher_until(lambda shown: shown["held"][-1:] == ["captures"])
        desktop.down_to("Recent")
        desktop.step_in()
        cat_drawn(desktop, scale, "nothingHere", "sitting")
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
        # The wait ends before the loop moves on, so the lambda reads this key's words.
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
