"""The desktop test's looks area for the launcher's vi modes.

Here the launcher is laid out at Joel's two scales, on a screen as large as the laptop's, which the
other areas' screen is too small to hold the card and its drawer at: 1.5, as the HP draws it, and 2,
as the laptop does. Y flashes the row it copies and d's ask stands on the line; visual counts its
rows, and a window arriving waits in the foot; space-right goes into the key list's drawer, its rule
on the cursor's line; and the drawer's search and visual are shot at each scale for a person to
judge. It's an area of its own, rather than more of looks, so the looks areas run side by side
rather than one after another; what they share is in looks_session.py. tests/desktop.nix runs this
through pytest inside the driver (tests/lib/vm_pytest.py), and its opening comment says what the
test proves. Every test asks for the desktop, started once a run (desktop_session.py), and leaves it
as it found it.
"""

from __future__ import annotations

import json
import tarfile
import time
from datetime import timedelta
from functools import partial
from typing import TYPE_CHECKING

import desktop_session

# pytest finds a fixture by its name in the test's module, so the looks areas' desktop and
# scales are brought here.
from looks_session import at_each_scale, at_scale, changed_from, chose, desktop, drawn, pressed  # noqa: F401
from PIL import Image, ImageChops

if TYPE_CHECKING:
    from collections.abc import Callable

    from desktop_session import Desktop, Json
    from PIL import Image

# How often and how long a sequence that waits for its second key is pressed before it
# counts as never doing what it does.
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


def test_y_flashes_its_rows_and_a_waiting_d_says_what_it_takes(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Y flashes the row it copies, and d waiting names the row it would close, tinting it."""
    machine = desktop.machine
    # A second window, beginning "Notes" as Tuesday's does, so d waiting
    # has to name its row whole to say which it takes.
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
        # key does while d waits, give way to it. It moves nothing, and
        # Escape lets it go, closing nothing.
        machine.send_chars("d")
        desktop.launcher_until(lambda shown: shown["modeWord"].startswith("d \u00b7 close"))
        shown = desktop.showing()
        assert shown["modeWord"] == "d \u00b7 close Notes for Tuesday", f"at {scale} {shown}"
        assert shown["hints"] == [], f"at {scale} {shown}"
        assert shown["tinted"] == [shown["current"]], shown
        assert len(shown["glance"]) == len(before["glance"]), f"at {scale} d cut {shown}"
        desktop.shoot(f"launcher-pending-at-{named}")
        machine.send_key("esc")
        desktop.launcher_until(
            lambda shown: (
                shown["open"] and not shown["modeWord"].startswith("d ") and shown["tinted"] == []
            )
        )
        after = desktop.showing()
        assert after["keysAt"] == before["keysAt"], f"at {scale} the keys moved"
        assert len(after["rows"]) == len(before["rows"]), f"at {scale} d closed one {after}"
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
