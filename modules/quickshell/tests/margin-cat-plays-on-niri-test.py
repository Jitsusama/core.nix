"""The margin's cat grows livelier the less Joel types, on niri, as its scale says.

modules/quickshell/checks/margin.nix runs this through pytest in tests/lib/nested-niri.sh, with the
account's whole shell, its cat counting Joel paused after 3 s with no keys and away after 8 s,
and its scenes coming fifteen times sooner than its own rests, so each is seen in seconds.
Which play comes at each level, how long each pose holds and where a wander goes are held in
Qt's engine (modules/margin-cat/tests/margin-cat.qml); here, only that niri's idle timers and
real columns reach it: a single pose held a moment while the keys come, which moves it none
and draws a frame each way, plays in place once they stop, and once they've been gone a while
a hop that rises and lands where it sat, a wander a few cells along the margin, a key
ending it at once, the cat walking back, and a ball of yarn batted off and a pounce on
nothing, each shown whole and each walking it back after, and the puppy's visit, the cat
hopping where it sits. How a hop, the yarn, a pounce and the puppy move is held in Qt's
engine too (modules/margin-cat/tests/margin-cat-games.qml and margin-cat-puppy.qml). Each
case asks the shell for the play it watches, so none waits on chance.
"""

from __future__ import annotations

import json
import time
from datetime import timedelta
from typing import TYPE_CHECKING

import launcher_on_niri
import pytest
from launcher_on_niri import placed

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

# What the cat says each time it's placed or posed (modules/margin-cat/MarginCatScreen.qml).
SITS = "sits cat "
# The small moves it makes while he types, and the poses of its plays in place.
GLANCES = {"earFlick", "tailSwish", "blinking"}
IN_PLACE = {"tailSwish", "batting", "stretching", "lookingAbout", "grooming", "blinking"}
SITTING = {"nothingHere", ""}
# How long a key is pressed apart while he types, how long typing goes on for a move to land
# (a rest of 3 to 5 s here), and how long a glance may hold, about 375 ms and some for a
# loaded builder.
TYPING, TYPED, GLANCE = 0.4, 12.0, (0.2, 0.9)
# The frames a small move draws: one for its pose, one for sitting again.
GLANCED = 2
# How long a play in place may last, and how long Joel is gone before a move asked for comes:
# 8 s here, the rest before it and the play under way as it's asked, with room to spare.
SCENE, STROLLS = 2.5, timedelta(seconds=60)
# How far a wander goes, in the cat's cells, and the cell and a half its walk overshoots.
WANDERS = (16, 40)
# How soon a key ends a stroll, and how long the walk back over its column may take: a
# gentle long walk, at most about 8 s, and some for a loaded builder.
BACK, HOME = 1.0, timedelta(seconds=15)
# What the cat says as a play that moves it is done (MarginCatScreen.qml's tell), each
# play's beats in order (modules/margin-cat/MarginCatGames.js), the ones every run must
# show, since a loaded nested frame can skip the shortest, the drawings of a hop, and how
# long a play may take: about 6 s, and some for a loaded builder.
PLAYED = "played cat "
# A play a case asked for, turned down by a cat, with why (MarginCatScreen.qml's turnedDown).
TURNED = "turned cat "
HOP = ["crouches", "hops", "lands"]
YARN = [
    "waits",
    "notices",
    "eyes",
    "bats",
    "watches",
    "crouches",
    "wiggles",
    "leaps",
    "lands",
    "pins",
    "sends",
]
POUNCE = ["notices", "eyes", "crouches", "wiggles", "leaps", "lands", "looks"]
YARN_SEEN, POUNCE_SEEN = {"waits", "watches", "sends"}, {"eyes", "leaps", "looks"}
# The puppy's visit (modules/margin-cat/MarginCatPuppy.js), on the cat's beats, and how long
# it may take: it trots in and off at the cat's walking pace, at most 80 cells a second,
# which over a nested column of a cell a pixel is up to about 10 s each way.
PUPPY = ["waits", "notices", "eyes", "crouches", "hops", "lands", "looks", "watches"]
PUPPY_SEEN = {"waits", "eyes", "hops", "watches"}
VISITS = timedelta(seconds=45)
# How long the shell may take to hear a key, through niri's word that he's no longer idle.
HEARS = timedelta(seconds=10)
HOPPING = {"pouncing", "leaping", "tucked", "falling"}
PLAYS = timedelta(seconds=20)
POLL = 0.05


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the account's whole shell once its cat has said where it sits, started once."""
    with launcher_on_niri.running() as started:
        started.reported(SITS, lambda _: True, 0)
        yield started


