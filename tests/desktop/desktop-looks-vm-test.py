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
import tarfile
import time
from datetime import timedelta
from functools import partial
from itertools import pairwise
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN
from PIL import Image, ImageChops, ImageStat

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from desktop_session import Desktop, Json
    from PIL import Image
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
# How far apart a Title's lines sit when its name wraps (Theme.space.xxl), tighter than two of
# the pitch, so the name reads as one heading.
TITLE_LEADING = 32
# How far the text keeps from the plate's frame on every side, its foot included: an inset and a
# step, 4 and 16, inside the border.
INSIDE = 20
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


def after_the_title(line: float, lines: Json) -> bool:
    """Say whether a line is the first of the pitch one empty line below the title's block.

    The title's lines sit its leading apart, and its block ends on the next whole line of the
    pitch after its last, so what follows it stays on the plate's lines.
    """
    title, last = float(lines["title"]), float(lines["titleLast"])
    later = round((last - title) / TITLE_LEADING)
    if later < 0 or abs(last - title - later * TITLE_LEADING) > NEAR:
        return False
    block = -(-(PITCH + later * TITLE_LEADING) // PITCH) * PITCH
    return abs(line - (title + block + PITCH)) <= NEAR


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
        # No key is named in a mode where it does nothing: in insert the arrows move the caret,
        # so home's first row, a place, says Return alone, and in normal its arrow as well.
        foot = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
        arrowed = [word["text"] for word in foot if set(word["text"]) & {"\u2192", "\u2190"}]
        assert not arrowed, f"at {scale} insert's foot names an arrow: {arrowed}"
        desktop.normal()
        foot = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
        assert any("\u2192" in word["text"] for word in foot), f"at {scale} normal's foot: {foot}"
        desktop.insert()
        desktop.type_in("kitty")
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "program:kitty")
        card = desktop.showing()["drawer"]["cardLines"]
        assert words_on_their_names(card, scale, serif) > 0, (
            f"at {scale} kitty's row says nothing: {card}"
        )
        desktop.photograph(f"launcher-found-words-at-{scale.replace('.', '-')}")
        # The mode's word is the one word saying where his keys go, so it never gives up a
        # letter; the facts before it give way first.
        foot = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
        mode = [word for word in foot if word["text"] == "insert"]
        assert mode, f"at {scale} the foot names no mode in insert: {foot}"
        assert not mode[0]["cut"], f"at {scale} insert's word is cut: {mode[0]}"
        # In normal the field draws no caret and the foot names no mode, the
        # row's words where insert left them.
        desktop.normal()
        card = desktop.showing()["drawer"]["cardLines"]
        assert words_on_their_names(card, scale, serif) > 0, (
            f"at {scale} kitty's row says nothing in normal: {card}"
        )
        desktop.photograph(f"launcher-found-normal-at-{scale.replace('.', '-')}")
        # A count typed stands where insert's word was until its move, as showcmd shows it.
        stands_in_the_mode_words_room(desktop, "12", f"launcher-count-at-{scale}")
        shows_the_command_line(desktop, scale)
        lists_every_key(desktop, scale)
        desktop.close_launcher()
        washes_what_it_finds(desktop, scale)


def washes_what_it_finds(desktop: Desktop, scale: str) -> None:
    """Type a find that several of home's rows hold, and photograph the letters it washed.

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
    desktop.photograph(f"launcher-find-washed-at-{scale.replace('.', '-')}")
    desktop.close_launcher()


def lists_every_key(desktop: Desktop, scale: str) -> None:
    """Show every key normal takes with Space ?, as a plate in the drawer, and photograph it.

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
    desktop.photograph(f"launcher-keys-at-{scale.replace('.', '-')}")
    desktop.machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["open"] and not shown["listing"])


