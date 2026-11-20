"""The margin's cat sits over the column the keys are in, on niri, and follows them.

modules/quickshell/checks/margin.nix runs this through pytest in tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run, as the graphical account has it with the cat's
module (modules/margin-cat). Where over a column it sits, beside the margin's marks or nowhere,
is held in Qt's engine (modules/margin-cat/tests/margin-cat.qml), as are a walk's beats; here,
only that niri's real layout reaches it, that it rests two seconds before walking over a step
every frame with none dropped, and never for a tap along the columns, each walk's steps kept as
the check's result, that a walk holds where it got to while the launcher is up, drawing nothing,
and one set off under it waits for it to close, that its layer is one taking no keys, that it
goes in fullscreen, where the gap it sits in is gone, that it dozes off after a minute with no
keys and stretches as the screen unlocks, that niri draws it in whole pixels at 1.5 and at 2,
each screen kept as the check's result, and that a move between workspaces has it ride the one
it leaves off the screen and come in beyond, a second move cutting it short holding on from
where the first got to, that on a workspace with no windows it sits on the floor until one opens
there, that a row of a column closed under it has it fall at once, and that a pointer moved onto
it leans it in while one it came to under doesn't, a click beside it reaching the window.

Long because every case shares the one shell, started with the cat's own doze and scenes, and
the readers of what it reports; the moves between workspaces could go to a check of their own,
which would start a second shell and carry those readers twice.
"""

from __future__ import annotations

import itertools
import json
import math
import os
import struct
import time
import zlib
from collections import Counter
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING, TypeGuard

import launcher_on_niri
import pytest
from launcher_on_niri import placed

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from launcher_on_niri import Json, Shell

# What the cat says each time it's placed (modules/margin-cat/MarginCatScreen.qml).
SITS = "sits cat "
# What it says as the workspace changes under it: whether it moves, or what kept it still.
MOVES = "moves cat "
# How long the cat's last frame, or its hearing the lock's word, may take to land.
LANDS = 2.0
# How long with no keys draws none of its frames, how long after the last key it may take to
# be heard asleep, its 45 s doze here (modules/quickshell/checks/margin.nix) and some, and the
# frames that curling up may draw.
IDLE, DOZES, POSED = 30.0, 60, 2
# How long the cat's stretch holds as the lock lets go: about half a second, and no less than
# a glance, nor so long that it's in the way of the keys pressed next.
STRETCHES = (0.3, 1.0)
# How long the keys rest on a column before the cat walks there
# (modules/margin-cat/MarginCatScreen.qml), and the longest the shell may take to sync and
# render one of its frames, a quarter of a frame at 60, so its own work never costs one.
RESTS = 2.0
# The shortest of a walk's beats, its crouch, in ms: a frame of the nested niri's longer than
# it may hide it.
SHORTEST_BEAT = 110
DRAWS = 4.0
WALKED = "walked cat "
# The fewest steps a second its walk may take, the nested screen drawing at 60.
SMOOTH = 50
# The farthest, in the cat's own size, a step of a move may take it between two frames
# the shell says, a builder's dropped frames included.
STEPS_ON = 2
# How long, in seconds, the launcher is held open over a walk, which steps none of it then.
HELD = 2.0
# The farthest, in ms, a held walk's clock may go on between its last step before the launcher
# and its first after: half the hold, where a clock run on under the launcher goes on by all of
# it. A frame or so is what
# it takes, but a loaded builder's nested niri has taken up to 160 ms between two (2026-11-19),
# and a few such frames mustn't fail it.
HELD_AHEAD = HELD * 1000 / 2
# The shell's windows that sit, like the cat, on the launcher's screen and in view whatever the
# columns are, by the names Frames.qml counts them under: a frame both drew is the whole
# scene's, not the cat's. Either may draw for itself, as a notice comes, so the floor is the
# fewer. The wallpaper is left out: in one such redraw it drew none while the rest drew 6.
STILL_WINDOWS = ("notices", "talk-line")
# How long, in seconds, the cat may take to tell of its walk once it sits: it waits up to 2 s
# for niri's answers on the frames still on their way (MarginCatShown.js's ANSWERS).
TOLD = 5.0
# How long niri may take to say its layers have changed.
LAYERED = 10.0
# How long a pointer's move or click may take to move niri's focus.
FOCUSES_BY = 10.0
# How long a window's layout must hold for it to have stopped sliding to its place.
STILL_FOR = 0.5
# How far inside a window's corner, in logical pixels, the pointer goes, past its border.
INSIDE = 24
# How far from that point, down and right in logical pixels, a float is put out of its way.
CLEAR = 96
POLL = 0.05
# Looks running at a sitting cat, each begun well after it last said anything, that find it
# drawn otherwise than it said: a cat that sat still that long and is still wrong is a fault.
JUDGING_LOOKS = 2
# The longest a wait for the cat to sit and be judged may last, for a cat that never sits.
SETTLES_BY = 90.0
# How far, in logical pixels, the cat's feet may be from a window's top for it to sit flush.
FLUSH = 0.5
# How far each channel may be from the cat's hue for a pixel to be called near it, when
# looking for a cat drawn somewhere other than where it said.
NEAR = 24
# How far up and left the pointer is sent to put it in the screen's corner, from which a move
# is to a point on the screen.
TO_THE_CORNER = -10000
# How long a pointer resting on the cat takes to pet it, and how long its lean holds after the
# pointer goes (modules/margin-cat/MarginCat.js), in seconds.
PETS, PETTED = 0.3, 0.6
# How long the cat must have sat still before a pointer coming onto it can pet it
# (MarginCat.SETTLED, 200 ms), with a frame's margin on a loaded builder, in seconds.
PETTED_FROM = 0.3
# What a PNG's header says of niri's screenshot: eight bits a channel, in red, green and
# blue, with alpha or without.
BYTE, RGB, RGBA = 8, 2, 6


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the account's whole shell once its cat has said where it sits, started once.

    Every case reads the cat's reports, so none starts before the cat's surface is loaded.
    Once they're done, nothing the cat did in any of them may have looped a binding or read
    a place left unworked: Joel's shell logged loops on `column` five times from 16:52 to
    16:59 on 2026-11-12, as a walk's handlers set where it sat while its column was said.
    """
    with launcher_on_niri.running() as started:
        heard(started, lambda _: True, 0)
        yield started
        faults = [
            line
            for line in started.log.read_text(errors="replace").splitlines()
            if ("Binding loop" in line or "TypeError" in line) and "MarginCat" in line
        ]
        assert not faults, f"the cat looped or read nothing: {faults}"


@pytest.fixture(autouse=True)
def awake(shell: Shell) -> None:
    """Wake the cat with a key before each case, since nobody types here and it dozes off."""
    since = shell.mark()
    shell.send_key("F24")
    if dozing(shell):
        heard(shell, lambda said: not asleep(said), since)


@pytest.fixture(autouse=True)
def launcher_closed_after(shell: Shell) -> Iterator[None]:
    """Close the launcher after each case, so one failing with the cat held fails alone."""
    yield
    shell.launcher_left_closed()


def dozing(shell: Shell) -> bool:
    """Say whether the cat's newest report has it asleep."""
    said, _ = shell.said_since(0)
    last = next((report for report in reversed(said) if report.startswith(SITS)), None)
    return last is not None and asleep(last[len(SITS) :])


def asleep(said: str) -> bool:
    """Say whether a report has the cat asleep where it sits."""
    at = placed(said)
    return at is not None and bool(at.get("asleep"))


def sits_over(
    shell: Shell, window: Json, since: int, timeout: timedelta = launcher_on_niri.WAITING
) -> None:
    """Wait until the cat says it sits on this window's column, over its middle.

    It sits a cell of air above the column's top, so its feet stand clear of the border. Each
    report is held to the window and column it names itself, never to where the windows are
    by the time it's read, since niri lays them out again as others open and close.
    """

    def over(said: str) -> bool:
        at = placed(said)
        if at is None or at.get("window") != window["id"]:
            return False
        column = json.loads(said.split(" over ", 1)[1])
        if column is None:
            return False
        left, top, width = column["left"], column["top"], column["width"]
        middle = at["x"] + at["size"] / 2
        return bool(left <= middle <= left + width and at["y"] + at["size"] + at["air"] == top)

    heard(shell, over, since, timeout)


def heard(
    shell: Shell,
    test: Callable[[str], bool],
    since: int,
    timeout: timedelta = launcher_on_niri.WAITING,
) -> None:
    """Wait for the cat to report what passes the test, or fail saying all it reported."""
    try:
        shell.reported(SITS, test, since, timeout)
    except AssertionError:
        said, _ = shell.said_since(0)
        told = (SITS, MOVES)
        sat = "\n".join(report for report in said if report.startswith(told)) or "nothing"
        failed = f"the cat never sat as wanted; it said, since the shell started:\n{sat}"
        raise AssertionError(failed) from None