def walking(at: Json) -> bool:
    """Say whether a place is a step of a walk, as MarginCatScreen.qml says each one."""
    return bool(at.get("walk"))


def story(said: list[str]) -> str:
    """Give what the cat did over the reports, once each time it changed, to say why a wait failed.

    A failed wait's tail holds only the last second of reports, and the yarn's wait for its
    play to begin has failed twice that way (2026-11-23 and 2026-11-24) with nothing to say
    what came between the ask and the strolling the tail shows. It runs only on a failed wait,
    and a fault of its own still fails the check, with the wait's failure as its cause.
    """
    told: list[str] = []
    was: dict[str, object] | None = None
    for line in said:
        if line.startswith((PLAYED, TURNED)):
            told.append(line)
            was = None
        if not line.startswith(SITS):
            continue
        at = placed(line[len(SITS) :]) or {}
        walk = at.get("walk") or {}
        move = at.get("move") or {}
        keys = ("window", "strolling", "pose", "stilled", "held", "departing", "asleep")
        now: dict[str, object] = {key: at.get(key) for key in keys} | {
            "restBegun": at.get("restBegun"),
            "beats": walk.get("beats"),
            "play": walk.get("play"),
            "to": walk.get("to"),
            "move": move.get("beats"),
            "way": move.get("way"),
        }
        if now != was:
            told.append(f"x {at.get('x')}: {json.dumps(now)}")
            was = now
    return "\n".join(told)


def reports(shell: Shell, since: int) -> tuple[list[Json], int]:
    """Give each place the cat has said it's at since the mark, and the mark they end at."""
    said, since = shell.said_since(since)
    found = [placed(report[len(SITS) :]) for report in said if report.startswith(SITS)]
    return [at for at in found if at is not None], since


def cat_frames(shell: Shell) -> int:
    """Give the frames the cat's windows drew since the count started (Frames.qml)."""
    drawn = json.loads(shell.as_account(f"qs -p {shell.folder} ipc call frames said"))
    return sum(count for name, count in drawn.items() if name.startswith("cat "))


def ask(shell: Shell, play: str) -> None:
    """Ask that the cat's next play be the one named, once its level has it (MarginCat.qml)."""
    shell.as_account(f"qs -p {shell.folder} ipc call cat next {play}")


def leaves_then_asks(shell: Shell, play: str) -> int:
    """Press a key, and once the shell has heard it, ask for `play`, giving the mark before.

    The ask waits until the shell's level is typing, which only a key heard makes it: idle or
    paused, the cat draws plays the key would cut short, and typing it draws none. Asked before
    the key was heard, the scene the ask draws could come up first, spending the ask on a play
    the key then cut short, and the yarn and the puppy rest for minutes once begun: on a lender
    running 12 builds, no yarn came in 20 s (2026-11-26).

    The key is pressed again before each read, since typing lasts only the 3 s pause, which a
    slow read on a busy lender could pass between one key and the read after it; a key more
    only keeps the level typing. A level left typing by an earlier key passes at once, which is
    as safe, since typing draws no play for the key to cut. A level held still fails at once,
    since the ask would be refused.
    """
    level = f"qs -p {shell.folder} ipc call cat level"
    deadline = time.monotonic() + HEARS.total_seconds()
    while True:
        shell.send_key("F24")
        now = shell.as_account(level).strip()
        if now == "typing":
            break
        assert now != "still", f"the cat is held still, so it would refuse:\n{shell.tail()}"
        assert time.monotonic() < deadline, (
            f"the shell never heard a key in {HEARS}, its cat left {now}:\n{shell.tail()}"
        )
        # niri says he's no longer idle on its own time, with no word the test could wait on.
        time.sleep(POLL)
    since = shell.mark()
    ask(shell, play)
    return since


def sitting_over(shell: Shell, title: str) -> tuple[Json, Json]:
    """Open a column, wait until the cat sits over it, still, and give both."""
    since = shell.mark()
    window = shell.opened(title)
    seats: list[Json] = []

    def sits(said: str) -> bool:
        at = placed(said)
        if at is None or at.get("window") != window["id"] or walking(at):
            return False
        seats.append(at)
        return True

    shell.reported(SITS, sits, since)
    return window, seats[-1]


def playing(at: Json | None) -> Json | None:
    """Give the step of a play a place is, as MarginCatScreen.qml says each, or None."""
    walk = (at or {}).get("walk") or {}
    return walk if walk.get("play") else None


