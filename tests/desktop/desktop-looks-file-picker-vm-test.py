"""The desktop test's looks area for the file picker.

Here the launcher is laid out at Joel's two scales, on a screen as large as the laptop's, which the
other areas' screen is too small to hold the card and its drawer at: 1.5, as the HP draws it, and 2,
as the laptop does. The file picker's plate is held to the card's lines for each kind of file it
previews (docs/design.md, the drawer); a picker for one file asks for one, a save is named on the
line, :arga gathers files across folders, and :sort lists a folder in the order chosen. It's an area
of its own, rather than more of looks, so the looks areas run side by side rather than one after
another; what they share is in looks_session.py. tests/desktop.nix runs this through pytest inside
the driver (tests/lib/vm_pytest.py), and its opening comment says what the test proves. Every test
asks for the desktop, started once a run (desktop_session.py), and leaves it as it found it.
"""

from __future__ import annotations

from datetime import timedelta
from functools import partial
from typing import TYPE_CHECKING

# pytest finds a fixture by its name in the test's module, so the looks areas' desktop and
# scales are brought here.
from looks_session import (  # noqa: F401
    NEAR,
    at_each_scale,
    at_scale,
    body_line,
    chose,
    desktop,
    drawn,
    on_the_beat,
    on_the_pitch,
    plate_shown,
    things_apart,
)

if TYPE_CHECKING:
    from desktop_session import Desktop, Json


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


# Each file's eyebrow, where then kind, its preview, and the facts its row doesn't say, its
# size or count and when it changed being its row's (VKCF3Z, 2026-11-08). The note comes
# before the pictures, since the facts below its lines say how far they stand below one.
PICKED = {
    "drafts": ("Folder", "lines", []),
    "archive.zip": ("ZIP", None, []),
    "notes.md": ("Markdown", "lines", ["Lines"]),
    "clip.mp4": ("MP4", "picture", ["Length"]),
    "plan.pdf": ("PDF", "picture", ["Pages"]),
    "wide.png": ("PNG", "picture", ["Dimensions"]),
}


def previewed(shown: Json, name: str, kind: str, preview: str | None) -> bool:
    """Say whether the plate is out for the file named, its kind said and its preview in.

    Its row says its size, or a folder its count, and when it changed, rather than its folder,
    even though the folder was read before the ask was shown. Its eyebrow says its folder at
    once, and its kind once file-preview has said.
    """
    drawer = shown["drawer"]
    if drawer["out"] is not True or drawer["title"] != name or None in drawer["values"]:
        return False
    if drawer["eyebrow"] != f"in ~/picked \u00b7 {kind}":
        return False
    if "\u00b7" not in (shown["selected"] or {}).get("side", ""):
        return False
    held = drawer["preview"] or {}
    if preview == "picture":
        return bool(held.get("picture"))
    if preview == "lines":
        return bool(held.get("lines"))
    return drawer["preview"] is None


def shut(shown: Json, name: str) -> bool:
    """Say whether the file named is selected with no plate out, having none to say more."""
    selected = shown["selected"] or {}
    return selected.get("name") == name and shown["drawer"]["out"] is not True


def facts_follow(drawer: Json, body: float, preview: str | None, after: float) -> float:
    """Hold a plate's facts to where its preview leaves them; say how far lines leave them.

    After lines, or no preview, they're on the card's beat. A picture stands its own height
    rather than whole lines of the pitch (VKCF3Z, 2026-11-08), so the facts after it stand as
    far below it as they do below lines, off the beat.
    """
    facts = drawer["lines"]["facts"]
    if preview == "picture":
        below = facts[0] - drawer["lines"]["previewEnd"]
        assert abs(below - after) <= NEAR, (
            f"{drawer['title']}'s facts stand {below} below its picture, not {after}: {drawer}"
        )
        return after
    assert all(on_the_beat(line, body) for line in facts), (
        f"{drawer['title']}'s facts are off the beat: {drawer}"
    )
    return facts[0] - drawer["lines"]["previewEnd"] if preview == "lines" else after


def facts_held(drawer: Json, placed: tuple[float, str | None, float], whose: str) -> float:
    """Hold a plate's facts to the pitch, below its preview and above its foot; say `after`.

    `placed` is the body's line, the kind of preview and how far facts stand below a picture.
    """
    body, preview, after = placed
    facts = drawer["lines"]["facts"]
    after = facts_follow(drawer, body, preview, after)
    assert things_apart(facts, wrapped=drawer["factsWrapped"] > 0), (
        f"{whose} facts are off the pitch: {drawer}"
    )
    if preview is not None:
        assert facts[0] > drawer["lines"]["preview"], f"{whose} facts lead: {drawer}"
    assert facts[-1] < drawer["foot"], f"{whose} facts are cut: {drawer}"
    return after


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
            # How far the facts stand below lines of preview, the note's.
            after = 0.0
            for name, (kind, preview, told) in PICKED.items():
                desktop.down_to(name)
                if preview is None and not told:
                    # Its kind is its name's ending and its folder the one listed, so a
                    # plate would only repeat its row, and none comes out.
                    desktop.launcher_until(partial(shut, name=name))
                    continue
                desktop.launcher_until(partial(previewed, name=name, kind=kind, preview=preview))
                drawer = plate_shown(desktop, scale)
                assert drawer["drawnFacts"] == told, drawer
                facts = drawer["lines"]["facts"]
                shown = drawer["lines"]["previewLines"]
                body = body_line(drawer["lines"], drawer["cardLines"])
                assert body is not None, f"at {scale} {name}'s title is off its leading: {drawer}"
                assert all(on_the_pitch(line, body) for line in shown), (
                    f"at {scale} {name}'s preview is off the pitch: {drawer}"
                )
                if facts:
                    after = facts_held(drawer, (body, preview, after), f"at {scale} {name}'s")
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
                    shown["question"] == "Replace a.md?"
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
