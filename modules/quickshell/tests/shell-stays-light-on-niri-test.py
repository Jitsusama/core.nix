"""Opening every place again keeps no more of the shell's JavaScript heap live, on niri.

modules/quickshell/checks/shell.nix runs this through pytest inside tests/lib/nested-niri.sh, with
the shell launcher_on_niri.py starts. A place's first opening builds what it keeps, so the first
round may grow the heap; every round after it must land where the one before it did. Joel's shell
grew from 108 MB a minute after a start to 150 and more with use (2026-11-11), while the lab fed his
records kept 38 to 42 MB live after every round, so a round that keeps more each time is a leak,
and this is where it's caught.

Live is what Qt's collector says is left after the collection each closing runs (Launcher.qml's
collectClosed), from its allocatorStats, which only the detailed log keeps at debug.

Each closing's line also says what jemalloc holds, found by name in Quickshell's process, so a
Quickshell whose jemalloc stopped answering to that name is caught here, where it really runs.
The shell's Qt makes the heap anonymous memory, so each of its pages is held once
(qtdeclarative-heap-held-once.patch), and the shell's own maps must show no memfd of it. And
some closing must lend resident free pages back, which catches a launcher that stopped lending and
a heap the kernel won't take lent (memory not anonymous refuses it, and a refusal isn't counted).
The kernel must then say it holds some of the shell lent, a figure for the whole process, so it
holds that the line reads it, not that the heap alone lent it. That lending keeps the pages until
the kernel needs them, rather than giving them back, is the heap test's lendsBack to hold.

What the heap holds resident must not climb round on round either, as live mustn't: Joel's shell
held 149 MB at 23:09 and 252 MB at 00:13 with live flat at 60 to 65 MB (2026-11-25), and each
closing's line says how much of what's held is resident, which counts what a page the kernel took
no longer costs. A lent page counts as resident until the kernel takes it, and a page swapped out,
to zswap's pool or the disk, until it's touched again, so a builder short of memory may read a
round low and the next one at its true height. The kernel's counts of those are the whole
machine's, so they bound what it could have taken of the shell: the round before is let that much
higher, which on a quiet builder is nothing and on a busy one says why the bound was loose.
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


def resident_after_closings(shell: Shell) -> list[int]:
    """Give the MB each closing said the heap held resident, in order, -1 where unknown."""
    said = shell.log.read_text(errors="replace")
    return [
        int(each.split()[0]) if each[0].isdigit() else -1
        for each in re.findall(r" MB held, (\d+ MB|an unknown amount) of it resident,", said)
    ]


def pages_taken() -> int:
    """Give the pages the kernel has taken anywhere since it started, lent or swapped out.

    Lent pages it took, pages written to swap and pages stored in zswap's pool; a count it doesn't
    keep counts none.
    """
    vmstat = Path("/proc/vmstat").read_text()
    counted = (
        re.search(rf"^{event} (\d+)$", vmstat, re.MULTILINE)
        for event in ("pglazyfreed", "pswpout", "zswpout")
    )
    return sum(int(found.group(1)) for found in counted if found)


def test_a_round_of_every_place_keeps_no_more_live(capsys: pytest.CaptureFixture[str]) -> None:
    """Open every place and type at home, three rounds, each after the first landing level."""
    # From before the shell starts, since a page taken in any round before the third lowers it.
    started = pages_taken()
    with launcher_on_niri.running() as shell:
        shell.launcher_until(lambda shown: shown.get("kept"))
        rounds, residents, taken = rounds_kept(shell)
        said = shell.log.read_text(errors="replace")
        jemalloc = re.search(r"collected after closing.*; jemalloc \d+ MB in use", said)
        assert jemalloc, f"no closing said what jemalloc holds:\n{shell.tail()}"
        lendings = re.findall(
            r" (\d+) MB of free pages lent in \d+ ms,"
            r" the kernel holding (\d+ MB|an unknown amount) of the shell lent",
            said,
        )
        closings = said.count("launcher: collected after closing in ")
        assert len(lendings) == closings, (
            f"{closings - len(lendings)} of {closings} closings didn't say what they lent"
            f" as this check reads it:\n{shell.tail()}"
        )
        lent = [int(each) for each, _ in lendings]
        held = [int(each.split()[0]) if each[0].isdigit() else -1 for _, each in lendings]
        assert any(lent), f"no closing lent its heap's free pages back: {lent}\n{shell.tail()}"
        assert any(each > 0 for each in held), (
            f"the kernel held nothing of the shell lent after any closing: {held}\n{shell.tail()}"
        )
        assert shell.process, "the shell's process was never started"
        maps = Path(f"/proc/{shell.process.pid}/maps").read_text().splitlines()
        memfds = [line for line in maps if "/memfd:JSGCHeap" in line]
        assert not memfds, f"the shell maps its heap from a memfd {len(memfds)} times"
    medians = [round(statistics.median(each) / 1024) for each in rounds]
    resident = [float(statistics.median(each)) if each else 0.0 for each in residents]
    # A take in the third round only lowers it, so only what was taken before it counts.
    allowed = (taken[1] - started) * os.sysconf("SC_PAGE_SIZE") / 2**20
    # Said whether it passes, so a change's cost is read from its run.
    with capsys.disabled():
        print(f"live after each round's closings, medians in kB: {medians}", flush=True)
        print(
            f"resident after each round's closings, medians in MB: {resident}, the machine's"
            f" taken pages before the third letting the second {allowed:.1f} MB higher",
            flush=True,
        )
    assert all(rounds), f"a round's closings weren't collected: {rounds}"
    second, third = medians[1], medians[2]
    assert third <= second * GROWTH, (
        f"a round of every place kept {third} kB live against {second} kB the round before"
        f" (medians by round, in kB: {medians}), so something each opening makes is never let go"
    )
    closings = len(PLACES) + len(TYPED)
    assert all(len(each) == closings for each in residents), (
        f"a round's closings didn't each say what's held resident as this check reads it:"
        f" {residents}"
    )
    assert all(each >= 0 for said in residents for each in said), (
        f"the kernel wouldn't say which of the heap's pages are resident: {residents}"
    )
    assert resident[2] <= (resident[1] + allowed) * GROWTH, (
        f"a round of every place held {resident[2]} MB resident against {resident[1]} MB the"
        f" round before, which the kernel could have taken {allowed:.1f} MB of (medians by"
        f" round, in MB: {resident}), so the heap keeps pages it no longer uses"
    )


def rounds_kept(shell: Shell) -> tuple[list[list[int]], list[list[int]], list[int]]:
    """Give the bytes left live, and the MB held resident, at each closing of three rounds.

    And the pages the kernel had taken anywhere by the end of each round, as pages_taken counts.
    """
    rounds: list[list[int]] = []
    residents: list[list[int]] = []
    taken: list[int] = []
    seen = len(live_after_closings(shell))
    for _ in range(3):
        # Said in the shell's own log as each closing's collection is waited for, so never behind.
        before = len(resident_after_closings(shell))
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
        residents.append(resident_after_closings(shell)[before:])
        taken.append(pages_taken())
        seen = len(live)
    return rounds, residents, taken