def in_order(seen: list[str], beats: list[str]) -> bool:
    """Say whether the beats seen, each once however many frames it held, come in order."""
    at = 0
    for beat in seen:
        if beat not in beats[at:]:
            return False
        at = beats.index(beat, at) + 1
    return True


def home(at: Json | None, sat: Json, seat: Json) -> bool:
    """Say whether the cat sits still over its column, where it sat before it played."""
    return (
        at is not None
        and not walking(at)
        and at.get("window") == sat["id"]
        and abs(at["x"] - seat["x"]) <= 1
    )


def plays_out(shell: Shell, kind: str, lasts: timedelta = PLAYS) -> tuple[list[Json], Json, Json]:
    """Sit over a column, leave the keys, ask for a play, and give it once it's done.

    It gives each place said while it played, how niri showed it, and where it sat before;
    and it waits for the cat to sit still over the column again, where it sat. The key ends a
    play already under way, and it reads from the asked play's first step, and only that
    play's steps (2026-11-15: a hop's beats were read as a pounce's).
    """
    sat, seat = sitting_over(shell, f"a column played beside, {kind}")
    since = leaves_then_asks(shell, kind)

    def asked(said: str) -> bool:
        return (playing(placed(said)) or {}).get("play") == kind

    try:
        shell.reported(SITS, asked, since, STROLLS)
    except AssertionError as failed:
        said, _ = shell.said_since(since)
        # The wait's own failure, its log tail with it, is printed once as this one's cause.
        told = f"what the cat did since the ask:\n{story(said)}"
        raise AssertionError(told) from failed
    began = shell.mark()
    shell.reported(PLAYED, lambda said: said.split(" ", 1)[1].startswith("{"), began, lasts)
    said, _ = shell.said_since(since)
    steps = [line[len(SITS) :] if line.startswith(SITS) else None for line in said]
    start = next(at for at, step in enumerate(steps) if step is not None and asked(step))
    done = next(at for at in range(start, len(said)) if said[at].startswith(PLAYED))
    told = json.loads(said[done][len(PLAYED) :].split(" ", 1)[1])
    during = [placed(step) for step in steps[start:done] if step is not None]
    frames = [at for at in during if at is not None and (playing(at) or {}).get("play") == kind]
    # A play is told once niri has answered for its last frames, which can be after it sits
    # again, so its seat is looked for from its last step on, not from the telling.
    ended = max(
        at
        for at, step in enumerate(steps[:done])
        if at >= start and step is not None and asked(step)
    )
    after = [placed(line[len(SITS) :]) for line in said[ended + 1 :] if line.startswith(SITS)]
    if not any(home(at, sat, seat) for at in after):
        shell.reported(SITS, lambda said: home(placed(said), sat, seat), shell.mark(), HOME)
    return frames, told, seat


def beats_of(frames: list[Json]) -> list[str]:
    """Give a play's beats as its frames came, each once however many frames it held."""
    beats = [at["walk"]["beat"] for at in frames]
    return [beat for at, beat in enumerate(beats) if at == 0 or beat != beats[at - 1]]


def quiet(shell: Shell) -> int:
    """Type until the cat has said and drawn nothing for a whole key's gap; give its mark.

    Sitting down can outlast a pause, so a pause's pose may be up, and the column just opened
    may still be settling: on a loaded builder either can draw after the first key, so the
    count starts only once neither has more to draw. A settling column redraws the cat's
    window without the cat saying anything, so its frames are counted as well as its words.
    """
    since = shell.mark()
    began = time.monotonic()
    while time.monotonic() - began < TYPED:
        shell.as_account(f"qs -p {shell.folder} ipc call frames start")
        shell.send_key("F24")
        time.sleep(TYPING)
        found, since = reports(shell, since)
        if not found and not cat_frames(shell):
            return since
    pytest.fail(f"the cat never sat quiet in {TYPED:.0f} s of typing")


