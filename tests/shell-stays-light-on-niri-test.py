"""Opening every place again keeps no more of the shell's JavaScript heap live, on niri.

modules/quickshell/checks/shell.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts. A place's first opening builds what it keeps, so the first round
may grow the heap; every round after it must land where the one before it did. Joel's shell grew
from 108 MB a minute after a start to 150 and more with use (2026-11-11), while the lab fed his
records kept 38 to 42 MB live after every round, so a round that keeps more each time is a leak,
and this is where it's caught.

Live is what Qt's collector says is left after the collection each closing runs (Launcher.qml's
collectClosed), from its allocatorStats, which only the detailed log keeps at debug.
"""

from __future__ import annotations

import os
import re
import statistics
import time
from pathlib import Path
from typing import TYPE_CHECKING

import launcher_on_niri

if TYPE_CHECKING:
    import pytest
    from launcher_on_niri import Shell

# The places the desktop's chords open, as Joel opens them most.
PLACES = ("on", "windows", "clipboard", "emoji", "spend", "shortcuts", "displays", "capture")
# What's typed at home, a letter at a time, as Joel's worst openings began.
TYPED = ("c", "ca", "q", "n", "me")
# How much more a round may keep live than the one before it, past the noise of when the
# collector last ran: the lab's medians moved under 3% a round.
GROWTH = 1.10


def collections_said(shell: Shell) -> int:
    """Count the collections the shell has said it ran after a closing."""
    return shell.log.read_text(errors="replace").count("collected after closing")


def closed_and_collected(shell: Shell) -> None:
    """Close the launcher, and wait for the collection its closing runs a moment after."""
    before = collections_said(shell)
    shell.close_launcher()
    shell.launcher_until(lambda shown: not shown["open"])
    deadline = time.monotonic() + 10
    while collections_said(shell) == before:
        assert time.monotonic() < deadline, f"a closing ran no collection:\n{shell.tail()}"
        time.sleep(0.1)


def opened_and_closed(shell: Shell, place: str) -> None:
    """Open a place from shut as its chord asks, wait for it to show, and close it."""
    shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
    shell.as_account(shell.ipc("collection", place))
    shell.launcher_ready()
    closed_and_collected(shell)


def searched_and_closed(shell: Shell, text: str) -> None:
    """Open home, type a few letters as Joel does, and close."""
    shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in(text)
    closed_and_collected(shell)


def live_after_closings(shell: Shell) -> list[int]:
    """Give the bytes Qt's collector left live at each closing's collection, in order."""
    runtime = Path(os.environ["XDG_RUNTIME_DIR"]) / "quickshell"
    kept = sorted(runtime.rglob("*.qslog"))
    assert kept, f"the shell kept no detailed log under {runtime}"
    read = shell.execute(f"qs log -r 'qt.qml.gc.allocatorStats=true' {kept[-1]}")[1]
    live: list[int] = []
    last = None
    for line in read.splitlines():
        if match := re.search(r"Regular item memory after GC: (\d+)", line):
            last = int(match.group(1))
        elif "collected after closing" in line and last is not None:
            live.append(last)
    return live


def test_a_round_of_every_place_keeps_no_more_live(capsys: pytest.CaptureFixture[str]) -> None:
    """Open every place and type at home, three rounds, each after the first landing level."""
    with launcher_on_niri.running() as shell:
        shell.launcher_until(lambda shown: shown.get("kept"))
        rounds = rounds_kept(shell)
    medians = [round(statistics.median(each) / 1024) for each in rounds]
    # Said whether it passes, so a change's cost is read from its run.
    with capsys.disabled():
        print(f"live after each round's closings, medians in kB: {medians}", flush=True)
    assert all(rounds), f"a round's closings weren't collected: {rounds}"
    second, third = medians[1], medians[2]
    assert third <= second * GROWTH, (
        f"a round of every place kept {third} kB live against {second} kB the round before"
        f" (medians by round, in kB: {medians}), so something each opening makes is never let go"
    )


def rounds_kept(shell: Shell) -> list[list[int]]:
    """Give the bytes left live at each closing of three rounds of every place, by round."""
    rounds: list[list[int]] = []
    seen = len(live_after_closings(shell))
    for _ in range(3):
        for place in PLACES:
            opened_and_closed(shell, place)
        for text in TYPED:
            searched_and_closed(shell, text)
        # The detailed log is written a little behind the shell, so its last lines may lag.
        want = seen + len(PLACES) + len(TYPED)
        deadline = time.monotonic() + 10
        live = live_after_closings(shell)
        while len(live) < want and time.monotonic() < deadline:
            time.sleep(0.5)
            live = live_after_closings(shell)
        rounds.append(live[seen:])
        seen = len(live)
    return rounds