def cat_layers(shell: Shell) -> list[Json]:
    """Give each layer surface niri lists as the cat's."""
    layers = json.loads(shell.as_account("niri msg --json layers"))
    return [layer for layer in layers if layer["namespace"] == "cat"]


def layered(shell: Shell, wanted: int) -> list[Json]:
    """Wait until niri lists this many of the cat's layers, and give them."""
    deadline = time.monotonic() + LAYERED
    shown = cat_layers(shell)
    while len(shown) != wanted:
        assert time.monotonic() < deadline, f"niri lists {len(shown)} of the cat, not {wanted}"
        time.sleep(POLL)
        shown = cat_layers(shell)
    return shown


def steps(shell: Shell, since: int) -> list[Json]:
    """Give each step of a walk the cat said since the mark, in the order it took them."""
    said, _ = shell.said_since(since)
    reports = [placed(report[len(SITS) :]) for report in said if report.startswith(SITS)]
    return [at for at in reports if at is not None and at.get("walk")]


def unseen_only_between_frames(walked: list[Json], beats: list[str]) -> list[str]:
    """Give the beats a walk missed of its own, out of order or with no frame to hide them.

    The nested niri draws in software, and one frame in a walk can take longer than a short
    beat lasts: the crouch's 110 ms fell between two frames 121 ms apart (2026-11-12), so a
    beat may go unseen only where the steps either side of it are farther apart than that.
    """
    took: list[tuple[str, float, float]] = []
    for step in walked:
        beat, t = step["walk"]["beat"], step["walk"]["t"]
        if took and took[-1][0] == beat:
            took[-1] = (beat, took[-1][1], t)
        else:
            took.append((beat, t, t))
    wrong: list[str] = []
    at = 0
    for seen, (beat, starts, _) in enumerate(took):
        if beat not in beats[at:]:
            return [f"{beat} out of order"]
        skipped = beats[at : beats.index(beat, at)]
        gap = starts - took[seen - 1][2] if seen else 0
        wrong += [f"{missed} unseen in {gap:.0f} ms" for missed in skipped if gap < SHORTEST_BEAT]
        at = beats.index(beat, at) + 1
    return wrong + [f"{missed} never" for missed in beats[at:]]


def walking(said: str) -> bool:
    """Say whether a report is a step of a walk."""
    at = placed(said)
    return at is not None and bool(at.get("walk"))


def flush_on_a_lower_row(window: Json) -> Callable[[str], bool]:
    """Say whether a report has the cat sat still, flush on the top of `window` below a row."""

    def flush(said: str) -> bool:
        at = placed(said)
        column = json.loads(said.split(" over ", 1)[1])
        return bool(
            at is not None
            and not at.get("walk")
            and at.get("window") == window["id"]
            and column is not None
            and column["top"] > at["size"] * 4
            and abs(at["y"] + at["size"] + at["air"] - column["top"]) < FLUSH
        )

    return flush