def test_while_he_types_it_makes_one_small_move_in_place(shell: Shell) -> None:
    """A key every 0.4 s, and within the rest a single pose comes, holds a moment, and goes.

    It moves none while it does, and draws a frame for the pose and one for sitting again. Its
    frames are counted from the key's gap the pose came in, as a settling column can redraw
    its window between keys on a loaded builder, the cat saying nothing (RCTRF8, 15 Nov).
    """
    sitting_over(shell, "a column typed in")
    quiet(shell)
    since = shell.mark()
    began, posed, sat, still = time.monotonic(), 0.0, 0.0, None
    seen: list[Json] = []
    while time.monotonic() - began < TYPED and not sat:
        if not posed:
            shell.as_account(f"qs -p {shell.folder} ipc call frames start")
        shell.send_key("F24")
        time.sleep(TYPING)
        found, since = reports(shell, since)
        for at in found:
            seen.append(at)
            if not posed and at.get("pose") in GLANCES:
                posed, still = time.monotonic(), (at["x"], at["y"])
            elif posed and at.get("pose") == "":
                sat = time.monotonic()
                assert (at["x"], at["y"]) == still, f"a move while typing shifted it: {seen}"
    assert posed, f"no small move came in {TYPED:.0f} s of typing: {seen}"
    assert sat, f"its pose never went: {seen}"
    shortest, longest = GLANCE
    assert shortest - TYPING <= sat - posed <= longest + TYPING, f"it held {sat - posed:.2f} s"
    drawn = cat_frames(shell)
    assert drawn <= GLANCED, f"a small move drew {drawn} frames: {seen}"


def test_paused_it_plays_a_little_in_place(shell: Shell) -> None:
    """Once the keys stop, a play comes: the sitting cat's own moves, briefly, in place."""
    sitting_over(shell, "a column paused over")
    shell.send_key("F24")
    since = shell.mark()
    shell.reported(SITS, lambda said: (placed(said) or {}).get("pose") in IN_PLACE, since)
    began = time.monotonic()
    shell.reported(SITS, lambda said: (placed(said) or {}).get("pose") == "", since)
    lasted = time.monotonic() - began
    found, _ = reports(shell, since)
    # The key that starts the pause is typing too, so a glance may come before the pause
    # does; the play is judged from its first pose on.
    first = next(i for i, at in enumerate(found) if at.get("pose") in IN_PLACE)
    found = found[first:]
    poses = {at.get("pose") for at in found}
    assert poses <= IN_PLACE | SITTING, f"a pause's play posed {poses}"
    assert lasted <= SCENE, f"a pause's play lasted {lasted:.2f} s"
    assert not any(walking(at) for at in found), f"a pause moved: {found}"


def test_away_it_hops_and_lands_where_it_sat(shell: Shell) -> None:
    """Gone from the keys, it hops: rising from a crouch and landing, never moving along."""
    frames, shown, seat = plays_out(shell, "hop")
    beats = beats_of(frames)
    assert in_order(beats, HOP), f"the hop's beats came as {beats}"
    poses = {at["walk"]["pose"] for at in frames}
    assert poses <= HOPPING, f"it hopped as {poses}"
    assert all(abs(at["x"] - seat["x"]) <= 1 for at in frames), f"a hop moved along: {frames}"
    # A hop is a cell high, and a nested cell is a pixel, so its arc's shape is the Qt case's;
    # here, only that niri draws it in the air over more than a frame.
    aloft = [at for at in frames if at["y"] < seat["y"]]
    assert len(aloft) > 1, f"a hop was in the air for {len(aloft)} frames: {frames}"
    assert shown["unseen"] == 0, f"niri never showed {shown['unseen']} of its frames: {shown}"


def test_away_it_wanders_and_a_key_brings_it_back(shell: Shell) -> None:
    """Gone from the keys, it wanders a few cells along the margin; a key walks it back.

    A play already under way when it asks plays out first, and a play's walk rests away from
    the column too before it walks back, so it reads the wander by its own walk, one that is
    no play's and goes somewhere other than the seat, and waits for the cat to sit where that
    walk was going (2026-11-16: a yarn's rest, 7 cells off, was read as the wander's end).
    """
    sat, seat = sitting_over(shell, "a column wandered from")
    ask(shell, "wander")
    since = shell.mark()
    goes: list[float] = []

    def wanders(said: str) -> bool:
        at = placed(said)
        if at is None or not at.get("strolling") or not walking(at) or at["walk"].get("play"):
            return False
        if abs(at["walk"]["to"] - seat["x"]) <= 1:
            return False
        goes.append(at["walk"]["to"])
        return True

    shell.reported(SITS, wanders, since, STROLLS)
    cells = abs(goes[0] - seat["x"]) / seat["air"]
    fewest, most = WANDERS
    assert fewest - 0.5 <= cells <= most + 0.5, f"it set off to wander {cells:.1f} cells"
    shell.reported(
        SITS,
        lambda said: (
            (at := placed(said)) is not None and not walking(at) and abs(at["x"] - goes[0]) <= 1
        ),
        since,
        STROLLS,
    )
    since = shell.mark()
    keyed = time.monotonic()
    shell.send_key("F24")
    shell.reported(SITS, lambda said: not (placed(said) or {}).get("strolling"), since)
    ended = time.monotonic() - keyed
    assert ended <= BACK, f"a key took {ended:.2f} s to end the stroll"
    shell.reported(
        SITS,
        lambda said: (
            (at := placed(said)) is not None
            and not walking(at)
            and at.get("window") == sat["id"]
            and abs(at["x"] - seat["x"]) <= 1
        ),
        since,
        HOME,
    )
    found, _ = reports(shell, since)
    assert any(walking(at) for at in found), f"it came back over {sat} unwalked"


