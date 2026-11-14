"""The margin's cat grows livelier the less Joel types, on niri, as its scale says.

modules/quickshell/checks/margin.nix runs this through pytest in tests/nested-niri.sh, with the
account's whole shell, its cat counting Joel paused after 3 s with no keys and away after 8 s,
and its scenes coming fifteen times sooner than its own rests, so each is seen in seconds.
Which play comes at each level, how long each pose holds and where a wander goes are held in
Qt's engine (modules/margin-cat/tests/margin-cat.qml); here, only that niri's idle timers and
real columns reach it: a single pose held a moment while the keys come, which moves it none
and draws a frame each way, plays in place once they stop, and once they've been gone a while
a hop that rises and lands where it sat, a wander a few cells along the margin, a key
ending it at once, the cat walking back, and a ball of yarn batted off and a pounce on
nothing, each shown whole and each walking it back after. How a hop, the yarn and a pounce
move is held in Qt's engine too (modules/margin-cat/tests/margin-cat-games.qml). Each case
asks the shell for the play it watches, so none waits on chance.
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


def plays_out(shell: Shell, kind: str) -> tuple[list[Json], Json, Json]:
    """Sit over a column, ask for a play, leave the keys, and give it once it's done.

    It gives each place said while it played, how niri showed it, and where it sat before;
    and it waits for the cat to sit still over the column again, where it sat.
    """
    sat, seat = sitting_over(shell, f"a column played beside, {kind}")
    ask(shell, kind)
    shell.send_key("F24")
    since = shell.mark()
    shell.reported(
        SITS, lambda said: (playing(placed(said)) or {}).get("play") == kind, since, STROLLS
    )
    shell.reported(PLAYED, lambda said: said.split(" ", 1)[1].startswith("{"), since, PLAYS)
    said, _ = shell.said_since(since)
    done = next(at for at, line in enumerate(said) if line.startswith(PLAYED))
    told = json.loads(said[done][len(PLAYED) :].split(" ", 1)[1])
    during = [placed(line[len(SITS) :]) for line in said[:done] if line.startswith(SITS)]
    frames = [at for at in during if at is not None and playing(at)]
    after = [placed(line[len(SITS) :]) for line in said[done:] if line.startswith(SITS)]
    if not any(home(at, sat, seat) for at in after):
        shell.reported(SITS, lambda said: home(placed(said), sat, seat), shell.mark(), HOME)
    return frames, told, seat


def beats_of(frames: list[Json]) -> list[str]:
    """Give a play's beats as its frames came, each once however many frames it held."""
    beats = [at["walk"]["beat"] for at in frames]
    return [beat for at, beat in enumerate(beats) if at == 0 or beat != beats[at - 1]]


def quiet(shell: Shell) -> int:
    """Type until the cat has said nothing for a whole key's gap, and give the mark it's at.

    Sitting down can outlast a pause, so a pause's pose may be up, and the column just opened
    may still be settling: on a loaded builder either can draw after the first key, so the
    count starts only once neither has more to draw.
    """
    since = shell.mark()
    began = time.monotonic()
    while time.monotonic() - began < TYPED:
        shell.send_key("F24")
        time.sleep(TYPING)
        found, since = reports(shell, since)
        if not found:
            return since
    pytest.fail(f"the cat never sat quiet in {TYPED:.0f} s of typing")


def test_while_he_types_it_makes_one_small_move_in_place(shell: Shell) -> None:
    """A key every 0.4 s, and within the rest a single pose comes, holds a moment, and goes.

    It moves none while it does, and draws a frame for the pose and one for sitting again.
    """
    sitting_over(shell, "a column typed in")
    quiet(shell)
    shell.as_account(f"qs -p {shell.folder} ipc call frames start")
    since = shell.mark()
    began, posed, sat, still = time.monotonic(), 0.0, 0.0, None
    seen: list[Json] = []
    while time.monotonic() - began < TYPED and not sat:
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
    """Gone from the keys, it wanders a few cells along the margin; a key walks it back."""
    sat, seat = sitting_over(shell, "a column wandered from")
    ask(shell, "wander")
    since = shell.mark()
    away: list[Json] = []

    def wandered(said: str) -> bool:
        at = placed(said)
        if at is None or not at.get("strolling") or walking(at) or abs(at["x"] - seat["x"]) <= 1:
            return False
        away.append(at)
        return True

    shell.reported(SITS, wandered, since, STROLLS)
    cells = abs(away[-1]["x"] - seat["x"]) / seat["air"]
    fewest, most = WANDERS
    assert fewest - 0.5 <= cells <= most + 0.5, f"it wandered {cells:.1f} cells"
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


def test_away_it_bats_a_ball_of_yarn_off_and_walks_back(shell: Shell) -> None:
    """Away, a ball of yarn rolls in, is batted and pounced on, and goes; it walks back.

    Its beats come in order, the yarn coming into sight, shown whole and then fading as it's
    sent off, and niri shows every frame of it.
    """
    frames, shown, _ = plays_out(shell, "yarn")
    beats = beats_of(frames)
    assert in_order(beats, YARN), f"the yarn's beats came as {beats}"
    assert set(beats) >= YARN_SEEN, f"the yarn's beats came as {beats}"
    seen = [at["walk"]["yarn"]["opacity"] for at in frames if at["walk"]["yarn"]]
    whole = seen.index(1) if 1 in seen else None
    assert whole is not None, f"the yarn was never shown whole: {seen}"
    assert min(seen[:whole] or [1]) < 1, f"the yarn came from nowhere: {seen}"
    assert min(seen[whole:]) < 1, f"the yarn never faded as it went: {seen}"
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
