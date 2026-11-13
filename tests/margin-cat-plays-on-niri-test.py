"""The margin's cat grows livelier the less Joel types, on niri, as its scale says.

modules/quickshell/checks/margin.nix runs this through pytest in tests/nested-niri.sh, with the
account's whole shell, its cat counting Joel paused after 3 s with no keys and away after 8 s,
and its scenes coming fifteen times sooner than its own rests, so each is seen in seconds.
Which play comes at each level, how long each pose holds and where a wander goes are held in
Qt's engine (modules/margin-cat/tests/margin-cat.qml); here, only that niri's idle timers and
real columns reach it: a single pose held a moment while the keys come, which moves it none
and draws a frame each way, plays in place once they stop, and once they've been gone a while
a hop that rises and lands where it sat, and a wander a few cells along the margin, a key
ending it at once, the cat walking back.
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
# How long a play in place may last, and how long Joel is gone before a move may come: 8 s
# here, and as many plays as make each move all but certain.
SCENE, STROLLS = 2.5, timedelta(seconds=60)
# How far a wander goes, in the cat's cells, and the cell and a half its walk overshoots.
WANDERS = (16, 40)
# How soon a key ends a stroll, and how long the walk back over its column may take: a
# gentle long walk, at most about 8 s, and some for a loaded builder.
BACK, HOME = 1.0, timedelta(seconds=15)
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
    """Once the keys stop, a play comes: the sitting cat's own moves, briefly, in place."""
    sitting_over(shell, "a column paused over")
    shell.send_key("F24")
    since = shell.mark()
    shell.reported(SITS, lambda said: (placed(said) or {}).get("pose") in IN_PLACE, since)
    began = time.monotonic()
    shell.reported(SITS, lambda said: (placed(said) or {}).get("pose") == "", since)
    lasted = time.monotonic() - began
    found, _ = reports(shell, since)
    poses = {at.get("pose") for at in found}
    assert poses <= IN_PLACE | SITTING, f"a pause's play posed {poses}"
    assert lasted <= SCENE, f"a pause's play lasted {lasted:.2f} s"
    assert not any(walking(at) or at.get("lift") for at in found), f"a pause moved: {found}"


def test_away_it_hops_and_lands_where_it_sat(shell: Shell) -> None:
    """Gone from the keys, it hops: rising from a crouch and landing, never moving along."""
    _, seat = sitting_over(shell, "a column hopped over")
    since = shell.mark()
    shell.reported(SITS, lambda said: ((placed(said) or {}).get("lift") or 0) > 0, since, STROLLS)
    risen = shell.mark()
    shell.reported(SITS, lambda said: (placed(said) or {}).get("lift") == 0, risen)
    found, _ = reports(shell, since)
    hop = [at for at in found if (at.get("lift") or 0) > 0]
    assert {at.get("pose") for at in hop} == {"pouncing"}, f"it hopped as {hop}"
    assert all(abs(at["x"] - seat["x"]) <= 1 for at in hop), f"a hop moved it along: {hop}"
    # A hop is a cell or two high, and a nested cell is a pixel, so its arc's shape is the Qt
    # case's; here, only that niri draws it in the air over more than a frame.
    assert len(hop) > 1, f"a hop showed for a single frame: {hop}"


def test_away_it_wanders_and_a_key_brings_it_back(shell: Shell) -> None:
    """Gone from the keys, it wanders a few cells along the margin; a key walks it back."""
    sat, seat = sitting_over(shell, "a column wandered from")
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