def behind_its_edge(yarn: Json, size: float) -> bool:
    """Whether some of the yarn, `size` across, is past the edge of the room it's drawn in."""
    return bool(yarn["x"] < yarn["within"]["left"] or yarn["x"] + size > yarn["within"]["right"])


def came_behind(frames: list[Json], size: float, key: str = "yarn") -> Json | None:
    """Give where the yarn was as it came, or None where it came from behind its edge.

    The puppy (`key` "puppy") is read the same way, since it too comes on the cat's clock.

    The yarn rolls on the cat's clock, so a frame drawn late finds it a few pixels on: a loaded
    builder's 50 ms frame showed it first 5 px out from its edge, though it rolled from behind
    it (2026-11-16, at d6345e9). So where its first frame isn't behind the edge, it rolls the
    yarn back at its own pace to the frame before it, or to the play's start where it shows
    from the play's first frame, and only a yarn that wasn't behind the edge then came in open
    space. Rolled back at its first two frames' pace, an easing yarn lands short of its true
    start, so one that started in open space stays there. Its going needs no such roll, since
    a play reports no frame after the yarn's last.
    """
    shown = [at for at, frame in enumerate(frames) if frame["walk"][key]]
    edge = shown[0]
    yarn: Json = frames[edge]["walk"][key]
    if behind_its_edge(yarn, size):
        return None
    after = edge + 1
    if after not in shown:
        return yarn
    then = frames[edge - 1]["walk"]["t"] if edge > 0 else 0
    now, later = frames[edge]["walk"]["t"], frames[after]["walk"]["t"]
    if later == now:
        return yarn
    pace = (frames[after]["walk"][key]["x"] - yarn["x"]) / (later - now)
    there = {**yarn, "x": yarn["x"] + pace * (then - now)}
    return None if behind_its_edge(there, size) else yarn


def went_behind(frames: list[Json], size: float) -> Json | None:
    """Give where the puppy was as it went, or None where it went behind its edge.

    It trots off to the play's end, behind its edge only for its last few cells, which a
    loaded builder can leave undrawn: so where its last frame isn't behind the edge, it trots
    it on at its last two frames' pace to the play's end (`walk`'s `ends`), and only a puppy
    that wasn't behind the edge then went in open space.
    """
    shown = [frame["walk"] for frame in frames if frame["walk"]["puppy"]]
    last = shown[-1]
    puppy: Json = last["puppy"]
    if behind_its_edge(puppy, size):
        return None
    before = shown[-2] if len(shown) > 1 else last
    if last["t"] == before["t"]:
        return puppy
    pace = (puppy["x"] - before["puppy"]["x"]) / (last["t"] - before["t"])
    there = {**puppy, "x": puppy["x"] + pace * (last["ends"] - last["t"])}
    return None if behind_its_edge(there, size) else puppy


def test_away_it_bats_a_ball_of_yarn_off_and_walks_back(shell: Shell) -> None:
    """Away, a ball of yarn rolls in, is batted and pounced on, and goes; it walks back.

    Its beats come in order, the yarn rolling out from behind its room's edge and going back
    behind it, never appearing or going in open space, and niri shows every frame of it.
    """
    frames, shown, _ = plays_out(shell, "yarn")
    beats = beats_of(frames)
    assert in_order(beats, YARN), f"the yarn's beats came as {beats}"
    assert set(beats) >= YARN_SEEN, f"the yarn's beats came as {beats}"
    seen = [at["walk"]["yarn"] for at in frames if at["walk"]["yarn"]]
    assert seen, f"the yarn was never shown: {frames}"
    size = frames[0]["size"]
    came = came_behind(frames, size)
    assert came is None, f"the yarn came from nowhere: {came}"
    assert behind_its_edge(seen[-1], size), f"the yarn went in open space: {seen[-1]}"
    assert shown["unseen"] == 0, f"niri never showed {shown['unseen']} of its frames: {shown}"