def test_the_cat_walks_over_a_frame_at_a_time_once_the_keys_rest(shell: Shell) -> None:
    """Two seconds after the keys move to another column, it walks there, a step each frame.

    It steps at the screen's rate, and its own work never costs a frame: each takes the shell
    a quarter of one to draw, and niri shows every one (Joel, 2026-11-12: "we need proper
    smooth animations with character to them"). How far apart niri shows them is the nested
    screen's, drawn in software under a second compositor, so it's kept, not judged; Joel's
    journal says it of his own screen. Its beats come in the order its way has them, the gap
    between the two columns leapt (Joel, 2026-11-13: "when crossing across gaps between
    windows, the kitty should jump"), and it goes in both its strides, never stepping back.
    Its steps and how niri showed them are kept as the check's result, for a strip of the walk.
    """
    since = shell.mark()
    first = shell.opened("a column to walk to")
    second = shell.opened("a column to walk from")
    sits_over(shell, second, since)
    since = shell.mark()
    moved = time.monotonic()
    shell.as_account("niri msg action focus-column-left")
    heard(shell, walking, since)
    rested = time.monotonic() - moved
    # The walk that brought it here tells of itself once niri has answered for its last frames,
    # which can be after the key, though always before this walk's first step; so this walk's
    # tellings are read only from its first step on.
    setoff = shell.mark()
    sits_over(shell, first, since)
    shell.reported(WALKED, lambda _: True, setoff, timedelta(seconds=TOLD))
    walked = steps(shell, since)
    said, _ = shell.said_since(setoff)
    told = [
        json.loads(line[len(WALKED) :].split(" ", 1)[1]) for line in said if line.startswith(WALKED)
    ]
    kept = Path(os.environ["out"])  # noqa: SIM112, the check's own result, as Nix names it
    kept.mkdir(exist_ok=True)
    (kept / "walk.json").write_text(json.dumps({"steps": walked, "presented": told}))
    assert rested >= RESTS, f"the cat set off {rested:.2f} s after the key, before resting"
    beats = [step["walk"]["beat"] for step in walked]
    way = walked[0]["walk"]["beats"]
    assert "leaps" in way, f"the cat's way leapt no gap: {way}"
    assert unseen_only_between_frames(walked, way) == [], f"the cat's beats came as {beats}"
    times = [step["walk"]["t"] for step in walked]
    gaps = sorted(after - before for before, after in itertools.pairwise(times))
    frame = gaps[len(gaps) // 2]
    assert frame <= 1000 / SMOOTH, f"the cat walked a step every {frame} ms: {times}"
    assert len(told) == 1, f"the cat said how its walk was shown {len(told)} times: {told}"
    (shown,) = told
    assert shown["frames"] > 0, f"niri showed none of the walk's frames: {shown}"
    assert shown["unseen"] == 0, f"niri never showed {shown['unseen']} of its frames: {shown}"
    # Each frame it draws asks niri when it's shown, so a frame asked after the next had begun
    # can't halve what the journal says of Joel's screen; it tells of the walk once niri has
    # answered for the frames still on their way as it sat, so every one it began is heard of.
    answered = shown["frames"] + shown["unseen"]
    assert answered >= shown["begun"], (
        f"niri was asked of {answered} of the {shown['begun']} frames it drew: {shown}"
    )
    # The typical frame is the shell's own cost; the slowest is a loaded builder's too, which
    # stretched one past 25 ms with the cat's code unchanged (2026-11-14), so it's only said.
    assert shown["typical"] <= DRAWS, f"a frame took the shell {shown['typical']} ms: {shown}"
    # Each stride stepped by its ground is MarginCat.js's, proven frame by frame in Qt's engine
    # (margin-cat.qml); here a frame can come late enough to skip a stride, so this asks only
    # that it goes only in its strides, more than one of them reaching the screen, the way it
    # faces.
    going = [step for step in walked if step["walk"]["beat"] == "goes"]
    poses = {step["walk"]["pose"] for step in going}
    walks = {"walkingA", "walkingB", "walkingC", "walkingD"}
    assert poses <= walks, f"the cat went in the poses {poses}"
    assert len(poses) > 1, f"the cat went in one stride only: {poses}"
    ahead = [
        (after["x"] - before["x"]) * after["walk"]["facing"]
        for before, after in itertools.pairwise(going)
    ]
    assert min(ahead) >= 0, f"the cat stepped back as it went: {[s['x'] for s in going]}"


def test_tapping_along_the_columns_never_sets_it_walking(shell: Shell) -> None:
    """Tapping h then l, back within the rest, leaves the cat sitting where it was."""
    since = shell.mark()
    shell.opened("a column tapped past")
    sat = shell.opened("a column the cat stays on")
    sits_over(shell, sat, since)
    since = shell.mark()
    shell.as_account("niri msg action focus-column-left")
    time.sleep(RESTS / 4)
    shell.as_account("niri msg action focus-column-right")
    time.sleep(RESTS + 1)
    walked = steps(shell, since)
    assert not walked, f"tapping set the cat walking: {walked[:3]}"


def test_a_walk_holds_where_it_got_to_while_the_launcher_is_up(shell: Shell) -> None:
    """The launcher opened mid-walk holds the cat where it got to, and it goes on as it closes.

    Joel's keys are in the launcher then, and each frame the cat draws is one the shell draws
    beside theirs: on his laptop it drew 10 to 19 minutes an hour as he worked (2026-11-19).
    It's held rather than cut short or skipped ahead, so it's never put anywhere it didn't get
    to (WCJQSG 2bd7c7c6): its first step once the launcher closes is a step from its last.
    """
    first = three_columns(shell, "a held walk")
    since = shell.mark()
    # Two columns over, its walk lasts well past the launcher's opening.
    shell.as_account("niri msg action focus-column-left")
    shell.as_account("niri msg action focus-column-left")
    heard(shell, walking, since)
    toggled = shell.mark()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    # The card holds it from its first frame, which comes before the launcher says it's ready,
    # so it steps and draws nothing from here: at most the one frame already on its way.
    held = shell.mark()
    shell.as_account(f"{counting(shell)} start")
    time.sleep(HELD)
    drew, drawn = cat_frames_of_its_own(shell)
    under, _ = shell.said_since(held)
    before = steps(shell, since)
    said, _ = shell.said_since(since)
    # A walk run on under the card is the cat's fault, said before the case's timing is, since
    # such a walk ends under the card as one done before it would; a step it says it took held
    # is too, from the card's rise, before the launcher said it was ready.
    held_steps = [step for step in steps(shell, toggled) if step.get("stilled")]
    assert not held_steps, f"the cat stepped while held under the launcher: {held_steps[:3]}"
    sat = [report for report in under if report.startswith(SITS)]
    assert not sat, f"the cat walked on under the launcher: {sat[:3]}"
    sat_last = [placed(report[len(SITS) :]) for report in said if report.startswith(SITS)][-1:]
    # A walk done before the card rose has nothing to go on with, so the case says so here
    # rather than as a sit never seen once the launcher closes.
    assert not sat_last or (sat_last[0] or {}).get("walk"), (
        f"the walk was done before the launcher opened, the case's timing: {sat_last}"
    )
    shell.close_launcher()
    shell.launcher_closed()
    sits_over(shell, first, held)
    after = steps(shell, held)
    # The walk that brought it to the third column tells of itself before this one's first
    # step, so this one's telling is read from its walking on.
    told = walked_report(shell, toggled)
    assert drew <= 1, f"the cat drew {drew} frames of its own under the launcher: {drawn}"
    assert before, "the cat took no step before the launcher opened"
    last = before[-1]
    assert after, "the cat took no step once the launcher closed, its walk done before it opened"
    # Its walk's clock goes on from where it was held, rather than the walk set off again or
    # its clock run on under the launcher.
    ahead = after[0]["walk"]["t"] - last["walk"]["t"]
    assert 0 <= ahead <= HELD_AHEAD, (
        f"the cat's walk went on at {after[0]['walk']['t']} ms from {last['walk']['t']}"
    )
    gone = math.hypot(after[0]["x"] - last["x"], after[0]["y"] - last["y"])
    assert gone <= STEPS_ON * last["size"], f"the cat went on from {after[0]}, having got to {last}"
    held_once(told)


def test_a_walk_set_off_under_the_launcher_waits_for_it_to_close(shell: Shell) -> None:
    """A walk whose rest ends under the launcher's card takes no step until the card goes.

    The keys that set it off came just before the launcher opened, so its walk begins while the
    card is up and is held from its first step, which it takes once the card goes. The keys
    move niri's focus, which goes to the launcher as it opens, so they come first, with the
    toggle in the same command to keep the card's rise well inside the rest.
    """
    first = three_columns(shell, "a walk set off under the launcher")
    since = shell.mark()
    keyed = time.monotonic()
    left = "niri msg action focus-column-left"
    shell.as_account(f"{left} && {left} && {shell.ipc('toggle')}")
    shell.launcher_ready()
    ready = time.monotonic() - keyed
    shell.as_account(f"{counting(shell)} start")
    early = steps(shell, since)
    # A step it says it took held is the cat's fault; one before the card is the case's timing.
    held_steps = [step for step in early if step.get("stilled")]
    assert not held_steps, f"the cat stepped while held under the launcher: {held_steps[:3]}"
    assert not early, (
        f"the launcher was ready {ready:.2f} s after the keys were sent, its rest of {RESTS} s "
        f"run out before the card rose, the case's timing and not the cat's: {early[:3]}"
    )
    time.sleep(RESTS + HELD)
    drew, drawn = cat_frames_of_its_own(shell)
    under = steps(shell, since)
    # The walk before tells of itself as this one sets off, under the card, so this one's
    # telling is read from here.
    closing = shell.mark()
    shell.close_launcher()
    shell.launcher_closed()
    sits_over(shell, first, since)
    after = steps(shell, since)
    told = walked_report(shell, closing)
    assert not under, f"the cat walked under the launcher: {under[:3]}"
    # Its walk set off under the card is started and held at once, and the start has asked
    # its window for the one frame niri draws it at its seat (2026-11-19, round 6's run).
    assert drew <= 1, f"the cat drew {drew} frames of its own under the launcher: {drawn}"
    assert after, "the cat took no step once the launcher closed"
    assert after[0]["walk"]["t"] <= HELD_AHEAD, (
        f"the cat's first step came {after[0]['walk']['t']} ms into its walk"
    )
    # Held once says its walk was set off under the card, not after it went.
    held_once(told)


def walked_report(shell: Shell, since: int) -> Json:
    """Wait for the cat to tell of a walk since the mark, and give the one it told of."""
    shell.reported(WALKED, lambda _: True, since, timedelta(seconds=TOLD))
    said, _ = shell.said_since(since)
    told: list[Json] = [
        json.loads(line[len(WALKED) :].split(" ", 1)[1]) for line in said if line.startswith(WALKED)
    ]
    assert len(told) == 1, f"the cat told of {len(told)} walks: {told}"
    return told[0]


def held_once(told: Json) -> None:
    """Check a walk told of was held once, with the hold left out of its frames and gaps."""
    assert told["held"] == 1, f"the cat said its walk was held {told['held']} times: {told}"
    answered = told["frames"] + told["unseen"]
    # Every frame it presented was begun, bar the one on its way as the card rose, whose
    # beginning falls in the hold, so a walk that left out more than its hold says fewer.
    assert told["frames"] <= told["begun"] + 1, (
        f"the walk presented {told['frames']} frames, but said it began {told['begun']}: {told}"
    )
    assert answered >= told["begun"], (
        f"niri was asked of {answered} of the {told['begun']} frames it drew: {told}"
    )
    assert told["slowest"] < HELD_AHEAD, f"the walk's gaps kept the hold in them: {told}"


def three_columns(shell: Shell, named: str) -> Json:
    """Open three columns with the launcher warm and closed, the cat over the last of them.

    A cold launcher's first opening can take longer than a walk lasts, so the shell is left to
    warm and the launcher opened once first, and a walk is held by the launcher's card, not its
    first load. Gives the first column.
    """
    launcher_on_niri.warmed(shell)
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.close_launcher()
    shell.launcher_closed()
    since = shell.mark()
    first = shell.opened(f"a column {named} goes to")
    shell.opened(f"a column {named} goes by")
    third = shell.opened(f"a column {named} sets off from")
    sits_over(shell, third, since)
    # Its last frame over the new column lands a moment after it says it sits there.
    time.sleep(LANDS)
    return first


def test_the_cat_takes_no_keys_on_a_layer_of_its_own(shell: Shell) -> None:
    """It's under the windows, taking no keys, so typing goes where it went before.

    The margin, the launcher and notices stay over it, on the overlay layer, so a mark's ink
    lies over its paws as it perches there, and it never hides one (VKCF3Z, 3d799798).
    """
    shell.opened("a column under the cat")
    on_layer(shell, "Bottom")
    (layer,) = layered(shell, 1)
    over = json.loads(shell.as_account("niri msg --json layers"))
    under = [
        (one["namespace"], one["layer"])
        for one in over
        if one["namespace"] in {"margin", "launcher", "notifications"} and one["layer"] != "Overlay"
    ]
    assert not under, f"these are under the overlay layer, beside the cat: {under}"
    assert layer["keyboard_interactivity"] == "None", (
        f"the cat takes keys: {layer['keyboard_interactivity']}"
    )


def on_layer(shell: Shell, wanted: str) -> None:
    """Wait until niri lists the cat's one layer surface on this layer.

    A failure gives the cat's last report, which says whether the shell asked for it over
    the windows, and niri's word on the window it sits on, so the cat's rule and the
    surface niri took tell themselves apart.
    """
    deadline = time.monotonic() + LAYERED
    (layer,) = layered(shell, 1)
    while layer["layer"] != wanted:
        if time.monotonic() >= deadline:
            said, _ = shell.said_since(0)
            last = next((one for one in reversed(said) if one.startswith(SITS)), "")
            at = placed(last[len(SITS) :]) if last else None
            seat = [one for one in shell.windows() if at and one["id"] == at.get("window")]
            assert layer["layer"] == wanted, f"on {layer['layer']}: {last}; its seat {seat}"
        time.sleep(POLL)
        (layer,) = layered(shell, 1)


def seated(shell: Shell, since: int) -> tuple[Json, Json]:
    """Give where the cat's newest report since the mark has it sit, and the column under it."""
    said, _ = shell.said_since(since)
    last = next(report for report in reversed(said) if report.startswith(SITS))
    at = placed(last[len(SITS) :])
    assert at is not None, f"the cat said it's nowhere: {last}"
    return at, json.loads(last.split(" over ", 1)[1])


def inks(shell: Shell, at: Json, wanted: Callable[[int], bool]) -> int:
    """Wait until the cat's box holds as much of its ink as wanted, and give how much."""
    scale = json.loads(shell.as_account("niri msg --json outputs"))["winit"]["logical"]["scale"]
    deadline = time.monotonic() + LAYERED
    drawn = inked(boxed(shell, at, scale), at["hue"])
    while not wanted(drawn) and time.monotonic() < deadline:
        time.sleep(POLL)
        drawn = inked(boxed(shell, at, scale), at["hue"])
    return drawn


def shows_whole(shell: Shell, since: int, window: Json) -> tuple[int, int]:
    """Wait until the cat's box holds all the ink its newest drawing has, and give both.

    Each look is held to the report newest at the time, since a cat that has just sat can
    settle into a drawing a pixel different from the one it sat down in (2026-11-16: 36 of
    37 showed, the 37 read from a report before its pose changed). Only a report with it sat
    on `window` is judged, so a cat that left it is never judged where it went.
    """
    scale = json.loads(shell.as_account("niri msg --json outputs"))["winit"]["logical"]["scale"]
    deadline = time.monotonic() + LAYERED
    while True:
        at, _ = seated(shell, since)
        on = at.get("window") == window["id"]
        whole = at["lit"] * at["cell"] ** 2
        drawn = inked(boxed(shell, at, scale), at["hue"]) if on else 0
        if (on and drawn == whole) or time.monotonic() >= deadline:
            assert on, f"the cat left its window as it was judged: {at}"
            return drawn, whole
        time.sleep(POLL)


def test_windows_cover_it_but_a_floating_one_holds_it_over_them(shell: Shell) -> None:
    """Under the windows, one passing in front of it covers it, never wearing it on its face.

    That holds however late the shell hears niri, where on top a loaded machine showed it on
    a sliding window's face (VKCF3Z, 3fe38ef5). Standing on a floating window, which sits in
    front of the tiled ones, it's over them, or the tiled window behind would hide it (Joel,
    2026-11-13 18:59), and back under them once it leaves.
    """
    # The lower of two rows, so it sits in the gap between them, with room above it for a
    # floating window to cover it.
    upper = shell.opened("a row over the one a floating window goes in front of")
    since = shell.mark()
    tiled = shell.opened("a row a floating window goes in front of")
    shell.as_account("niri msg action consume-or-expel-window-left")
    sits_over(shell, tiled, since)
    on_layer(shell, "Bottom")
    _, column = seated(shell, since)
    shown, whole = shows_whole(shell, since, tiled)
    assert shown == whole, f"in a gap under the windows, {shown} of its {whole} pixels show"
    since = shell.mark()
    floated = shell.opened("a floating window in front of a tiled one")
    shell.as_account(f"niri msg action toggle-window-floating --id {floated['id']}")
    asked = (round(column["left"] + column["width"] / 4), round(column["top"] + 120))
    shell.as_account(
        f"niri msg action move-floating-window --id {floated['id']} -x {asked[0]} -y {asked[1]}"
    )
    sits_over(shell, floated, since)
    on_layer(shell, "Top")
    _, floating = seated(shell, since)
    shown, whole = shows_whole(shell, since, floated)
    assert shown == whole, f"on a floating window over a tiled one, {shown} of {whole} show"
    # Where niri put it against where it was asked to, since a floating window's place is
    # the working area's, which the margin's layer may take some of.
    off = (floating["left"] - asked[0], floating["top"] - asked[1])
    since = shell.mark()
    shell.as_account(f"niri msg action focus-window --id {tiled['id']}")
    sits_over(shell, tiled, since)
    on_layer(shell, "Bottom")
    at, _ = seated(shell, since)
    over = (round(at["x"] - 40 - off[0]), round(at["y"] - 40 - off[1]))
    shell.as_account(
        f"niri msg action move-floating-window --id {floated['id']} -x {over[0]} -y {over[1]}"
    )
    shown = inks(shell, at, lambda drawn: drawn == 0)
    assert shown == 0, f"a floating window moved in front of it, {shown} of its pixels show"
    for window in (floated, tiled, upper):
        shell.close(window)


def counting(shell: Shell) -> str:
    """Give the command that counts the shell's frames by window (Frames.qml)."""
    return f"qs -p {shell.folder} ipc call frames"


def sitting(at: Json | None, scale: float) -> TypeGuard[Json]:
    """Say whether the cat's report has it sitting, sized for the scale, so its box is known.

    Its first report at a new scale can still be sized for the old one, and a walking cat's
    report runs ahead of the screen, so neither says what a look should find.
    """
    return (
        at is not None
        and at.get("walk") is None
        and round(at["size"] * scale) == at["cell"] * at["cells"]
    )


def cat_frames(shell: Shell) -> tuple[int, Json]:
    """Give the frames the cat's windows drew since the count started, and every window's."""
    drawn = json.loads(shell.as_account(f"{counting(shell)} said"))
    return sum(count for name, count in drawn.items() if name.startswith("cat ")), drawn


def cat_frames_of_its_own(shell: Shell) -> tuple[int, Json]:
    """Give the frames the cat drew beyond those its still neighbours drew, and every window's.

    A loaded builder once had the whole shell draw again under the launcher, the notices, the
    talk line and the cat 6 frames each (2026-11-20, cat-niri8), with nothing of the cat's
    own moving. A frame every still window drew too is the scene's, so the fewest any drew is
    the floor, a window that drew none counted as none, and only what the cat drew past it is
    its own.
    """
    drew, drawn = cat_frames(shell)
    floor = min(drawn.get(name, 0) for name in STILL_WINDOWS)
    return max(0, drew - floor), drawn


def test_the_cat_dozes_off_with_no_keys_drawing_nothing_more(shell: Shell) -> None:
    """With no keys it draws nothing, then curls up asleep in a frame, and a key wakes it.

    Thirty seconds with nothing typed draw none of its frames (VKCF3Z, 2026-11-12: "30 s idle
    draws 0 frames"), its doze has it asleep, and asleep it draws nothing new.
    """
    since = shell.mark()
    sits_over(shell, shell.opened("a column to doze over"), since)
    # Its last frame over the new column lands a moment after it says it sits there.
    time.sleep(LANDS)
    since = shell.mark()
    shell.send_key("F24")
    shell.as_account(f"{counting(shell)} start")
    time.sleep(IDLE)
    still, drawn = cat_frames(shell)
    said, _ = shell.said_since(since)
    sat = [report for report in said if report.startswith(SITS)]
    assert still == 0, (
        f"the cat drew {still} frames in {IDLE:.0f} s with no keys: {drawn}; it said {sat}"
    )
    shell.reported(SITS, asleep, since, timedelta(seconds=DOZES))
    time.sleep(LANDS)
    dozed, drawn = cat_frames(shell)
    assert dozed <= POSED, f"the cat drew {dozed} frames dozing off: {drawn}"
    since = shell.mark()
    shell.send_key("F24")
    heard(shell, lambda said: not asleep(said), since)


def stretching(said: str) -> bool:
    """Say whether a report has the cat stretching where it sits."""
    at = placed(said)
    return at is not None and bool(at.get("stretching"))


def test_the_cat_stretches_as_the_screen_unlocks(shell: Shell) -> None:
    """As the lock lets the screens go it stretches for a moment, in one pose, then sits.

    The lock's word is written as Lock.qml writes it, so no lock covers the nested screen, and
    a shell that starts unlocked never stretches, since nothing was let go.
    """
    since = shell.mark()
    sits_over(shell, shell.opened("a column to stretch over"), since)
    marker = Path(os.environ["XDG_RUNTIME_DIR"]) / "quickshell-locked"
    said, _ = shell.said_since(0)
    started = [report for report in said if report.startswith(SITS)]
    assert not any(stretching(report[len(SITS) :]) for report in started), (
        f"the cat stretched with nothing unlocked: {started}"
    )
    marker.write_text("locked\n")
    # The cat hears the lock's word by watching its file, a moment after it's written.
    time.sleep(LANDS)
    since = shell.mark()
    marker.write_text("unlocked\n")
    heard(shell, stretching, since)
    began, since = time.monotonic(), shell.mark()
    heard(shell, lambda said: placed(said) is not None and not stretching(said), since)
    held = time.monotonic() - began
    shortest, longest = STRETCHES
    assert shortest <= held <= longest, f"the cat stretched for {held:.2f} s"


def test_the_cat_goes_in_fullscreen(shell: Shell) -> None:
    """A window in fullscreen leaves no gap above it, so the cat goes, and comes back after."""
    since = shell.mark()
    filling = shell.opened("a column to fill the screen")
    sits_over(shell, filling, since)
    layered(shell, 1)
    since = shell.mark()
    shell.as_account("niri msg action fullscreen-window")
    heard(shell, lambda said: placed(said) is None, since)
    layered(shell, 0)
    shell.as_account("niri msg action fullscreen-window")
    layered(shell, 1)


def test_leaving_and_coming_back_at_1_5_works_its_place_out_once(shell: Shell) -> None:
    """At 1.5 the cat goes and comes back without its place depending on itself.

    Its window's scale and the screen's differ at a fractional scale, so showing or hiding the
    window moved the scale its place is worked out from: Joel's shell logged a binding loop on
    `place` 327 times from 15:12 to 15:57 on 2026-11-12, three around each launcher opening,
    and read a place left undefined meanwhile ("Cannot read property 'x' of undefined").
    """
    since = shell.mark()
    filling = shell.opened("a column to leave at 1.5")
    sits_over(shell, filling, since)
    began = shell.mark()
    shell.as_account("niri msg output winit scale 1.5")
    try:
        for _ in range(2):
            since = shell.mark()
            shell.as_account("niri msg action fullscreen-window")
            heard(shell, lambda said: placed(said) is None, since)
            since = shell.mark()
            shell.as_account("niri msg action fullscreen-window")
            heard(shell, lambda said: placed(said) is not None, since)
        with shell.log.open("rb") as log:
            log.seek(began)
            looped = [
                line
                for line in log.read().decode(errors="replace").splitlines()
                if ("Binding loop" in line or "TypeError" in line) and "MarginCat" in line
            ]
        assert not looped, f"the cat's place depended on itself, or went unread: {looped}"
    finally:
        shell.as_account("niri msg output winit scale 1")


def screen() -> Path:
    """Give where a screenshot taken only to be read goes."""
    return Path(os.environ["TMPDIR"]) / "screen.png"


def picture(shell: Shell, taken: Path, rows: int | None = None) -> tuple[int, bytes]:
    """Take the screen as niri draws it, in its own pixels, and give its width and its pixels.

    Only its first `rows` where given: unfiltering a whole screen in Python takes seconds on a
    loaded builder, which left a wait for the cat two looks at it (2026-11-12, 2 of 6 runs).

    niri's own screenshot rather than grim's: grim scales its picture by the output's pixels
    over its logical size, which niri rounds at 1.5 (1276 over 851), so it filters every
    edge by a hair and smears pixels the screen has whole.
    """
    taken.unlink(missing_ok=True)
    shell.as_account(f"niri msg action screenshot-screen --show-pointer false --path {taken}")
    deadline = time.monotonic() + LAYERED
    while not taken.exists() or taken.stat().st_size == 0:
        assert time.monotonic() < deadline, f"niri wrote no screenshot to {taken}"
        time.sleep(POLL)
    return decoded(taken.read_bytes(), rows)


def decoded(png: bytes, upto: int | None = None) -> tuple[int, bytes]:
    """Give a PNG's width and its pixels as red, green and blue bytes, as niri writes one."""
    assert png.startswith(b"\x89PNG\r\n\x1a\n"), "niri's screenshot isn't a PNG"
    at, width, height, channels, packed = 8, 0, 0, 0, b""
    while at < len(png):
        (length,) = struct.unpack(">I", png[at : at + 4])
        kind, data = png[at + 4 : at + 8], png[at + 8 : at + 8 + length]
        if kind == b"IHDR":
            width, height, depth, colour = struct.unpack(">IIBB", data[:10])
            assert depth == BYTE, f"niri's screenshot has {depth} bits a channel"
            channels = {RGB: 3, RGBA: 4}[colour]
        elif kind == b"IDAT":
            packed += data
        at += 12 + length
    raw, stride = zlib.decompress(packed), width * channels
    rows: list[bytearray] = []
    for row in range(height if upto is None else min(upto, height)):
        start = row * (stride + 1)
        line = bytearray(raw[start + 1 : start + 1 + stride])
        unfilter(raw[start], line, rows[-1] if rows else bytearray(stride), channels)
        rows.append(line)
    pixels = b"".join(rows)
    if channels == RGB:
        return width, pixels
    return width, b"".join(pixels[spot : spot + 3] for spot in range(0, len(pixels), 4))


def unfilter(kind: int, line: bytearray, above: bytearray, channels: int) -> None:
    """Undo a PNG row's filter in place, from the row above it (the PNG standard, 9.2)."""
    for spot, value in enumerate(line):
        left = line[spot - channels] if spot >= channels else 0
        up = above[spot]
        corner = above[spot - channels] if spot >= channels else 0
        guess = [0, left, up, (left + up) // 2, paeth(left, up, corner)][kind]
        line[spot] = (value + guess) & 0xFF


def paeth(left: int, up: int, corner: int) -> int:
    """Give whichever neighbour is nearest left + up - corner, as PNG's Paeth filter does."""
    guess = left + up - corner
    near = min((abs(guess - left), 0), (abs(guess - up), 1), (abs(guess - corner), 2))
    return (left, up, corner)[near[1]]


def anywhere(shell: Shell, hue: str) -> str:
    """Say where on the whole screen pixels near `hue` are, for a cat not found in its box."""
    width, pixels = picture(shell, screen())
    spots = [
        (spot % width, spot // width)
        for spot in range(len(pixels) // 3)
        if near(pixels[spot * 3 : spot * 3 + 3].hex(), hue)
    ]
    if not spots:
        return f"no pixel near it on the {width} pixels wide screen"
    columns, rows = [x for x, _ in spots], [y for _, y in spots]
    return (
        f"{len(spots)} pixels near it on the {width} pixels wide screen, from "
        f"({min(columns)}, {min(rows)}) to ({max(columns)}, {max(rows)})"
    )


def near(colour: str, hue: str) -> bool:
    """Say whether each of a colour's channels is within NEAR of the hue's."""
    pairs = zip(bytes.fromhex(colour[-6:]), bytes.fromhex(hue[-6:]), strict=True)
    return all(abs(a - b) <= NEAR for a, b in pairs)


def inked(colours: Counter[str], hue: str) -> int:
    """Count the pixels of the ink the cat's hue comes out as, its box's commonest near it.

    The software renderer gives a theme's colour back a step or two off in its channels, so the
    ink is the colour it drew rather than the hue it was asked for. A cell split across a
    pixel mixes with the gap behind it into colours the count leaves out.
    """
    ink = [count for colour, count in colours.most_common() if near(colour, hue)]
    return ink[0] if ink else 0


def drawn_as(shell: Shell, at: Json, scale: float) -> str:
    """Draw the cat's box as the screen has it: # its exact hue, ~ near it, . anything else."""
    width, pixels = picture(shell, screen())
    left, top = round(at["x"] * scale) - 1, max(0, round(at["y"] * scale) - 1)
    across = round(at["size"] * scale) + 2
    hue = at["hue"][-6:].lower()

    def mark(spot: int) -> str:
        colour = pixels[spot * 3 : spot * 3 + 3].hex()
        return "#" if colour == hue else "~" if near(colour, hue) else "."

    # A box past the screen's edge, the cat on its floor, is drawn only as far as the screen.
    height = len(pixels) // (width * 3)
    columns = range(max(0, left), min(width, left + across))
    return "\n".join(
        "".join(mark(row * width + column) for column in columns)
        for row in range(top, min(height, top + across))
    )


def boxed(shell: Shell, at: Json, scale: float) -> Counter[str]:
    """Count each colour of the screen's pixels in the cat's box and a pixel around it.

    Only those on the screen: the cat's first report at a new scale can still give its place
    at the old one, past the screen's edge at the new, until niri lays the columns out again.
    """
    left, top = round(at["x"] * scale) - 1, max(0, round(at["y"] * scale) - 1)
    across = round(at["size"] * scale) + 2
    width, pixels = picture(shell, screen(), top + across)
    tall = len(pixels) // (width * 3)
    return Counter(
        "#" + pixels[(row * width + column) * 3 : (row * width + column) * 3 + 3].hex()
        for row in range(top, min(top + across, tall))
        for column in range(max(0, left), min(left + across, width))
    )


@pytest.mark.parametrize("scale", [2, 1.5])
def test_the_cat_is_drawn_in_whole_pixels(shell: Shell, scale: float) -> None:
    """At 1.5 as at 2, every lit cell is whole device pixels of its hue, none blurred.

    Each lit cell drawn whole gives the cell's pixels squared of exactly the hue, and a cell
    blurred or split across a pixel gives fewer, so the count says it's sharp. The screen is
    kept as the check's result, for a look at it (WCJQSG, 2026-11-12: "Send VKCF3Z a scale-2
    shot of each before it reaches Joel").
    """
    since = shell.mark()
    sits_over(shell, shell.opened(f"a column at {scale}"), since)
    since = shell.mark()
    shell.as_account(f"{counting(shell)} start")
    shell.as_account(f"niri msg output winit scale {scale}")
    began = time.monotonic()
    heard: list[str] = []
    seen = 0
    try:
        # niri lays the columns out again at the new scale a moment after it takes it, so the
        # cat's first report there may still be over the old ones: each try takes its newest.
        # It may walk to a seat there, and on a loaded builder a look can take longer than any
        # fixed wait, so the wait goes by the cat rather than the clock: only a look begun
        # LANDS after its newest report, with it sitting and saying nothing new meanwhile,
        # judges it (FEH1HE and Y2YBTS's gates, 2026-11-14: one look in a 10 s wait, taken
        # before the cat was drawn, and a look taking 32 s while it walked).
        at: Json | None = None
        drawn, wanted, colours = 0, -1, Counter[str]()
        heard_at, judged = began, 0
        while drawn != wanted and judged < JUDGING_LOOKS and time.monotonic() - began < SETTLES_BY:
            said, _ = shell.said_since(since)
            if len(said) > seen:
                heard_at = time.monotonic()
            heard += [
                f"{time.monotonic() - began:.1f}s {report}"
                for report in said[seen:]
                if report.startswith(SITS)
            ]
            seen = len(said)
            sits = [placed(report[len(SITS) :]) for report in said if report.startswith(SITS)]
            at = sits[-1] if sits else None
            looked = time.monotonic()
            if sitting(at, scale):
                wanted = at["lit"] * at["cell"] ** 2
                colours = boxed(shell, at, scale)
                drawn = inked(colours, at["hue"])
                still = len(shell.said_since(since)[0]) == seen and looked - heard_at >= LANDS
                judged = judged + 1 if still else 0
                heard.append(
                    f"{time.monotonic() - began:.1f}s look from {looked - began:.1f}s: "
                    f"{drawn} in its box, {'judged' if still else 'not judged'}"
                )
            if drawn != wanted:
                time.sleep(POLL)
        assert at is not None, f"the cat said nowhere at {scale}"
        # Whether the shell drew the cat's window at all meanwhile, so a cat missing from the
        # screen says which side of the socket kept it.
        frames = cat_frames(shell) if drawn != wanted else (0, {})
        assert drawn == wanted, (
            f"the shell drew {frames[0]} of the cat's frames in the wait, of {frames[1]}; "
            f"at {scale}, {drawn} pixels are the cat's ink, not {at['lit']} cells of "
            f"{at['cell']} squared: {at}; its box holds {colours.most_common(6)}, and "
            f"there are {anywhere(shell, at['hue'])}; drawn:\n{drawn_as(shell, at, scale)}\n"
            f"heard:\n"
            + "\n".join(heard)
            + "\nniri's windows: "
            + shell.as_account("niri msg --json windows")
        )
        kept = Path(os.environ["out"])  # noqa: SIM112, the check's own result, as Nix names it
        kept.mkdir(exist_ok=True)
        picture(shell, kept / f"cat-at-{scale}.png")
    finally:
        shell.as_account("niri msg output winit scale 1")


def moved(shell: Shell, since: int) -> list[Json]:
    """Give each step of a move between workspaces the cat said since the mark, in order."""
    said, _ = shell.said_since(since)
    reports = [placed(report[len(SITS) :]) for report in said if report.startswith(SITS)]
    return [at for at in reports if at is not None and at.get("move")]


def moving(said: str) -> bool:
    """Say whether a report is a step of a move between workspaces."""
    at = placed(said)
    return at is not None and bool(at.get("move"))


def moving_back(said: str) -> bool:
    """Say whether a report is a step of a move up a workspace, the way back from below."""
    at = placed(said)
    return at is not None and bool(at.get("move")) and at["move"]["way"] < 0


def crossed(shell: Shell, since: int, way: int) -> list[Json]:
    """Give a move's steps once it sits, walked as if the workspaces were one screen.

    Down (`way` 1) it goes off the top with the workspace it leaves, never off the bottom, and
    its way drops across the gap to grab a top's corner below (Joel, 2026-11-14 17:30, "drop
    down and grab with its claws on the top of the window below"), or lands on a top with no
    corner in its reach, never on the floor through a window; niri centres a window alone on
    its workspace, so no corner below is in reach and it lands (2026-11-15). Up (-1) it goes
    off the bottom, never the top, and leaps up the gap to hang on a side's foot above, never
    its underside (VKCF3Z fcd95edd). Which it takes is MarginCatStacked.js's, held in Qt's
    engine.
    """
    took = moved(shell, since)
    assert took, "the cat never said a step of its move"
    beats = took[-1]["move"]["beats"]
    leaps = "drops" if way > 0 else "springs"
    caught = {"grabs", "lands"} if way > 0 else {"grabs"}
    assert leaps in beats, f"its way was {beats}"
    assert beats[beats.index(leaps) + 1] in caught, f"its way was {beats}"
    above = [step for step in took if step["y"] + step["size"] <= 0]
    under = [step for step in took if step["y"] + step["size"] >= step["reach"]]
    gone, kept = (above, under) if way > 0 else (under, above)
    assert gone, f"it never went off the screen with its workspace: {took[:3]}"
    assert not kept, f"it went to the screen's other edge: {kept[:3]}"
    assert {step["move"]["way"] for step in took} == {way}, f"it moved {took[:3]}"
    return took


def test_the_cat_walks_between_workspaces_as_if_on_one_screen(shell: Shell) -> None:
    """Up and down a workspace, it rides the one it leaves off the screen and comes in beyond.

    Its ways are MarginCatStacked.js's, frame by frame in Qt's engine; here, only that niri's
    workspaces and their windows reach it, up one and down one: up, it rides the old workspace
    off the bottom and leaps in from below; down, off the top and drops in from above (VKCF3Z
    fcd95edd). Each move's steps are kept as the check's result, for a strip.
    """
    since = shell.mark()
    upper = shell.opened("a workspace to go up to")
    sits_over(shell, upper, since)
    shell.as_account("niri msg action focus-workspace-down")
    since = shell.mark()
    lower = shell.opened("a workspace to go down to")
    sits_over(shell, lower, since)
    since = shell.mark()
    shell.as_account("niri msg action focus-workspace-up")
    heard(shell, moving, since)
    sits_over(shell, upper, since)
    up = crossed(shell, since, -1)
    # On a loaded builder the case runs past the cat's 45 s doze, and asleep it keeps its place
    # as the workspace goes (2026-11-20, cat-niri10), so a key wakes it, as before each case.
    woken = shell.mark()
    shell.send_key("F24")
    if dozing(shell):
        heard(shell, lambda said: not asleep(said), woken)
    since = shell.mark()
    shell.as_account("niri msg action focus-workspace-down")
    heard(shell, moving, since)
    sits_over(shell, lower, since)
    down = crossed(shell, since, 1)
    kept = Path(os.environ["out"])  # noqa: SIM112, the check's own result, as Nix names it
    kept.mkdir(exist_ok=True)
    (kept / "moves.json").write_text(json.dumps({"up": up, "down": down}))


def on_the_floor(said: str) -> bool:
    """Say whether a report has the cat sitting on the screen's floor, still."""
    at = placed(said)
    return at is not None and not at.get("walk") and at["y"] + at["size"] == at["reach"]


def test_an_empty_workspace_has_it_fall_to_the_floor_and_climb_back(shell: Shell) -> None:
    """On a workspace with no windows it sits on the floor, and climbs to one opened there.

    Its fall and climb are MarginCatPaths.js's, frame by frame in Qt's engine; here, only that
    niri's empty workspace reaches it: it's never nowhere from the move on, and sits still at
    the screen's foot until a window comes (VKCF3Z, e5a3ee44).
    """
    since = shell.mark()
    above = shell.opened("a workspace to leave for an empty one")
    sits_over(shell, above, since)
    holding = {window["workspace_id"] for window in shell.windows()}
    spaces = json.loads(shell.as_account("niri msg --json workspaces"))
    empty = next(space for space in spaces if space["id"] not in holding)
    since = shell.mark()
    shell.as_account(f"niri msg action focus-workspace {empty['idx']}")
    heard(shell, on_the_floor, since)
    said, _ = shell.said_since(since)
    reports = [report[len(SITS) :] for report in said if report.startswith(SITS)]
    nowhere = [report for report in reports if placed(report) is None]
    assert not nowhere, f"the cat went from the screen: {nowhere[:2]}"
    since = shell.mark()
    there = shell.opened("a window come to an empty workspace")
    sits_over(shell, there, since)


def test_a_second_move_holds_on_from_where_the_first_got_to(shell: Shell) -> None:
    """A move between workspaces as the last still holds moves it on from where it got to.

    It's never put anywhere it didn't get to (WCJQSG, 2bd7c7c6): its first place said after
    the second move is made is a step from its last before, or both are off the screen, where
    neither is seen, and the second move is one of its own, back the other way. A step of the
    first move said as the second is made may come after the mark, and is a step on too. Under
    load the first may not yet have left its seat, and the cat says it sits there before the
    second's first step (2026-11-14), so the second is waited for by its own steps rather than
    by where it sits.
    """
    since = shell.mark()
    upper = shell.opened("a workspace moved back to")
    sits_over(shell, upper, since)
    shell.as_account("niri msg action focus-workspace-down")
    since = shell.mark()
    lower = shell.opened("a workspace moved back from")
    sits_over(shell, lower, since)
    since = shell.mark()
    shell.as_account("niri msg action focus-workspace-up")
    sits_over(shell, upper, since)
    first = shell.mark()
    shell.as_account("niri msg action focus-workspace-down")
    heard(shell, moving, first)
    before, since = shell.said_since(first)
    shell.as_account("niri msg action focus-workspace-up")
    heard(shell, moving_back, since)
    sits_over(shell, upper, since)
    said, _ = shell.said_since(since)
    places = [placed(report[len(SITS) :]) for report in before + said if report.startswith(SITS)]
    # The first place the second move says, past any step of the first said after the mark.
    down = [((one or {}).get("move") or {}).get("way", 0) > 0 for one in places]
    assert any(down), "the first move never said where the cat was"
    turned = next((i for i in range(down.index(True), len(down)) if not down[i]), None)
    assert turned is not None, "the second move never said where the cat was"
    was, after = places[turned - 1], places[turned]
    assert was is not None, "the cat was nowhere as the first move went"
    assert after is not None, "the cat was nowhere as the second move began"
    # Caught off the screen between workspaces, waiting to drop in, it
    # may ride back in from anywhere else off it, since neither place is
    # seen (the net, 2026-11-16: from y -849 in its unseen beat to -131).
    off = [one["y"] + one["size"] <= 0 or one["y"] >= one["reach"] for one in (was, after)]
    gone = math.hypot(after["x"] - was["x"], after["y"] - was["y"])
    assert all(off) or gone <= STEPS_ON * was["size"], (
        f"the second move put it at {after}, from {was}"
    )
    again = [step for step in moved(shell, since) if step["move"]["way"] < 0]
    assert again, "the second move set it on its seat, moving it no more"


def test_a_row_closed_under_it_has_it_fall_at_once(shell: Shell) -> None:
    """A row of its column closed under it, it falls at once, never waiting for the keys to rest.

    Its fall ahead of the window grown down over it, and its climb from there, are
    MarginCatFalls.js's, frame by frame in Qt's engine; here, only that niri's own close reaches
    it: it sets off well inside the rest, falling first, never shoved, and sits on the window
    left (Joel, 2026-11-14 17:24: "it seems to hang out in the middle then walk across, that is
    not believable").
    """
    since = shell.mark()
    left = shell.opened("a row left in its column")
    closed = shell.opened("a row closed under the cat")
    sits_over(shell, closed, since)
    since = shell.mark()
    shell.as_account("niri msg action consume-or-expel-window-left")

    # Sat flush on the lower row's top, so the move into the column has settled and the
    # first walk after the close is the close's (a loaded builder, 2026-11-14: a ride
    # of the move's last pixel came first).
    heard(shell, flush_on_a_lower_row(closed), since)
    since = shell.mark()
    gone = time.monotonic()
    shell.close(closed)
    heard(shell, walking, since)
    set_off = time.monotonic() - gone
    sits_over(shell, left, since)
    walked = steps(shell, since)
    way = walked[0]["walk"]["beats"]
    assert set_off < RESTS / 2, f"the cat hung {set_off:.2f} s where its row was, then went"
    said, _ = shell.said_since(since)
    told = [report[len(SITS) :][:400] for report in said if report.startswith(SITS)][:6]
    assert way[:3] == ["rides", "lands", "shakes"], f"the cat's way off was {way}: {told}"
    assert "shoved" not in way, f"the cat was shoved across a window: {way}"


def test_a_middle_row_closed_under_it_has_it_fall_onto_the_row_below(shell: Shell) -> None:
    """The middle of three rows closed under it, it falls onto the row below, never riding it.

    niri first puts the row below in the closed row's place, its windows not resized yet, and
    lays the column out again only once they have: the cat waits for that, then falls in the
    falling drawing, lands and shakes it off, rather than riding the row below down sitting (the
    GPU rig, 2026-11-15: a jump of 105 px).
    """
    since = shell.mark()
    shell.opened("a row left over the middle")
    closed = shell.opened("a middle row closed under the cat")
    shell.as_account("niri msg action consume-or-expel-window-left")
    below = shell.opened("a row left under the middle")
    shell.as_account("niri msg action consume-or-expel-window-left")
    # Sat flush on the middle row once the keys come back to it, so the first walk after the
    # close is the close's.
    since = shell.mark()
    shell.as_account(f"niri msg action focus-window --id {closed['id']}")
    heard(shell, flush_on_a_lower_row(closed), since)
    since = shell.mark()
    shell.close(closed)
    heard(shell, walking, since)
    sits_over(shell, below, since)
    walked = steps(shell, since)
    way = walked[0]["walk"]["beats"]
    said, _ = shell.said_since(since)
    told = told_briefly(said)
    assert way[:3] == ["rides", "lands", "shakes"], f"the cat's way off was {way}: {told}"
    # Onto the row below, never through its face to the floor and back up (the GPU rig,
    # 2026-11-15).
    assert "climbs" not in way, f"the cat fell past the row below, its way {way}: {told}"
    # And so every walk set again as niri lays the column out, which the shell plans whether or
    # not a frame of it is drawn: never riding the row below down and walking on (a builder,
    # 2026-11-17: rides, turns, lands), nor climbing back from the floor.
    ways = {tuple(step["walk"]["beats"]) for step in walked}
    rode = [one for one in ways if "climbs" in one or ("rides", "turns") in itertools.pairwise(one)]
    assert not rode, f"a walk after the close rode or climbed, its ways {rode}: {told}"
    # Falling wherever a frame of its fall is drawn. The slow software niri may draw none (a
    # builder, 2026-11-17: from y 258 straight to its landing), so the drawing every frame of
    # it shows is margin-cat-rides.qml's to prove, at 120 Hz.
    falling = {step["walk"]["pose"] for step in walked if step["walk"]["beat"] == "rides"}
    assert falling <= {"falling"}, f"the cat fell as {falling}: {told}"


def told_briefly(said: list[str]) -> list[str]:
    """Give each report briefly: where it sat, on what, over what, and its walk's beat and pose.

    So a failure shows the order the shell heard niri's changes in, the windows' tops and the
    walk each set going, rather than the first few reports cut short.
    """
    told: list[str] = []
    for report in said:
        at = placed(report[len(SITS) :]) if report.startswith(SITS) else None
        if at is None:
            continue
        over = report[report.rindex(" over ") + len(" over ") :]
        top = (json.loads(over) or {}).get("top")
        walk = at.get("walk") or {}
        line = (
            f"y {at['y']:.0f} on {at.get('window')} seat {(at.get('seat') or {}).get('top')}"
            f" over {top}: {walk.get('beat')} {walk.get('pose')} {walk.get('beats', [])[:4]}"
        )
        if not told or told[-1] != line:
            told.append(line)
    return told[:40]


def point(shell: Shell, x: float, y: float) -> None:
    """Put the pointer at a point on the screen, through a virtual pointer from its corner."""
    wlrctl = os.environ["WLRCTL"]
    shell.as_account(f"{wlrctl} pointer move {TO_THE_CORNER} {TO_THE_CORNER}")
    shell.as_account(f"{wlrctl} pointer move {round(x)} {round(y)}")


def leaning(said: str) -> int:
    """Give the way a report has the cat leaning into the pointer, 0 for none."""
    at = placed(said)
    return 0 if at is None else int(at.get("leaning") or 0)


def leaning_to(way: int) -> Callable[[str], bool]:
    """Give a test of a report having the cat lean this way."""
    return lambda said: leaning(said) == way


def seat(shell: Shell, window: Json, since: int) -> Json:
    """Wait until the cat sits still on this window's column, and give where it sits.

    Still, its walk done, since it says it's over the middle from its walk's last beats on,
    and a pointer it comes to under as it walks never pets it (a builder, 2026-11-15). Its
    way there may climb from the floor to a top, which a builder drawing slowly took past
    the shell's usual wait to finish (2026-11-16: it sat a moment after the wait gave up), so
    its two waits share as long as a cat is given to settle.
    """

    def still_on(said: str) -> bool:
        at = placed(said)
        return bool(
            at is not None
            and at.get("window") == window["id"]
            and at.get("walk") is None
            and at.get("move") is None
        )

    deadline = time.monotonic() + SETTLES_BY
    sits_over(shell, window, since, timedelta(seconds=SETTLES_BY))
    heard(shell, still_on, since, timedelta(seconds=max(deadline - time.monotonic(), 1)))
    said, _ = shell.said_since(since)
    sat = next(report[len(SITS) :] for report in reversed(said) if report.startswith(SITS))
    assert still_on(sat), f"the cat sat still over its window and went on: {sat}"
    at = placed(sat)
    assert at is not None
    return at


def settled(shell: Shell, window: Json, away_from: float, since: int) -> Json:
    """Wait until the cat sits still on this window but not at `away_from` across; give where."""

    def still_elsewhere(said: str) -> bool:
        at = placed(said)
        return bool(
            at is not None
            and at.get("window") == window["id"]
            and at["x"] != away_from
            and at.get("walk") is None
            and at.get("move") is None
        )

    heard(shell, still_elsewhere, since)
    said, _ = shell.said_since(since)
    sat = next(report for report in reversed(said) if report.startswith(SITS))[len(SITS) :]
    assert still_elsewhere(sat), f"the cat sat still and went on: {sat}"
    at = placed(sat)
    assert at is not None
    return at


def test_a_pointer_resting_on_the_cat_leans_it_in(shell: Shell) -> None:
    """A pointer moved onto the cat leans it in toward its side, and the lean ends as it goes.

    Joel, 2026-11-12: "i want to be able to also use my mouse to engage with my animals". The
    times and the drawing are held in Qt's engine (margin-cat.qml); here, that niri's pointer
    reaches its cells on its layer, on either side. A pet takes the cat at rest a moment
    (MarginCat.strokes), and a pointer that comes sooner never pets it, as one it came to
    under, so the pointer comes once it has rested that long. Coming sooner is the likeliest
    reason it missed its lean in 29 of 100 gate runs (2026-11-16).
    """
    since = shell.mark()
    petted = shell.opened("a column the cat is petted on")
    at = seat(shell, petted, since)
    time.sleep(PETTED_FROM)
    middle = at["y"] + at["size"] / 2
    for across, way in ((1, -1), (at["size"] - 2, 1)):
        shell.send_key("F24")
        since = shell.mark()
        point(shell, at["x"] + across, middle)
        heard(shell, leaning_to(way), since)
        since = shell.mark()
        point(shell, TO_THE_CORNER, TO_THE_CORNER)
        heard(shell, lambda said: leaning(said) == 0, since)


def test_a_click_beside_the_cat_reaches_the_window(shell: Shell) -> None:
    """The cat's layer spans the screen, yet only its cells take the pointer.

    So a click on the window beside it focuses that window: the cat never takes a click Joel
    meant for his work. It sits on a floating window, where its layer is over the windows: on
    a tiled one it's under them, where the window would take the pointer with no mask at all.
    With the mask, niri's focus following the pointer focuses the window on the pointer's move,
    before the click; with none, the layer takes the move and the click alike, and focus stays.
    """
    beside = shell.opened("a column clicked into past the cat")
    since = shell.mark()
    floated = shell.opened("a floating window the cat sits on as the pointer passes")
    try:
        shell.as_account(f"niri msg action toggle-window-floating --id {floated['id']}")
        # The column left alone is centred again, so its corner is read once it's still.
        place = laid_out(shell, beside)["tile_pos_in_workspace_view"]
        corner = (place[0] + INSIDE, place[1] + INSIDE)
        shell.as_account(
            f"niri msg action move-floating-window --id {floated['id']}"
            f" -x {round(corner[0] + CLEAR)} -y {round(corner[1] + CLEAR)}"
        )
        layout = laid_out(shell, floated)
        left, top = layout["tile_pos_in_workspace_view"]
        assert corner[0] < left or corner[1] < top, f"the float covers the click: {layout}"
        # A report on the window while it was still tiled passes this, so the Top layer,
        # which only a floating seat gives, is what says it sits over the windows.
        sits_over(shell, floated, since, timedelta(seconds=SETTLES_BY))
        on_layer(shell, "Top")
        focused = json.loads(shell.as_account("niri msg --json focused-window"))
        assert focused["id"] == floated["id"], f"the column had the keys first: {focused}"
        click(shell, *corner)
        deadline = time.monotonic() + FOCUSES_BY
        while focused["id"] != beside["id"]:
            if time.monotonic() >= deadline:
                said, _ = shell.said_since(since)
                last = next((one for one in reversed(said) if one.startswith(SITS)), "")
                layers = [layer["layer"] for layer in cat_layers(shell)]
                stayed = f"focus stayed on {focused['title']!r}, the cat on {layers}: {last}"
                failed = f"a click beside the cat never reached the window; {stayed}"
                raise AssertionError(failed)
            time.sleep(POLL)
            focused = json.loads(shell.as_account("niri msg --json focused-window"))
    finally:
        # Closed even on a failure, so the cases after start with no float over the columns.
        for window in (floated, beside):
            shell.close(window)


def laid_out(shell: Shell, window: Json) -> Json:
    """Give where niri lays this window out, once it has stopped moving there.

    niri slides a window to its place, so its layout is read until it holds still.
    """
    deadline = time.monotonic() + LAYERED
    held = 0.0
    last: Json = {}
    while held < STILL_FOR:
        assert time.monotonic() < deadline, f"the window never held still: {last}"
        time.sleep(POLL)
        now = next(one for one in shell.windows() if one["id"] == window["id"])["layout"]
        held = held + POLL if now == last else 0.0
        last = dict(now)
    return last


def lying(said: str) -> Json | None:
    """Give where a report has the cat's yarn lying after a click, or None."""
    at = placed(said)
    return None if at is None else at.get("yarn")


def click(shell: Shell, x: float, y: float) -> None:
    """Click the pointer's left button at a point on the screen."""
    point(shell, x, y)
    shell.as_account(f"{os.environ['WLRCTL']} pointer click left")


def test_a_click_on_the_cat_rolls_its_yarn_in_and_one_on_the_yarn_sends_it_on(
    shell: Shell,
) -> None:
    """A click on the cat rolls its yarn in to lie by it, and a click on the yarn rolls it on.

    Joel, 2026-11-12: "i want to be able to also use my mouse to engage with my animals". The
    plays are held in Qt's engine (margin-cat-games.qml); here, that niri's clicks reach the
    cat's cells and the yarn's, whose cells take the pointer while it lies there. A click on
    its left rolls it away to the right.
    """
    since = shell.mark()
    played = shell.opened("a column the cat plays with its yarn on")
    at = seat(shell, played, since)
    shell.send_key("F24")
    since = shell.mark()
    click(shell, at["x"] + at["size"] / 2, at["y"] + at["size"] / 2)
    heard(shell, lambda said: lying(said) is not None, since)
    said, _ = shell.said_since(since)
    yarn = next(lying(report[len(SITS) :]) for report in reversed(said) if report.startswith(SITS))
    assert yarn is not None, "the cat's yarn came to lie by it and then went"
    since = shell.mark()
    click(shell, yarn["x"] + 1, yarn["y"] + at["size"] / 2)
    heard(shell, lambda said: (now := lying(said)) is not None and now["x"] > yarn["x"], since)
    point(shell, TO_THE_CORNER, TO_THE_CORNER)


def test_the_cat_never_leans_on_a_pointer_it_came_to_under(shell: Shell) -> None:
    """A pointer at rest where the cat sits down leaves it sitting, until the pointer moves.

    Only a pointer that moved onto it pets it (VKCF3Z, b8fc852e). Its column narrows, so it
    sits over its new middle, and widens again; the pointer waits at that middle, and the
    column narrows once more. Never a focus change, which would warp the pointer to the
    window's middle (warp-mouse-to-focus). Under it, the cat sits a pet's time and its hold's,
    unleaning, and a step of the pointer then leans it, so the pointer was over it all along.
    """
    since = shell.mark()
    under = shell.opened("a column the cat comes to sit under the pointer on")
    wide = seat(shell, under, since)
    since = shell.mark()
    shell.as_account("niri msg action set-column-width -10%")
    there = settled(shell, under, wide["x"], since)
    since = shell.mark()
    shell.as_account("niri msg action set-column-width +10%")
    settled(shell, under, there["x"], since)
    point(shell, there["x"] + there["size"] / 2, there["y"] + there["size"] / 2)
    since = shell.mark()
    shell.as_account("niri msg action set-column-width -10%")
    back = settled(shell, under, wide["x"], since)
    assert (back["x"], back["y"]) == (there["x"], there["y"]), (
        f"the cat came back to sit at {back}, not under the pointer at {there}"
    )
    # A pet would have leaned it by now, and a lean begun as it sat would still hold.
    time.sleep(PETS + PETTED)
    said, _ = shell.said_since(since)
    leant = [report for report in said if report.startswith(SITS) and leaning(report[len(SITS) :])]
    assert not leant, f"the cat leaned on a pointer it came to under: {leant[:2]}"
    shell.send_key("F24")
    since = shell.mark()
    shell.as_account(f"{os.environ['WLRCTL']} pointer move 1 0")
    heard(shell, lambda said: leaning(said) != 0, since)
    point(shell, TO_THE_CORNER, TO_THE_CORNER)
