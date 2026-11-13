"""The margin's cat grows livelier the less Joel types, on niri, as its scale says.

modules/quickshell/checks/margin.nix runs this through pytest in tests/nested-niri.sh, with the
account's whole shell, its cat counting Joel paused after 3 s with no keys and away after 8 s,
and its scenes coming fifteen times sooner than its own rests, so each is seen in seconds.
Which scene comes at each level, how long each pose holds and where a stroll goes are held in
Qt's engine (modules/margin-cat/tests/margin-cat.qml); here, only that niri's idle timers and
real columns reach it: a single pose held a moment while the keys come, which moves it none
and draws a frame each way, mild scenes once they stop, a stroll to the gutter beside its
column once they've been gone a while, and a key ending it at once, the cat walking back;
then, still away, a ball of yarn batted off and a pounce on nothing, each shown whole and
each walking it home after. They come in the order away's scenes take turns, so the cases
run in the order they're written.
"""

from __future__ import annotations

import json
import time
from datetime import timedelta
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

# What the cat says each time it's placed or posed (modules/margin-cat/MarginCat.qml).
SITS = "sits cat "
# The small moves it makes while he types, and the rest of a pause's poses.
GLANCES = {"earFlick", "tailSwish", "blinking"}
MILD = {*GLANCES, "nothingHere"}
# How long a key is pressed apart while he types, how long typing goes on for a move to land
# (a rest of 3 to 5 s here), and how long a glance may hold, about 375 ms and some for a
# loaded builder.
TYPING, TYPED, GLANCE = 0.4, 12.0, (0.2, 0.9)
# The frames a small move draws: one for its pose, one for sitting again.
GLANCED = 2
# How long a pause's scene may last, and how long Joel is gone before a stroll may come: 8 s
# here, and as many scenes as make one of them a stroll all but certain.
SCENE, STROLLS = 2.5, timedelta(seconds=60)
# How soon a key ends a stroll, and how long the walk back over its column may take: a
# gentle long walk, at most about 8 s, and some for a loaded builder.
BACK, HOME = 1.0, timedelta(seconds=15)
# What the cat says as a play is done (MarginCat.qml), each play's beats in order
# (modules/margin-cat/Play.js), the ones every run must show, since a nested frame comes
# about every 120 ms and skips the shortest, and how long a play may take: about 6 s, and
# some for a loaded builder.
PLAYED = "played cat "
YARN = [
    "rolls in",
    "notices",
    "eyes",
    "bats",
    "knocked",
    "wiggles",
    "leaps",
    "lands",
    "holds",
    "sends off",
]
POUNCE = ["notices", "eyes", "wiggles", "leaps", "lands", "looks"]
YARN_SEEN, POUNCE_SEEN = {"rolls in", "knocked", "sends off"}, {"eyes", "leaps", "looks"}
PLAYS = timedelta(seconds=20)
POLL = 0.05


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the account's whole shell once its cat has said where it sits, started once."""
    with launcher_on_niri.running() as started:
        started.reported(SITS, lambda _: True, 0)
        yield started


def placed(said: str) -> Json | None:
    """Give where a report says the cat sits, or None where it says nowhere."""
    _, rest = said.split(" ", 1)
    at, _ = rest.split(" over ", 1)
    return None if at == "nowhere" else cast("Json", json.loads(at))


def walking(at: Json) -> bool:
    """Say whether a place is a step of a walk, as MarginCat.qml says each one."""
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


def in_order(seen: list[str], beats: list[str]) -> bool:
    """Say whether the beats seen, each once however many frames it held, come in order."""
    at = 0
    for beat in seen:
        if beat not in beats[at:]:
            return False
        at = beats.index(beat, at) + 1
    return True


def plays_out(shell: Shell, kind: str, what: str) -> tuple[list[Json], Json]:
    """Sit over a column, leave the keys, and give a play of its frames once it's done.

    It gives each place said while it played and how niri showed it, once the cat sits
    still over the column again, where it sat before.
    """
    sat, seat = sitting_over(shell, f"a column {what} played beside")
    shell.send_key("F24")
    since = shell.mark()
    shell.reported(
        SITS,
        lambda said: ((placed(said) or {}).get("play") or {}).get("kind") == kind,
        since,
        STROLLS,
    )
    told: list[Json] = []

    def shown(said: str) -> bool:
        _, rest = said.split(" ", 1)
        if not rest.startswith(f"{what} "):
            return False
        told.append(json.loads(rest[len(what) + 1 :]))
        return True

    shell.reported(PLAYED, shown, since, PLAYS)
    said, _ = shell.said_since(since)
    done = next(at for at, line in enumerate(said) if line.startswith(PLAYED))
    during = [placed(line[len(SITS) :]) for line in said[:done] if line.startswith(SITS)]
    after = [placed(line[len(SITS) :]) for line in said[done:] if line.startswith(SITS)]
    frames = [at for at in during if at is not None and at.get("play")]
    if not any(home(at, sat, seat) for at in after):
        shell.reported(SITS, lambda said: home(placed(said), sat, seat), shell.mark(), HOME)
    return frames, told[-1]