def test_away_it_pounces_on_nothing_and_walks_back(shell: Shell) -> None:
    """Away, it eyes, wiggles and leaps on nothing, lands, and walks back."""
    frames, shown, seat = plays_out(shell, "pounce")
    beats = beats_of(frames)
    assert in_order(beats, POUNCE), f"the pounce's beats came as {beats}"
    assert set(beats) >= POUNCE_SEEN, f"the pounce's beats came as {beats}"
    assert min(at["y"] for at in frames) < seat["y"], f"it never left the ground: {frames}"
    assert not any(at["walk"]["yarn"] for at in frames), f"a pounce brought yarn: {frames}"
    assert shown["unseen"] == 0, f"niri never showed {shown['unseen']} of its frames: {shown}"


def test_away_a_puppy_visits_and_the_cat_hops_where_it_sits(shell: Shell) -> None:
    """Away, the puppy trots in from behind its room's edge, and off again; the cat hops.

    Its beats come in order, the puppy never appearing or going in open space, the cat never
    leaving its seat but to hop a cell up and land, and niri shows every frame of it.
    """
    frames, shown, seat = plays_out(shell, "puppy", VISITS)
    beats = beats_of(frames)
    assert in_order(beats, PUPPY), f"the visit's beats came as {beats}"
    assert set(beats) >= PUPPY_SEEN, f"the visit's beats came as {beats}"
    seen = [at["walk"]["puppy"] for at in frames if at["walk"]["puppy"]]
    assert seen, f"the puppy was never shown: {frames}"
    lit = max(at.get("puppyLit", 0) for at in frames)
    assert lit > 0, f"the puppy was placed but drawn in no cell: {frames}"
    size = frames[0]["size"]
    came = came_behind(frames, size, "puppy")
    assert came is None, f"the puppy came from nowhere: {came}"
    went = went_behind(frames, size)
    assert went is None, f"the puppy went in open space: {went}"
    assert all(abs(at["x"] - seat["x"]) <= 1 for at in frames), f"the cat moved off: {frames}"
    aloft = [at for at in frames if at["y"] < seat["y"]]
    assert len(aloft) > 1, f"its hop was in the air for {len(aloft)} frames: {frames}"
    assert not any(at["walk"]["yarn"] for at in frames), f"a visit brought yarn: {frames}"
    assert shown["unseen"] == 0, f"niri never showed {shown['unseen']} of its frames: {shown}"


def test_a_key_ends_the_puppys_visit(shell: Shell) -> None:
    """The puppy visits only while he's away, so a key mid-visit ends it: it goes, the cat sits.

    A visit lasts up to about 25 s, and played on it would have the cat hopping as he types and
    asking niri for every tile (#6573's council, 2026-11-25).
    """
    sat, seat = sitting_over(shell, "a column a visit is cut short over")
    since = leaves_then_asks(shell, "puppy")

    def visiting(said: str) -> bool:
        return (playing(placed(said)) or {}).get("play") == "puppy"

    def in_sight(said: str) -> bool:
        step = playing(placed(said)) or {}
        return step.get("play") == "puppy" and step.get("beat") in PUPPY[1:-1]

    # Pressed once the puppy stands by the cat in plain sight, so it ends a visit he can see,
    # not one still behind the room's edge, where the puppy is drawn nowhere either way.
    try:
        shell.reported(SITS, in_sight, since, VISITS)
    except AssertionError as failed:
        # A visit that never came in 45 s said nothing of why (2026-11-26), so the story since
        # the ask names each time a cat turned it down, and what held it.
        said, _ = shell.said_since(since)
        told = f"what the cat did since the ask:\n{story(said)}"
        raise AssertionError(told) from failed
    since = shell.mark()
    keyed = time.monotonic()
    shell.send_key("F24")
    shell.reported(SITS, lambda said: not visiting(said), since)
    ended = time.monotonic() - keyed
    assert ended <= BACK, f"a key took {ended:.2f} s to end the visit"
    # `walk` says no puppy once the cat sits, so only the report's own `puppy` shows it gone.
    shell.reported(
        SITS,
        lambda said: (
            (at := placed(said)) is not None
            and not walking(at)
            and at.get("window") == sat["id"]
            and abs(at["x"] - seat["x"]) <= 1
            and not at.get("puppy")
        ),
        since,
        HOME,
    )