def shows_the_command_line(desktop: Desktop, scale: str) -> None:
    """Open the command line on kitty's row, photograph it and its menu, and let it go.

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
    desktop.photograph(f"launcher-command-at-{scale.replace('.', '-')}")
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
    desktop.photograph(f"launcher-command-chosen-at-{scale.replace('.', '-')}")
    machine.send_key("esc")
    desktop.launcher_until(
        lambda shown: shown["open"] and shown["command"] is None and shown["mode"] == "normal"
    )


def stands_in_the_mode_words_room(desktop: Desktop, typed: str, shot: str) -> None:
    """Type keys normal holds in the mode word's room, photograph them there, and let them go.

    They move nothing else in the foot, and Escape lets them go without a step back.
    """
    laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
    desktop.machine.send_chars(typed)
    desktop.launcher_until(lambda shown: shown["counted"] == typed)
    held = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
    rest = sorted(word["x"] for word in held if word["text"] != typed)
    assert rest == sorted(word["x"] for word in laid), (laid, held)
    desktop.photograph(shot.replace(".", "-"))
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
        desktop.photograph(f"launcher-pending-at-{named}")
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
        desktop.photograph(f"launcher-asks-at-{named}")
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
    photographs = desktop.values["photographs"]
    if photographs:
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
    if photographs:
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


def test_space_right_goes_into_the_drawer_its_rule_on_the_cursors_line(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Go into the key list's drawer at each of Joel's scales, move there, and photograph it.

    Gone into, the drawer holds the keys: its frame takes the accent and its cursor's line the
    rule in its left margin. Five lines down and four letters along, it's photographed with the
    foot at the laptop's size, for a person to judge, and Space with the left arrow comes back.
    """

    def relisted() -> None:
        closed_from_the_list(desktop)
        listed_keys(desktop)

    for scale in at_each_scale:
        at_scale(desktop, scale)
        listed_keys(desktop)
        # The arrow alone steps into the list's row, so the list is shown again.
        pressed_until(
            desktop, "spc right", lambda shown: shown["drawer"]["cursor"] is not None, relisted
        )
        line = int(desktop.showing()["drawer"]["cursor"]["line"])
        pressed(desktop, "5 j l l l l")
        desktop.launcher_until(partial(moved_to, line + 5))
        drawn = desktop.showing()["drawer"]
        assert drawn["keys"], f"at {scale} the drawer doesn't hold the keys: {drawn}"
        assert drawn["cursor"]["rule"]["width"] > 0, f"at {scale} no rule: {drawn['cursor']}"
        desktop.photograph(f"launcher-drawer-in-at-{scale.replace('.', '-')}")
        # Space ? in it lists the drawer's own keys there; the pair come apart leaves ? alone,
        # a search back, which Escape lets go.
        pressed_until(
            desktop,
            "spc shift-0x35",
            lambda shown: shown["drawer"]["title"] == "The Drawer's Keys",
            lambda: pressed(desktop, "esc"),
        )
        desktop.photograph(f"launcher-drawer-listed-at-{scale.replace('.', '-')}")
        # The arrow alone moves the cursor a letter, which leaves nothing to put back.
        pressed_until(
            desktop,
            "spc left",
            lambda shown: shown["mode"] == "normal" and not shown["drawer"]["keys"],
            lambda: None,
        )
        closed_from_the_list(desktop)


