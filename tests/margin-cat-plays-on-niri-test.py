"""The margin's cat grows livelier the less Joel types, on niri, as its scale says.

modules/quickshell/checks/margin.nix runs this through pytest in tests/nested-niri.sh, with the
account's whole shell, its cat counting Joel paused after 3 s with no keys and away after 8 s,
and its scenes coming fifteen times sooner than its own rests, so each is seen in seconds.
Which scene comes at each level, how long each pose holds and where a stroll goes are held in
Qt's engine (modules/margin-cat/tests/margin-cat.qml); here, only that niri's idle timers and
real columns reach it: a single pose held a moment while the keys come, which moves it none
and draws a frame each way, mild scenes once they stop, a stroll to the gutter beside its
column once they've been gone a while, and a key bringing it straight back.
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
# How soon a key brings a stroll back over its column.
BACK = 1.0
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


def reports(shell: Shell, since: int) -> tuple[list[Json], int]:
    """Give each place the cat has said it's at since the mark, and the mark they end at."""
    said, since = shell.said_since(since)
    found = [placed(report[len(SITS) :]) for report in said if report.startswith(SITS)]
    return [at for at in found if at is not None], since


def cat_frames(shell: Shell) -> int:
    """Give the frames the cat's windows drew since the count started (Frames.qml)."""
    drawn = json.loads(shell.as_account(f"qs -p {shell.folder} ipc call frames said"))
    return sum(count for name, count in drawn.items() if name.startswith("cat "))


def sitting_over(shell: Shell, title: str) -> Json:
    """Open a column and wait until the cat sits over it, still and posing nothing."""
    since = shell.mark()
    window = shell.opened(title)
    shell.reported(
        SITS,
        lambda said: (
            (at := placed(said)) is not None
            and at.get("window") == window["id"]
            and not at["walking"]
        ),
        since,
    )
    return window


def test_while_he_types_it_makes_one_small_move_in_place(shell: Shell) -> None:
    """A key every 0.4 s, and within the rest a single pose comes, holds a moment, and goes.

    It moves none while it does, and draws a frame for the pose and one for sitting again.
    """
    sitting_over(shell, "a column typed in")
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
    assert not any(at["walking"] for at in found), f"a pause's scene walked: {found}"


def test_away_it_strolls_to_the_gutter_and_a_key_brings_it_back(shell: Shell) -> None:
    """Gone from the keys, it strolls to the gutter beside its column; a key has it back."""
    beside = shell.opened("a column beside the stroll")
    sat = sitting_over(shell, "a column strolled from")
    shell.send_key("F24")
    since = shell.mark()
    shell.reported(SITS, lambda said: bool((placed(said) or {}).get("strolling")), since, STROLLS)
    found, _ = reports(shell, since)
    walked = [at for at in found if at["walking"]]
    assert walked, f"it strolled without walking: {found}"
    assert {at["cells"] for at in walked} == {8}, f"its walk was drawn as {walked}"
    since = shell.mark()
    keyed = time.monotonic()
    shell.send_key("F24")
    shell.reported(
        SITS,
        lambda said: (
            (at := placed(said)) is not None
            and not at.get("strolling")
            and not at["walking"]
            and at.get("window") == sat["id"]
        ),
        since,
    )
    back = time.monotonic() - keyed
    assert back <= BACK, f"a key took {back:.2f} s to bring it back over {sat}, not {beside}"
