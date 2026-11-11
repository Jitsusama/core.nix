"""Time each key typed from home as Joel types them, and say what its search spent, on niri.

A lab, not a check: modules/quickshell/checks/core.nix builds it as the `keys-lab` of
the-launcher-opens-on-what-it-kept-on-niri, which no gate runs, and docs/testing.md's Timing the
Launcher says how its runs are read. Joel's first letter from home spent 15 and 19 ms of its
search before searching, where a later one spent 6 to 9 (2026-11-11), so this one opens home round
after round and types each word a letter at a time, each letter once the one before is shown, as
he types, then prints each key's line and each part's median by key. Given LAB_PROFILE, it holds
Qt's profiler on the shell (QMLPROFILER, the shell started with `--debug`), and says which
bindings, handlers and functions took the time inside each span the shell marks with `lab-span
<name> <letters>` and `lab-span-end` lines, which only a lab's own patch to the shell writes. It
fails only when a key is never shown.
"""

from __future__ import annotations

import os
import re
import statistics
import subprocess
import time
import xml.etree.ElementTree as ET
from collections import defaultdict
from pathlib import Path
from typing import IO, TYPE_CHECKING, cast

import launcher_on_niri

if TYPE_CHECKING:
    import pytest
    from launcher_on_niri import Shell

# The words typed from home, each a letter at a time, as Joel typed them on 2026-11-11.
WORDS = ("meet", "om")

# How many times each word is typed from an opening of home, unless the run says otherwise.
ROUNDS = 10

# How long a key's line may take to be said once its letters are shown, as it's said when its
# frame is swapped; one quicker than the shell says (Trials.js's FELT_MS) is never said.
SAYING = 2.0

# The port the shell is opened to Qt's profiler on, as the check gives QS_GIVEN.
PORT = "3768"

# A key's line, as Trials.qml says it, and each part of it the lab reads by name.
KEY = re.compile(r'launcher: "(?P<typed>[^"]*)" took (?P<took>\d+) ms to its frame, (?P<rest>.*)')
PARTS = ("searching", "rows", "answers", "before", "counting", "typed", "words", "readying")

# How long the profiler may take to write its trace once asked, in seconds.
FLUSHING = 60

# How many of a span's costliest places are said.
COSTLIEST = 25


def part(line: str, name: str) -> int | None:
    """Give a part of a key's line by its name, or None where the line doesn't say it."""
    # The search's whole comes before its name, every other part after its own.
    pattern = r"(\d+) ms searching" if name == "searching" else rf"(?:^|[ (]){name} (\d+)"
    found = re.search(pattern, line)
    return int(found.group(1)) if found else None


def keys_said(shell: Shell, since: int) -> dict[str, str]:
    """Give each key's line the shell has said since the mark, by what was typed."""
    with shell.log.open("rb") as log:
        log.seek(since)
        written = log.read().decode(errors="replace")
    said = {}
    for line in written.splitlines():
        found = KEY.search(line)
        if found:
            said[found.group("typed")] = found.group("took") + " ms, " + found.group("rest")
    return said


def shown_typed(shell: Shell, typed: str) -> None:
    """Wait until the launcher shows what's typed as the letters given."""
    shell.launcher_until(lambda shown: shown["typed"] == typed)


def typed_from_home(shell: Shell, word: str) -> dict[str, str]:
    """Open home, type the word a letter at a time, and give each key's line, then close."""
    shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
    since = shell.mark()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    for end in range(1, len(word) + 1):
        shell.send_chars(word[end - 1])
        shown_typed(shell, word[:end])
    deadline = time.monotonic() + SAYING
    said = keys_said(shell, since)
    while word not in said and time.monotonic() < deadline:
        time.sleep(launcher_on_niri.POLL)
        said = keys_said(shell, since)
    shell.close_launcher()
    shell.launcher_until(lambda shown: not shown["open"])
    return said