def test_the_drawers_search_and_visual_are_photographed_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Search the key list's drawer, choose in it and copy, photographing each at both scales.

    What's typed after / is washed as it's typed; v chooses letters and V lines, the mode word
    counting them, "visual line \u00b7 3"; and y flashes what it took. Each is kept for a person
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
            desktop, "spc right", lambda shown: shown["drawer"]["cursor"] is not None, relisted
        )
        pressed(desktop, "slash d o w n")
        desktop.launcher_until(lambda shown: shown["drawer"]["search"]["typing"] == "down")
        desktop.photograph(f"launcher-drawer-washed-at-{named}")
        pressed(desktop, "ret")
        desktop.launcher_until(partial(landed_on, "Down"))
        pressed(desktop, "v l l l l")
        desktop.launcher_until(partial(chose, "letters", 5))
        desktop.photograph(f"launcher-drawer-letters-at-{named}")
        pressed(desktop, "esc shift-v j j")
        desktop.launcher_until(partial(chose, "lines", 3))
        desktop.photograph(f"launcher-drawer-lines-at-{named}")
        caught_flash(desktop, f"launcher-drawer-flash-at-{named}")
        pressed_until(
            desktop,
            "spc left",
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
                if desktop.values["photographs"]:
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
        # One empty line below the title's block.
        assert after_the_title(lines["words"], lines), drawer
        assert drawer["eyebrow"].startswith("Desktop \u00b7 "), drawer
        desktop.photograph(f"launcher-plate-at-{scale.replace('.', '-')}")
        desktop.close_launcher()


def test_a_plates_facts_lie_on_the_rows_lines_and_halfway(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """A time server's plate has its facts from one empty line below the title, on the pitch."""
    # chrony keeps the clock by the VM's own time server (tests/desktop.nix), whose plate
    # holds every figure chronyd has of it.
    desktop.machine.wait_until_succeeds(
        "chronyc -c -N sources | grep -q '^^,[*],time.harbour.test,'",
        timeout=timedelta(seconds=90),
    )
    for scale in at_each_scale:
        at_scale(desktop, scale)
        desktop.answer("time")
        desktop.step_in()
        desktop.launcher_until(lambda shown: "Servers" in shown["rows"])
        desktop.down_to("Servers")
        desktop.step_in()
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
        # the facts start one empty line below its block, on the pitch.
        facts = drawer["lines"]["facts"]
        assert after_the_title(facts[0], drawer["lines"]), drawer
        assert all(
            later > earlier and on_the_pitch(later, earlier) for earlier, later in pairwise(facts)
        ), f"at {scale} a fact is off the pitch: {drawer}"
        # Its name is the title, so it isn't said again under it; every other name is whole;
        # and what doesn't fit above the foot's inset waits for the drawer's keys.
        assert "Name" not in drawer["drawnFacts"], drawer
        assert drawer["labelsWhole"], f"at {scale} a fact's name is broken: {drawer}"
        # Each fact is its name and its value on one line, or its name with its value under
        # it, so a value too long beside its name, as an address or a reach, never wraps
        # there or breaks a word.
        assert drawer["besideOnOneLine"], f"at {scale} a value wraps beside its name: {drawer}"
        assert drawer["valuesWhole"], f"at {scale} a fact's value breaks a word: {drawer}"
        assert drawer["more"] or facts[-1] < drawer["foot"], (
            f"at {scale} a fact past the foot can't be scrolled to: {drawer}"
        )
        assert drawer["height"] - drawer["foot"] >= INSIDE - NEAR, (
            f"at {scale} the last line runs into the plate's foot: {drawer}"
        )
        desktop.photograph(f"launcher-plate-facts-at-{scale.replace('.', '-')}")
        desktop.close_launcher()
        # The zone's plate, under its eyebrow: a value that won't sit beside its name on one
        # line, as "none before 2046", takes the plate's width under it instead, so no fact
        # stands taller than two lines and none reads a word a line.
        desktop.answer("time")
        desktop.step_in()
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
        assert all(later - earlier <= 2 * PITCH + NEAR for earlier, later in pairwise(facts)), (
            f"at {scale} a value stands a word a line beside its name: {drawer}"
        )
        assert drawer["besideOnOneLine"], f"at {scale} a value wraps beside its name: {drawer}"
        desktop.photograph(f"launcher-plate-zone-at-{scale.replace('.', '-')}")
        desktop.close_launcher()


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
            desktop.photograph(f"file-picker-one-file-at-{scale.replace('.', '-')}")
            # Escape leaves insert, then refuses the ask from the folder it opened in.
            desktop.close_launcher()
            machine.wait_until_succeeds(f"grep -qx 1 {chose}", timeout=timedelta(seconds=60))
    finally:
        desktop.as_account(f"rm -rf {folder}")


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
                    and shown["current"] == -1
                    and (shown["selected"] or {}).get("name") == "a.md"
                    and "a.md" not in shown["rows"]
                )
            )
            desktop.photograph(f"file-picker-save-name-at-{shot}")
            desktop.normal()
            desktop.photograph(f"file-picker-save-name-normal-at-{shot}")
            machine.send_key("ret")
            desktop.launcher_until(
                lambda shown: (
                    shown["question"] == "Replace a.md? (y/n)"
                    and shown["takesAnswer"]
                    # Read whole, the path giving way, never cut.
                    and shown["questionWhole"]
                )
            )
            desktop.photograph(f"file-picker-save-replace-at-{shot}")
            machine.send_chars("n")
            desktop.launcher_until(lambda shown: shown["question"] == "" and shown["open"])
            # Escape, from normal on the name, refuses the ask from the folder it opened in.
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
            desktop.photograph(f"file-picker-nothing-gathered-at-{said}")
            ran(desktop, "arga")
            desktop.down_to("more")
            desktop.step_in()
            desktop.launcher_until(lambda shown: shown["path"][-1:] == ["~/gathering/more"])
            desktop.down_to("d.md")
            ran(desktop, "arga")
            desktop.launcher_until(
                lambda shown: (
                    shown["placeSays"] == "2 gathered"
                    and selected_is(shown, name="d.md", told="\u21b5 open 2")
                )
            )
            desktop.photograph(f"file-picker-two-gathered-at-{said}")
            ran(desktop, "args")
            desktop.launcher_until(lambda shown: "Forget Them" in shown["rows"])
            # The cursor stands on the first gathered, and Return still answers both.
            desktop.launcher_until(partial(selected_is, name="a.md", told="\u21b5 open 2"))
            gathered_is_in_sight(desktop.showing(), scale)
            desktop.photograph(f"file-picker-inside-gathered-at-{said}")
            machine.send_key("left")
            desktop.launcher_until(lambda shown: shown["rows"][-1:] == ["Copy the Path"])
            desktop.down_to("Copy the Path")
            desktop.photograph(f"file-picker-copy-last-at-{said}")
            # Escape climbs back to the folder it opened in, then refuses the ask there.
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
            desktop.photograph(f"file-picker-sort-menu-at-{said}")
            machine.send_key("ctrl-u")
            machine.send_key("esc")
            desktop.launcher_until(lambda shown: shown["command"] is None)
            for line, listed in ORDERS.items():
                ran(desktop, line)
                desktop.launcher_until(partial(listed_from_the_top, listed=listed))
                shot = line.replace(" ", "-").replace("!", "-turned")
                desktop.photograph(f"file-picker-{shot}-at-{said}")
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
            # Short of room, a side gives way whole, the tree, the address, then the state,
            # before a name gives up a letter, and what it cost stays, never slid from its left.
            costs = {"TESTQA": "$14.20 today in all", "TESTQD": "$5.10 today"}
            for row in rows:
                one = address_of(row)
                side_gives_way_whole(row, titles[one], costs.get(one, ""), scale)
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
        # And it leads the words drawn after its name, since the note is only in the foot.
        drawn = [word["text"] for word in row["words"]]
        assert any(text.startswith("under TESTQC") for text in drawn), drawn
        desktop.photograph("launcher-agents-typed-flat")
        desktop.close_launcher()
    finally:
        unorchestrated(desktop)
