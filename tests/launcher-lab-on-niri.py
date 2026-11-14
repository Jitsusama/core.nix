"""Time each chord's opening the one way every launcher change quotes, on niri.

A lab, not a check: modules/quickshell/checks/core.nix builds it as the `lab` of
the-launcher-opens-on-what-it-kept-on-niri, which no gate runs, and docs/testing.md's Timing the
Launcher says how its runs are taken a side at a time and read. Two labs gave chords 43 to 73 ms
and 173 to 225 ms presented (2026-11-11), for they opened different places, kept and whole, with
one shell or two sharing a builder, so neither said where chords stood. This one opens every
chord's place in turn, a round at a time, each from shut as its chord does, and prints each
opening's presented time from the ask with whether it was kept, so a run's places and rounds are
the same on both sides. Each place's line also splits its time at the shell's swap: what the
shell took to its frame, and what niri took after it to present it, since the VM's niri draws on
the CPU and once held about half its frames some 150 ms past the swap, which read as the
launcher's when only the whole was said (RTOIMQ, 2026-11-11). It fails only when an opening
never says it was presented. Beside it, a second test times each step into a place from home,
as Joel takes them, since a step is navigation and held to the same 50 ms. A third holds a place
open while its daemon's changes come, and prints the line the shell says of them as it closes,
since an update is held to it too: 6 of Machine's 20 on Joel's shell took longer (2026-11-11).
"""

from __future__ import annotations

import json
import os
import re
import statistics
import subprocess
import threading
import time
from pathlib import Path
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

# Each chord's place as niri's binds ask for it (modules/niri/binds.nix), and home as the
# launcher's own chord toggles it.
PLACES = (
    "home",
    "on",
    "windows",
    "clipboard",
    "emoji",
    "spend",
    "shortcuts",
    "displays",
    "capture",
)

# How many times each place is opened in a run, unless the run says otherwise.
ROUNDS = 10

# How many windows are open as it runs, so Windows has rows to make.
WINDOWS = 4

# The places stepped into from home, by the name home lists each by: Capture, whose step
# showed in 138 ms on Joel's shell (2026-11-11), and the two with most rows beside it.
STEPS = ("Capture", "Windows", "Emoji")

# The places held open while their daemons change, by the name home lists each by: Machine,
# whose figures change each second.
WATCHED = ("Machine",)