class Spent:
    """What one place took inside a span: how often it ran, its own time and its whole."""

    def __init__(self) -> None:
        """Begin with nothing spent."""
        self.count = 0
        self.own = 0.0
        self.whole = 0.0


def profiled(trace: Path) -> dict[str, dict[str, Spent]]:
    """Give what each place took inside each span of the profiler's trace, by the span's name.

    A place is a binding, handler or function by its kind, file and line. Its own time leaves out
    what it called that the profiler recorded too, so a handler holding the whole search is not
    named for it; its whole is all of it.
    """
    root = ET.parse(trace).getroot()  # noqa: S314, the trace this run wrote
    places: dict[str, str] = {}
    # The launcher's search function, whose every call is a span of its own, `search`, beside
    # those the shell marks, since the profiler's own trace may keep no message.
    searching: set[str] = set()
    for event in root.iter("event"):
        where = Path(event.findtext("filename") or "").name
        kind = event.findtext("type") or ""
        details = (event.findtext("details") or "").replace("\n", " ")[:60]
        places[event.get("index", "")] = f"{kind} {where}:{event.findtext('line')} {details}"
        if where == "Launcher.qml" and details == "search":
            searching.add(event.get("index", ""))
    spans: dict[str, dict[str, Spent]] = defaultdict(lambda: defaultdict(Spent))
    span = ""
    search_ends = 0
    # The ranges still open where the next starts: when each ends, and its places' tallies.
    held: list[tuple[int, list[Spent]]] = []
    for each in sorted(root.iter("range"), key=lambda one: int(one.get("startTime", "0"))):
        start = int(each.get("startTime", "0"))
        text = each.get("text")
        if text is not None:
            span = text.split()[1] if text.startswith("lab-span ") else ""
            continue
        index = each.get("eventIndex", "")
        ends = start + int(each.get("duration", "0"))
        took = int(each.get("duration", "0")) / 1e6
        while held and held[-1][0] <= start:
            held.pop()
        for parent in held[-1][1] if held else []:
            parent.own -= took
        named = [name for name in (span, "search" if start < search_ends else "") if name]
        tallies = [spans[name][places.get(index, "?")] for name in named]
        for tally in tallies:
            tally.count += 1
            tally.own += took
            tally.whole += took
        if index in searching and start >= search_ends:
            search_ends = ends
        held.append((ends, tallies))
    return spans


def census(trace: Path) -> list[str]:
    """Give how many events and ranges of each kind the trace holds, and a range of each."""
    root = ET.parse(trace).getroot()  # noqa: S314, the trace this run wrote
    kinds = {one.get("index", ""): one.findtext("type") or "" for one in root.iter("event")}
    counted: dict[str, int] = defaultdict(int)
    sample: dict[str, str] = {}
    for each in root.iter("range"):
        kind = kinds.get(each.get("eventIndex", ""), "?")
        counted[kind] += 1
        sample.setdefault(kind, str(each.attrib))
    return [f"{kind}: {count} ranges, as {sample[kind]}" for kind, count in counted.items()]


def say_profiled(trace: Path, said: list[str], capsys: pytest.CaptureFixture[str]) -> None:
    """Print each span's costliest places by their own time, or what the trace held instead.

    With no span, what the profiler and the shell said of the debugger is printed too, as what's
    said leaves the run with it.
    """
    spans = profiled(trace) if trace.exists() else {}
    with capsys.disabled():
        for line in census(trace) if trace.exists() else []:
            print(f"lab-profile held {line}", flush=True)
        if not spans:
            for line in said:
                print(f"lab-profile said: {line}", flush=True)
            whole = ET.parse(trace).getroot() if trace.exists() else None  # noqa: S314
            each = list(whole.iter("range")) if whole is not None else []
            texts = [one.get("text") for one in each if one.get("text")]
            ranges = len(each)
            print(
                f"lab-profile: no span profiled, of {ranges} ranges; messages {texts[:5]}",
                flush=True,
            )
        for span, places in spans.items():
            ordered = sorted(places.items(), key=lambda one: one[1].own, reverse=True)
            for place, spent in ordered[:COSTLIEST]:
                print(
                    f"lab-profile {span}: own {spent.own:.2f} ms, whole {spent.whole:.2f}"
                    f" over {spent.count}, {place}",
                    flush=True,
                )