def beats_of(frames: list[Json]) -> list[str]:
    """Give a play's beats as its frames came, each once however many frames it held."""
    beats = [at["play"]["beat"] for at in frames]
    return [beat for at, beat in enumerate(beats) if at == 0 or beat != beats[at - 1]]


def home(at: Json | None, sat: Json, seat: Json) -> bool:
    """Say whether the cat sits still over its column, where it sat before it played."""
    return (
        at is not None
        and not walking(at)
        and not at.get("play")
        and at.get("window") == sat["id"]
        and abs(at["x"] - seat["x"]) <= 1
    )


def test_while_he_types_it_makes_one_small_move_in_place(shell: Shell) -> None:
    """A key every 0.4 s, and within the rest a single pose comes, holds a moment, and goes.

    It moves none while it does, and draws a frame for the pose and one for sitting again.
    """
    sitting_over(shell, "a column typed in")
    # Sitting down can outlast a pause, so a pause's pose may be up: a key
    # first ends it, and its frame is drawn before the count starts.
    shell.send_key("F24")
    time.sleep(TYPING)
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
    assert drawn <= GLANCED, f"a small move drew {drawn} frames"


def test_paused_it_plays_a_little_in_place(shell: Shell) -> None:
    """Once the keys stop, a mild scene comes: the sitting cat's own moves, briefly, in place."""
    sitting_over(shell, "a column paused over")
    shell.send_key("F24")
    since = shell.mark()
    shell.reported(SITS, lambda said: (placed(said) or {}).get("pose") in MILD, since)
    began = time.monotonic()
    shell.reported(SITS, lambda said: (placed(said) or {}).get("pose") == "", since)
    lasted = time.monotonic() - began
    found, _ = reports(shell, since)
    poses = {at.get("pose") for at in found}
    assert poses <= {*MILD, ""}, f"a pause's scene posed {poses}"
    assert lasted <= SCENE, f"a pause's scene lasted {lasted:.2f} s"
    assert not any(walking(at) for at in found), f"a pause's scene walked: {found}"


def test_away_it_strolls_to_the_gutter_and_a_key_brings_it_back(shell: Shell) -> None:
    """Gone from the keys, it walks to the gutter beside its column; a key walks it back."""
    beside = shell.opened("a column beside the stroll")
    sat, seat = sitting_over(shell, "a column strolled from")
    shell.send_key("F24")
    since = shell.mark()
    shell.reported(
        SITS,
        lambda said: (at := placed(said)) is not None and bool(at.get("strolling")) and walking(at),
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
    assert any(walking(at) for at in found), f"it came back over {sat}, not {beside}, unwalked"


def test_away_it_bats_a_ball_of_yarn_off_and_walks_home(shell: Shell) -> None:
    """Still away, a ball of yarn rolls in, is batted and pounced on, and goes; it walks home.

    Its beats come in order, the yarn shown whole and then fading as it's sent off, and niri
    shows every frame of it. It walks home to where it sat.
    """
    frames, shown = plays_out(shell, "yarn", "the yarn")
    beats = beats_of(frames)
    assert in_order(beats, YARN), f"the yarn's beats came as {beats}"
    assert set(beats) >= YARN_SEEN, f"the yarn's beats came as {beats}"
    seen = [at["play"]["yarn"]["opacity"] for at in frames if at["play"]["yarn"]]
    whole = seen.index(1) if 1 in seen else None
    assert whole is not None, f"the yarn was never shown whole: {seen}"
    assert min(seen[whole:]) < 1, f"the yarn never faded as it went: {seen}"
    assert shown["unseen"] == 0, f"niri never showed {shown['unseen']} of its frames: {shown}"


def test_away_it_pounces_on_nothing_and_walks_home(shell: Shell) -> None:
    """A mild scene later, it eyes, wiggles and leaps on nothing, lands, and walks home."""
    frames, shown = plays_out(shell, "pounce", "a pounce")
    beats = beats_of(frames)
    assert in_order(beats, POUNCE), f"the pounce's beats came as {beats}"
    assert set(beats) >= POUNCE_SEEN, f"the pounce's beats came as {beats}"
    assert max(at["play"]["lift"] for at in frames) > 0, f"it never left the ground: {frames}"
    assert not any(at["play"]["yarn"] for at in frames), f"a pounce brought yarn: {frames}"
    assert shown["unseen"] == 0, f"niri never showed {shown['unseen']} of its frames: {shown}"