# How long each is held open a round, in seconds, for some twenty of Machine's changes.
WATCHING = 20.0


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell once it has kept home by itself, started once a run and never opened."""
    with launcher_on_niri.running() as started:
        started.launcher_until(lambda shown: shown.get("kept"))
        yield started


def openings(shell: Shell) -> list[Json]:
    """Give the last openings the launcher timed, each from its ask to its first frame."""
    return cast("list[Json]", json.loads(shell.as_account(shell.ipc("openings"))))


def latest(timed: list[Json]) -> int:
    """Give the number of the last opening timed, as only the last 20 are kept."""
    return cast("int", timed[-1].get("nth", len(timed))) if timed else 0


def opened(shell: Shell, place: str) -> Json:
    """Open a place from shut as its chord asks, and give that opening once niri presented it.

    It's asked once the shut launcher has kept home, as Joel's openings come seconds apart.
    """
    shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
    before = latest(openings(shell))
    words = ("toggle",) if place == "home" else ("collection", place)
    shell.as_account(shell.ipc(*words))
    each: list[Json] = []

    def presented(_: Json) -> bool:
        each[:] = openings(shell)
        return latest(each) > before and "presented" in each[-1]

    shell.launcher_until(presented)
    return each[-1]


def p90(spans: list[int]) -> int:
    """Give the span nine in ten openings came within."""
    ordered = sorted(spans)
    return ordered[min(len(ordered) - 1, round(0.9 * (len(ordered) - 1)))]


def said_of(place: str, presented: list[int], took: list[int]) -> str:
    """Say a place's openings: presented, and that split at the shell's swap of its frame."""
    niri = [whole - shell for whole, shell in zip(presented, took, strict=True)]
    return (
        f"lab-place {place}: {len(presented)} presented a median "
        f"{round(statistics.median(presented))} ms, p90 {p90(presented)}; the shell's frame a "
        f"median {round(statistics.median(took))} ms, niri's after it a median "
        f"{round(statistics.median(niri))} ms, p90 {p90(niri)}"
    )


def test_each_chord_opening_is_timed(shell: Shell, capsys: pytest.CaptureFixture[str]) -> None:
    """Open each chord's place from shut, round after round, and print each one's times.

    They're printed past pytest's capture, so the build's log holds them when it passes.
    """
    places = os.environ.get("LAB_PLACES", " ".join(PLACES)).split()
    rounds = int(os.environ.get("LAB_ROUNDS", str(ROUNDS)))
    windows = [shell.opened(f"Lab {nth}", app="foot") for nth in range(WINDOWS)]
    # Opened once untimed first: a fresh niri blurs behind its first surface only as that
    # shows, which held the first opening of every run 368 to 902 ms from its ask though it was
    # drawn in 11, against 38 and 42 with the blur off (H6EDLC, 2026-11-11). Joel's niri has
    # blurred since he logged in, so his openings never pay it.
    opened(shell, "home")
    shell.close_launcher()
    timed: dict[str, list[int]] = {place: [] for place in places}
    # From the ask to the shell's swap of the frame at the card's size, which `presented` runs
    # on from to niri's presented frame.
    took: dict[str, list[int]] = {place: [] for place in places}
    for round_ in range(rounds):
        # Each round starts one place further on, so no place always follows the same one.
        for place in places[round_ % len(places) :] + places[: round_ % len(places)]:
            opening = opened(shell, place)
            said = {
                "place": place,
                "round": round_,
                "presented": opening["presented"],
                "kept": bool(opening.get("kept")),
                "took": opening.get("took"),
                "niri": opening["presented"] - cast("int", opening["took"]),
                "rows": opening.get("rows"),
            }
            with capsys.disabled():
                print("lab-opening " + json.dumps(said), flush=True)
            if opening["presented"] < 0:
                pytest.fail(f"{place}'s opening closed unpresented: {opening}")
            timed[place].append(opening["presented"])
            took[place].append(cast("int", opening["took"]))
            shell.close_launcher()
            shell.launcher_until(lambda shown: not shown["open"])
    for window in windows:
        shell.close(window)
    for place, spans in timed.items():
        with capsys.disabled():
            print(said_of(place, spans, took[place]), flush=True)


def said_after(shell: Shell, mark: int, *wanted: str) -> list[str]:
    """Give the shell's log lines since the mark that hold any of the words wanted."""
    with shell.log.open("rb") as log:
        log.seek(mark)
        written = log.read().decode(errors="replace")
    return [line for line in written.splitlines() if any(word in line for word in wanted)]


def stepped(shell: Shell, name: str) -> tuple[Json, list[str]]:
    """Open home, find the place by its name, step in, and give the step's key and lines."""
    shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in(name.lower(), selecting=name)
    before = len(json.loads(shell.as_account(shell.ipc("keysPresented"))))
    mark = shell.mark()
    shell.step_in()
    keys: list[Json] = []

    def presented(_: Json) -> bool:
        keys[:] = json.loads(shell.as_account(shell.ipc("keysPresented")))
        return len(keys) > before

    shell.launcher_until(presented)
    # The step's own line is written as its frame is drawn, before it's presented, and only
    # for a step Joel could feel (launcher/Trials.js's felt).
    return keys[-1], said_after(shell, mark, '"→ ')


def test_each_places_changes_are_timed(shell: Shell, capsys: pytest.CaptureFixture[str]) -> None:
    """Hold each watched place open a while, round after round, and print its changes' lines."""
    places = os.environ.get("LAB_WATCHED", " ".join(WATCHED)).split()
    rounds = int(os.environ.get("LAB_ROUNDS", str(ROUNDS)))
    watching = float(os.environ.get("LAB_WATCHING", str(WATCHING)))
    opened(shell, "home")
    shell.close_launcher()
    for round_ in range(rounds):
        for name in places:
            stepped(shell, name)
            mark = shell.mark()
            # Held open for the changes the place's daemon makes by itself, which nothing
            # here can hurry: the time is the lab's measure, not a wait for a state.
            time.sleep(watching)
            shell.close_launcher()
            shell.launcher_until(lambda shown: not shown["open"])
            # The shell says them as it closes (Launcher.qml's keysPresentedSaid).
            with capsys.disabled():
                for line in said_after(shell, mark, " changes in "):
                    said = line.split("qml: ", 1)[-1]
                    print(f"lab-changes {name} round {round_}: {said}", flush=True)