def flushed(profiler: subprocess.Popen[bytes], trace: Path) -> None:
    """Have the profiler stop recording and write its trace, then quit, while the shell runs.

    A shell stopped while it records drops the connection before its trace is sent, and the
    profiler writes none, so the trace is asked for first, and taken once it stops growing.
    """
    # Started with a pipe to its commands, as the test starts it.
    commands = cast("IO[bytes]", profiler.stdin)
    commands.write(b"f\n")
    commands.flush()
    deadline = time.monotonic() + FLUSHING
    size = -1
    while time.monotonic() < deadline:
        time.sleep(1)
        now = trace.stat().st_size if trace.exists() else -1
        if now > 0 and now == size:
            break
        size = now
    commands.write(b"q\n")
    commands.flush()


def test_each_home_key_is_timed(capsys: pytest.CaptureFixture[str], tmp_path: Path) -> None:
    """Type each word from home, round after round, and print each key's line and medians.

    They're printed past pytest's capture, so the build's log holds them when it passes.
    """
    words = os.environ.get("LAB_WORDS", " ".join(WORDS)).split()
    rounds = int(os.environ.get("LAB_ROUNDS", str(ROUNDS)))
    trace = tmp_path / "keys.qtd"
    profiling = tmp_path / "profiler.log"
    profiler: subprocess.Popen[bytes] | None = None
    parts: dict[str, dict[str, list[int]]] = defaultdict(lambda: defaultdict(list))
    with launcher_on_niri.running() as shell:
        shell.launcher_until(lambda shown: shown.get("kept"))
        if os.environ.get("LAB_PROFILE"):
            profiler = subprocess.Popen(  # noqa: S603, the profiler the check gives
                [
                    os.environ["QMLPROFILER"],
                    "-a",
                    "localhost",
                    "-p",
                    PORT,
                    "--interactive",
                    "-o",
                    str(trace),
                    "--include",
                    "binding,javascript,handlingsignal,debugmessages",
                ],
                stdin=subprocess.PIPE,
                stdout=profiling.open("wb"),
                stderr=subprocess.STDOUT,
            )
        # Typed once untimed first, as the openings' lab opens once, for what a fresh shell
        # pays only once: its first search compiles what Joel's has run since he logged in.
        typed_from_home(shell, words[0])
        for round_ in range(rounds):
            for word in words[round_ % len(words) :] + words[: round_ % len(words)]:
                said = typed_from_home(shell, word)
                for end in range(1, len(word) + 1):
                    typed = word[:end]
                    line = said.get(typed)
                    with capsys.disabled():
                        print(f"lab-key {round_} {typed!r}: {line or 'under the felt'}", flush=True)
                    took = part(f"took {line}", "took") if line else None
                    parts[typed]["took"].append(took or 0)
                    for name in PARTS:
                        parts[typed][name].append((part(line, name) or 0) if line else 0)
        if profiler is not None:
            flushed(profiler, trace)
    heard: list[str] = []
    if profiler is not None:
        heard.append(f"the profiler ended {profiler.wait(timeout=60)}")
        heard.extend(profiling.read_text(errors="replace").splitlines())
        logged = shell.log.read_text(errors="replace").splitlines()
        heard.extend(line for line in logged if "ebug" in line or "qmltooling" in line)
    for typed, each in parts.items():
        medians = ", ".join(f"{name} {statistics.median(spans):g}" for name, spans in each.items())
        with capsys.disabled():
            print(f"lab-keys {typed!r}: {len(each['took'])} typed, medians {medians}", flush=True)
    if profiler is not None:
        say_profiled(trace, heard, capsys)