def test_each_step_is_timed(shell: Shell, capsys: pytest.CaptureFixture[str]) -> None:
    """Step into each place from home, round after round, and print each step's times."""
    steps = os.environ.get("LAB_STEPS", " ".join(STEPS)).split()
    rounds = int(os.environ.get("LAB_ROUNDS", str(ROUNDS)))
    windows = [
        shell.opened(f"Lab {nth}", app="foot")
        for nth in range(int(os.environ.get("LAB_WINDOWS", str(WINDOWS))))
    ]
    opened(shell, "home")
    shell.close_launcher()
    timed: dict[str, list[int]] = {name: [] for name in steps}
    for round_ in range(rounds):
        for name in steps[round_ % len(steps) :] + steps[: round_ % len(steps)]:
            key, lines = stepped(shell, name)
            span = round(cast("float", key["at"]) - cast("float", key["from"]))
            timed[name].append(span)
            with capsys.disabled():
                print(f"lab-step {name} round {round_}: presented {span} ms", flush=True)
                for line in lines:
                    print(f"lab-step-line {name}: {line.split('qml: ', 1)[-1]}", flush=True)
            shell.close_launcher()
            shell.launcher_until(lambda shown: not shown["open"])
    for window in windows:
        shell.close(window)
    for name, spans in timed.items():
        with capsys.disabled():
            print(
                f"lab-step {name}: {len(spans)} presented a median "
                f"{round(statistics.median(spans))} ms, p90 {p90(spans)}",
                flush=True,
            )


# The row of Machine whose rows the drawer walk steps between, by its name, unless the run says
# otherwise: Memory's parts, whose drawers say kernel counts that move on every read.
DRAWN = "Memory"

# A drawer's line as the shell says it once niri presented it (launcher/Presented.js).
DRAWER_PRESENTED = re.compile(
    r'launcher: drawer for "([^"]+)" presented (\d+) ms from the key: .*, ([^,]+)$'
)


def drawer_said(shell: Shell, mark: int) -> re.Match[str] | None:
    """Give the first drawer said presented since the mark, waiting a while for niri's frame."""
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        for line in said_after(shell, mark, "drawer for "):
            found = DRAWER_PRESENTED.search(line)
            if found:
                return found
        time.sleep(launcher_on_niri.POLL)
    return None


def test_each_drawer_is_timed(shell: Shell, capsys: pytest.CaptureFixture[str]) -> None:
    """Walk the rows inside one of Machine's by letter, down and up, and print each drawer's line.

    A drawer served apart once came a round trip after its row, the row before's out meanwhile
    and never timed (VKCF3Z, 2026-11-12, bcbe9a24). So each key's drawer is said from the key,
    with how it came, and whether the drawer seen first once the selection moved was another
    row's: on both sides, as the frame before a drawer follows shows the last one on either.
    """
    within = os.environ.get("LAB_DRAWN", DRAWN)
    rounds = int(os.environ.get("LAB_ROUNDS", str(ROUNDS)))
    opened(shell, "home")
    shell.close_launcher()
    stepped(shell, "Machine")
    shell.down_to(within)
    shell.step_in()
    shell.normal()
    # Its own rows, each with a drawer, not the acts that follow them, as Copy, which have none.
    things = cast("list[Json]", shell.showing()["things"])
    drawn = [thing for thing in things if not str(thing["is"]).startswith("return:")]
    walk = "j" * (len(drawn) - 1) + "k" * (len(drawn) - 1)
    timed: list[int] = []
    others = unsaid = 0
    came: dict[str, int] = {}
    for round_ in range(rounds):
        for letter in walk:
            was = (shell.showing()["selected"] or {}).get("name")
            mark = shell.mark()
            shell.send_chars(letter)
            first: list[object] = []

            def reached(shown: Json, was: object = was, first: list[object] = first) -> bool:
                selected = (shown["selected"] or {}).get("name")
                drawer = shown["drawer"] or {}
                if selected != was and not first:
                    first.append(drawer.get("title") if drawer.get("shown") else None)
                return (
                    selected != was
                    and bool(drawer.get("shown"))
                    and (drawer.get("title") == selected)
                )

            shell.launcher_until(reached)
            name = (shell.showing()["selected"] or {}).get("name")
            other = first[0] not in {None, name}
            others += other
            found = drawer_said(shell, mark)
            if found is None:
                unsaid += 1
                # What the shell said of the drawer since the key, so one never said is read.
                with capsys.disabled():
                    for line in said_after(shell, mark, "drawer"):
                        print(f"lab-drawer-unsaid {line.split('qml: ', 1)[-1]}", flush=True)
            else:
                timed.append(int(found.group(2)))
                came[found.group(3)] = came.get(found.group(3), 0) + 1
            with capsys.disabled():
                print(
                    f"lab-drawer {within} round {round_}: {name}, "
                    f"{found.group(2) + ' ms, ' + found.group(3) if found else 'never said'}"
                    f"{', first showing ' + repr(first[0]) if other else ''}",
                    flush=True,
                )
    shell.close_launcher()
    shell.launcher_until(lambda shown: not shown["open"])
    with capsys.disabled():
        print(
            f"lab-drawer {within}: {len(timed)} presented a median "
            f"{round(statistics.median(timed)) if timed else 0} ms, "
            f"p90 {p90(timed) if timed else 0}; {unsaid} never said; {others} showed another "
            f"row's drawer first; came {json.dumps(came)}",
            flush=True,
        )


def test_a_chord_through_a_restart(shell: Shell, capsys: pytest.CaptureFixture[str]) -> None:
    """Restart the shell as a switch does, say a toggle a while after its stop, and print it.

    Each round's chord is said by launcher-say the given seconds after the old shell is told
    to stop, as Joel's Super+Space may come at any moment of a switch, and the round says how
    launcher-say ended, whether the launcher opened for the chord and how long after it, and
    when the new shell answered. The socket goes first, as the unit's ExecStop takes it
    (modules/quickshell/home.nix), unless LAB_UNLINK is 0, as before it did.
    """
    spaced = os.environ.get("LAB_OFFSETS", "0 0.1 0.2 0.4 0.8 1.2 1.6 2 3 5")
    unlinks = os.environ.get("LAB_UNLINK", "1") == "1"
    # How long a round waits once the new shell has kept home for the chord's
    # opening, before it says the chord was lost.
    kept_long_enough = 3
    for offset in [float(each) for each in spaced.split()]:
        shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
        if shell.process is None:
            pytest.fail("the lab's shell isn't running")
        heard: dict[str, object] = {}
        stopping = time.monotonic()

        def chorded(heard: dict[str, object] = heard) -> None:
            heard["at"] = time.monotonic()
            said = subprocess.run(
                [os.environ["LAUNCHER_SAY"], "toggle"], capture_output=True, check=False
            )
            heard["path"] = f"launcher-say exited {said.returncode}"

        chord = threading.Timer(offset, chorded)
        chord.start()
        if unlinks:
            (Path(os.environ["XDG_RUNTIME_DIR"]) / "launcher.sock").unlink(missing_ok=True)
        shell.process.terminate()
        shell.process.wait(timeout=10)
        stopped = time.monotonic()
        given = os.environ.get("QS_GIVEN", "").split()
        with shell.log.open("ab") as log:
            shell.process = subprocess.Popen(
                ["qs", "-p", str(shell.folder), *given],
                stdout=log,
                stderr=subprocess.STDOUT,
            )
        answering = opened_at = kept = None
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            status, said = shell.execute(shell.ipc("showing"))
            now = time.monotonic()
            if status == 0:
                shown = json.loads(said)
                answering = answering or now
                opened_at = opened_at or (now if shown["open"] else None)
                kept = kept or (now if shown.get("kept") else None)
            if opened_at is not None or (kept is not None and now - kept > kept_long_enough):
                break
            time.sleep(launcher_on_niri.POLL)
        chord.join()

        def since(moment: float | None, start: float) -> str:
            return "never" if moment is None else f"{round((moment - start) * 1000)} ms"

        chord_at = cast("float", heard["at"])
        timed = openings(shell) if opened_at is not None else []
        with capsys.disabled():
            print(
                f"lab-restart chord {offset} s after the stop began: {heard['path']}; "
                f"stopped in {since(stopped, stopping)}; answering {since(answering, stopped)} "
                f"and kept {since(kept, stopped)} after; opened {since(opened_at, chord_at)} "
                f"after the chord, presented {timed[-1].get('presented') if timed else 'never'}",
                flush=True,
            )
        if opened_at is not None:
            shell.close_launcher()
            shell.launcher_until(lambda shown: not shown["open"])
