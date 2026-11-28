"""Time each key typed from home as Joel types them, and say what its search spent, on niri.

A lab, not a check: modules/quickshell/checks/core.nix builds it as the `keys-lab` of
the-launcher-opens-on-what-it-kept-on-niri, which no gate runs, and docs/testing.md's Timing the
Launcher says how its runs are read. Joel's first letter from home spent 15 and 19 ms of its
search before searching, where a later one spent 6 to 9 (2026-11-11), so this one opens home round
after round and types each word a letter at a time, each letter once the one before is shown, as
he types, then prints each key's line and each part's median by key. Given LAB_PROFILE, it holds
Qt's profiler on the shell (QMLPROFILER, the shell started with `--debug`), and says which
bindings, handlers and functions took the most of their own time in the launcher's searches and
each of their parts, as the key's line splits them. Given LAB_STEP, it steps into that place
from home after each round's words, and given LAB_PROGRAMS, it installs that many programs
first, so a step into Programs is profiled over as many rows as Joel's. Given LAB_NAMED, desktop
ids and names as `google-chrome=Google Chrome`, comma between, it installs those too, each with
two actions, so a program's own page, whose provider keys it by its id, is there to step into,
by Tab as Joel goes into one given LAB_STEP_KEY=tab (Return, the default, starts a program).
Given LAB_BACK too, it steps back out of that place with - each round and prints that step's
line, and given LAB_WINDOWS, it opens that many windows first, so Windows holds some. Given
LAB_AGENTS, it lays that many live sessions and serves them through the realm's node and the
agents daemon (agents_served.py), so a step into Agents lays as many agent rows as Joel's, one
of them moving before each step as his do between his looks. Given
LAB_CHORD, a place's name as its chord asks for it, it opens that place from shut each round
and steps out of it to home with nothing typed, as Joel's Super+Tab then - does; given home, it
opens home by the shell's toggle and closes it again, as his Super does. Every such opening here
opens on what home kept, unless LAB_WHOLE=first asks for it whole: it restarts the shell before
each round, its first opening being whole until home is first kept. Profiled, each opening's
toggled() call is then split where openWorkDone returned, into its working and its settling, and
the wait before the shell heard its frame swapped is a span of its own; given LAB_TRACES, each
trace is printed too, packed, so it can be read again later.
Given LAB_WITHIN too, names of the launcher's functions, it says what their calls took and what
ran inside them. Given LAB_CHANGES, it
leaves the first word typed at home each round while a window holding it in its title takes that
many new titles, and prints what the shell says of those changes, as Joel's daemons change rows
under his typed words at 17 to 35 ms in the shell (2026-11-15). It fails when a key is never
shown, and at once when LAB_WITHIN names no function of the launcher's or comes without
LAB_PROFILE, LAB_PROFILE without QS_GIVEN's `--debug 3768`, or LAB_BACK without LAB_STEP.
"""

from __future__ import annotations

import base64
import gzip
import itertools
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
from agents_served import agents_served

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

# The kinds of the trace's ranges that are the shell's own script, as against the scene graph's
# frames, which the profiler times apart.
SCRIPTED = frozenset(("Binding", "HandlingSignal", "Javascript"))

# What an opening's line says of its frame's time from its sync until the shell heard it swapped,
# as launcher/Thread.js says it.
SWAPPED = re.compile(r"(\d+) before the shell heard it swapped")

# How many characters of a trace, packed, each of its lines LAB_TRACES asks for holds.
PACKED = 4000

# How long the profiler may take to write its trace once asked, in seconds.
FLUSHING = 60

# How many of the shell's lines LAB_HEARD asks for are said, at most.
HEARD = 400

# How many of the slowest searches' time before gathering is said apart.
SLOWEST = 6

# How many of a span's costliest places are said, unless LAB_COSTLIEST says otherwise.
COSTLIEST = int(os.environ.get("LAB_COSTLIEST", "25"))

# How long a window LAB_CHANGES retitles keeps each title, in seconds, longer than the pace a
# change waits on, so each is a refresh of its own.
RETITLED = 0.5

# What the shell says of the changes to windows as the launcher closes.
CHANGES = re.compile(r"launcher: \d+ changes? in windows .*")

# A window's shell, which waits for its go before retitling itself, each title holding the word.
RETITLING = """\
while [ ! -e {go} ]; do sleep 0.1; done
i=0
while [ $i -lt {count} ]; do
  i=$((i + 1))
  printf '\\033]2;{word} %s\\007' $i
  sleep {span}
done
sleep 3600
"""


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


def named(given: str, step: str, key: str) -> None:
    """Install each program given as `id=Name`, comma between, with two actions, as Chrome has.

    An entry that isn't an id and a name fails at once, rather than as a step that never comes,
    and so does an id the lab's own programs have, or a `step` into one of these by Return,
    which starts a program, closing the launcher, rather than going into its page. Joel's
    "\u2192 Google Chrome" took a median 80 ms over 10 steps since 2026-11-10, its showing 12
    to 24 of it, so its page is stepped into here under the profiler, as his is.
    """
    data = Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local/share")
    applications = data / "applications"
    applications.mkdir(parents=True, exist_ok=True)
    for one in filter(None, (each.strip() for each in given.split(","))):
        desktop, _, name = (part.strip() for part in one.partition("="))
        refused = (
            f"LAB_NAMED wants id=Name: {one!r}"
            if not re.fullmatch(r"[\w.-]+", desktop) or not name
            else f"LAB_PROGRAMS has {desktop}"
            if desktop.startswith("lab-program-")
            else f"{name} is stepped into by LAB_STEP_KEY=tab"
            if name == step and key != "tab"
            else None
        )
        if refused:
            raise AssertionError(refused)
        (applications / f"{desktop}.desktop").write_text(
            f"[Desktop Entry]\nType=Application\nName={name}\nExec=true\n"
            "Actions=new-window;new-private-window;\n\n"
            # Each its own command, as the launcher lists no action that only starts it.
            "[Desktop Action new-window]\nName=New Window\nExec=true --new-window\n\n"
            "[Desktop Action new-private-window]\nName=New Incognito Window\n"
            "Exec=true --incognito\n"
        )


def stepped_from_home(shell: Shell, name: str, key: str = "ret") -> tuple[str | None, str | None]:
    """Open home, find the place by its name, step in by `key`, and give the step's line.

    The step's line is a key's, typed as an arrow and the place's name, said only for a step
    Joel could feel (launcher/Trials.js's felt); one quicker gives none. Given LAB_BACK, it then
    steps back out with -, and gives that step's line second; else None. That step puts back
    the words typed to find the place, so it searches them as a key: Joel's out of Windows put
    none back, which LAB_CHORD steps as. Given LAB_STEP_BY=arrow, it reaches the place by arrows
    from home with nothing typed, as Joel reaches Agents, which home builds no rows for until a
    step or a letter wants them (launcher/Wanted.js).
    """
    shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    if os.environ.get("LAB_STEP_BY") == "arrow":
        shell.down_to(name)
        # Return comes once the arrow's frames are drawn, as Joel's hand comes after his ↓,
        # not in the frame the arrow moved.
        if not shell.held_still():
            refused = "the launcher never held still after the arrows"
            raise AssertionError(refused)
    else:
        shell.type_in(name.lower(), selecting=name)
    since = shell.mark()
    shell.step_in(key)
    step = f"\u2192 {name}"
    deadline = time.monotonic() + SAYING
    said = keys_said(shell, since)
    while step not in said and time.monotonic() < deadline:
        time.sleep(launcher_on_niri.POLL)
        said = keys_said(shell, since)
    out = stepped_out(shell) if os.environ.get("LAB_BACK") else None
    shell.close_launcher()
    shell.launcher_until(lambda shown: not shown["open"])
    return said.get(step), out


def chorded_out(shell: Shell, place: str) -> tuple[str | None, str | None]:
    """Open the place from shut as its chord does, step out to home, and give both lines.

    The first is the opening's own, as Joel's into Now by its chord spent a median 18 ms from
    the ask to its start where home's spent none (2026-11-21); the second is the step out's.
    Nothing is typed, so no words are put back and the step searches none, as Joel's out of
    Windows spent 20 and 19 ms of its 58 and 53 ms holding, before it searched (2026-11-17 and
    18).
    """
    shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
    since = shell.mark()
    shell.as_account(shell.ipc("collection", place))
    shell.launcher_until(lambda shown: shown["open"] and len(shown["held"]) > 0)
    shell.launcher_ready()
    opening = into_said(shell, since)
    out = stepped_out(shell)
    shell.close_launcher()
    shell.launcher_until(lambda shown: not shown["open"])
    return opening, out


def opened_home(shell: Shell, whole: str, profiling: Profiling | None) -> str | None:
    """Open home from shut by its toggle, as Super+Space does, and give the opening's line.

    Home is no place a chord steps into, so nothing is stepped out of. Kept, it opens from what
    was kept, as every opening but a shell's first does. Given LAB_WHOLE=first, the shell starts
    again first and home opens at once, before warming keeps it, so each is whole and the first
    since the shell started, as two of Joel's three whole openings were (2026-11-27).
    """
    if whole == "first":
        restarted(shell, profiling)
    else:
        shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
    since = shell.mark()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    opening = into_said(shell, since)
    shell.close_launcher()
    shell.launcher_until(lambda shown: not shown["open"])
    return opening


def restarted(shell: Shell, profiling: Profiling | None) -> None:
    """Start the shell again, and its profiler with it, so a trace holds its first opening.

    A restart drops the profiler's connection, so the trace so far is written first, and the
    shell starts waiting for a new profiler (`--waitfordebug`), which tries its port until it
    listens, so nothing the shell runs as it starts goes unrecorded.
    """
    if profiling is None:
        shell.restart()
        return
    profiling.written()
    given = os.environ.get("QS_GIVEN", "")
    os.environ["QS_GIVEN"] = f"{given} --waitfordebug"
    try:
        shell.restart(while_down=lambda: profiling.begin(fresh=True))
    finally:
        os.environ["QS_GIVEN"] = given


def into_said(shell: Shell, since: int) -> str | None:
    """Give the opening's line the shell has said since the mark, from its time on, or None.

    A kept opening says it only after its sized frames, which may come after the launcher is
    ready, so it's waited for as a step's is, and a line still being written is left out.
    """
    deadline = time.monotonic() + SAYING
    while True:
        with shell.log.open("rb") as log:
            log.seek(since)
            written = log.read()
        whole = written[: written.rfind(b"\n") + 1].decode(errors="replace")
        said = [line for line in whole.splitlines() if "launcher: opening took" in line]
        if said:
            return said[0].split("launcher: opening ", 1)[1]
        if time.monotonic() >= deadline:
            return None
        time.sleep(launcher_on_niri.POLL)


def stepped_out(shell: Shell) -> str | None:
    """Press - from normal in the place held, and give the step out's line, typed as ← and its name.

    None when the step was quicker than Joel could feel, as a step in's is.
    """
    shell.normal()
    depth = len(shell.showing()["held"])
    if depth == 0:
        refused = "there was no place held to step out of"
        raise AssertionError(refused)
    since = shell.mark()
    shell.send_chars("-")
    shell.launcher_until(lambda shown: len(shown["held"]) < depth)
    deadline = time.monotonic() + SAYING
    while time.monotonic() < deadline:
        out = [typed for typed in keys_said(shell, since) if typed.startswith("\u2190 ")]
        if out:
            return f"{out[0]}: {keys_said(shell, since)[out[0]]}"
        time.sleep(launcher_on_niri.POLL)
    return None


def changed_under_words(shell: Shell, word: str, count: int, folder: Path) -> list[str]:
    """Open home on the word typed while a window holding it is retitled, and give what's said.

    The window opens before the launcher, as opening one takes the keys, and retitles itself
    only once the word is shown, so every change is a refresh of what's typed.
    """
    go = folder / f"go-{word}"
    go.unlink(missing_ok=True)
    script = folder / f"retitling-{word}.sh"
    script.write_text(RETITLING.format(go=go, count=count, word=word, span=RETITLED))
    window = shell.opened(f"{word} 0", running=f"sh {script}")
    shell.launcher_until(lambda shown: not shown["open"] and shown.get("kept"))
    since = shell.mark()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.send_chars(word)
    shown_typed(shell, word)
    go.touch()
    deadline = time.monotonic() + count * RETITLED + SAYING
    while time.monotonic() < deadline:
        if any(one["title"] == f"{word} {count}" for one in shell.windows()):
            break
        time.sleep(launcher_on_niri.POLL)
    # The last title's refresh is drawn before the launcher closes.
    time.sleep(RETITLED)
    shell.close_launcher()
    shell.launcher_until(lambda shown: not shown["open"])
    shell.close(window)
    deadline = time.monotonic() + SAYING
    said: list[str] = []
    while not said and time.monotonic() < deadline:
        with shell.log.open("rb") as log:
            log.seek(since)
            written = log.read().decode(errors="replace")
        said = [found.group(0) for found in CHANGES.finditer(written)]
        time.sleep(launcher_on_niri.POLL)
    return said


def changed_instead(
    shell: Shell,
    word: str,
    rounds: int,
    folder: Path,
    capsys: pytest.CaptureFixture[str],
) -> int:
    """Given LAB_CHANGES, print each round's changes under the word, and give the rounds left.

    Its rounds are the run's, so none are left for typing; without it, all of them are.
    """
    changes = int(os.environ.get("LAB_CHANGES", "0"))
    if changes == 0:
        return rounds
    for round_ in range(rounds):
        for line in changed_under_words(shell, word, changes, folder) or ["none"]:
            with capsys.disabled():
                print(f"lab-changes {round_} {word!r}: {line}", flush=True)
    return 0


def tallied(
    round_: int,
    word: str,
    said: dict[str, str],
    parts: dict[str, dict[str, list[int]]],
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Print each of a word's keys' lines as typed this round, and add its parts to the tally."""
    for end in range(1, len(word) + 1):
        typed = word[:end]
        line = said.get(typed)
        with capsys.disabled():
            print(f"lab-key {round_} {typed!r}: {line or 'under the felt'}", flush=True)
        took = part(f"took {line}", "took") if line else None
        parts[typed]["took"].append(took or 0)
        for name in PARTS:
            parts[typed][name].append((part(line, name) or 0) if line else 0)


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
    parts = Parts()
    # The functions whose calls are spans of their own, `within-<name>`, each call holding itself
    # and what ran inside it, so the function's own place in its span says what its calls took.
    asked = set(within_asked())
    within: dict[str, str] = {}
    kinds: dict[str, str] = {}
    for event in root.iter("event"):
        where = Path(event.findtext("filename") or "").name
        kind = event.findtext("type") or ""
        details = (event.findtext("details") or "").replace("\n", " ")[:60]
        places[event.get("index", "")] = f"{kind} {where}:{event.findtext('line')} {details}"
        kinds[event.get("index", "")] = kind
        if where == "Launcher.qml" and details in parts.functions:
            parts.functions[details].add(event.get("index", ""))
        if where == "Launcher.qml" and kind == "Javascript" and details in asked:
            within[event.get("index", "")] = details
    spans: dict[str, dict[str, Spent]] = defaultdict(lambda: defaultdict(Spent))
    # The ranges still open where the next starts: when each ends, its places' tallies, and the
    # functions asked for that it runs within.
    held: list[tuple[int, list[Spent], frozenset[str]]] = []
    # A call starting where its caller does is read after it, by its shorter duration.
    for each in sorted(
        root.iter("range"),
        key=lambda one: (int(one.get("startTime", "0")), -int(one.get("duration", "0"))),
    ):
        start = int(each.get("startTime", "0"))
        index = each.get("eventIndex", "")
        if kinds.get(index) not in SCRIPTED:
            continue
        ends = start + int(each.get("duration", "0"))
        took = int(each.get("duration", "0")) / 1e6
        while held and held[-1][0] <= start:
            held.pop()
        for parent in held[-1][1] if held else []:
            parent.own -= took
        # A named call is in its own span as well as what it ran; one inside another of the same
        # name adds its whole again at that place, as no function of the launcher's recurses.
        inside = (held[-1][2] if held else frozenset()) | (
            {within[index]} if index in within else set()
        )
        names = parts.at(index, start, ends) + [f"within-{name}" for name in sorted(inside)]
        tallies = [spans[name][places.get(index, "?")] for name in names]
        for tally in tallies:
            tally.count += 1
            tally.own += took
            tally.whole += took
        held.append((ends, tallies, inside))
    return spans


class Opening:
    """One toggled() call that opened the launcher, as the trace holds it, in its clock's ns."""

    def __init__(self, start: int, ends: int) -> None:
        """Begin with the call's span, its way of opening and its work's end still unknown."""
        self.start = start
        self.ends = ends
        self.how = ""
        # When openWorkDone returned, its stamp ending the opening's own work, or the call's end.
        self.done = ends
        self.worked = False
        # When the shell heard its frame swapped, its handler's start, and the opening's line.
        self.heard = -1
        self.line = ""


# A span a place is tallied in: its name, from and to, and where its working ends, if it's split.
Window = tuple[str, int, int, int | None]


def portions(start: int, ends: int, done: int | None) -> tuple[int, int]:
    """Give how much of a span came before `done` and how much after, all before with none."""
    if done is None:
        return ends - start, 0
    return max(0, min(ends, done) - start), max(0, ends - max(start, done))


class Openings:
    """Each opening toggled() made in a trace, read range by range in order of their start.

    Whole runs wakeOpen within the call and kept runs beginOpening; a call running neither shut
    the launcher. The frame is heard where the handler holding openDrawn (whole) or openShown
    (kept) begins, which runs it as the frame is swapped, and an opening's line is the next the
    shell says.
    """

    def __init__(self, root: ET.Element) -> None:
        """Know the launcher's functions, the signal handlers and the openings' lines by event."""
        self.named: dict[str, str] = {}
        self.handlers: set[str] = set()
        self.lines: dict[str, str] = {}
        for event in root.iter("event"):
            index = event.get("index", "")
            kind = event.findtext("type") or ""
            text = event.findtext("details") or ""
            if (
                Path(event.findtext("filename") or "").name == "Launcher.qml"
                and kind == "Javascript"
            ):
                self.named[index] = text
            if kind == "HandlingSignal":
                self.handlers.add(index)
            if "opening took" in text:
                self.lines[index] = text
        self.calls: list[Opening] = []
        self.handling: list[tuple[int, int]] = []

    def read(self, start: int, ends: int, index: str) -> None:
        """Read one range, starting and ending so, of this event."""
        name = self.named.get(index, "")
        if name == "toggled":
            self.calls.append(Opening(start, ends))
        elif index in self.handlers:
            self.handling.append((start, ends))
        elif index in self.lines:
            unsaid = [one for one in self.calls if one.how and not one.line]
            if unsaid:
                unsaid[0].line = self.lines[index]
        call = self.calls[-1] if self.calls else None
        if call is None or not call.start <= start < call.ends:
            self.heard(name, start)
        elif name == "wakeOpen":
            call.how = "whole"
        elif name == "beginOpening":
            call.how = "kept"
        elif name == "openWorkDone" and not call.worked:
            call.done, call.worked = ends, True

    def heard(self, name: str, start: int) -> None:
        """Mark the last opening still unheard as heard, where a call of `name` says it is."""
        waiting = [one for one in self.calls if one.how and one.heard < 0]
        holding = [each for each in self.handling if each[0] <= start < each[1]]
        if name not in {"openDrawn", "openShown"} or not waiting or not holding:
            return
        if (name == "openDrawn") == waiting[-1].how.startswith("whole"):
            waiting[-1].heard = holding[-1][0]


def openings(root: ET.Element, *, fresh: bool) -> list[Opening]:
    """Give each opening toggled() made in the trace, how it opened and when it was heard.

    The first in a trace whose profiler began with the shell is the first since the shell
    started, `whole-first`.
    """
    read = Openings(root)
    for start, ends, index in ranged(root):
        read.read(start, ends, index)
    opened = [one for one in read.calls if one.how]
    if fresh and opened and opened[0].how == "whole":
        opened[0].how = "whole-first"
    return opened


def ranged(root: ET.Element) -> list[tuple[int, int, str]]:
    """Give each range's start, end and event, a call after its caller where both start at once."""
    every = [
        (int(one.get("startTime", "0")), int(one.get("duration", "0")), one.get("eventIndex", ""))
        for one in root.iter("range")
    ]
    return [
        (start, start + took, index)
        for start, took, index in sorted(every, key=lambda one: (one[0], -one[1]))
    ]


def settled(trace: Path, *, fresh: bool) -> tuple[dict[str, dict[str, Spent]], list[str]]:
    """Split each opening's toggled() at its own work's end, and say what its frame waited on.

    Root's open handler (whole) or beginOpening (kept) ends the opening's own work by calling
    openWorkDone, and what toggled() runs after that is its settling: the other handlers and
    bindings hearing `open` or `opening` turn. So each place a toggled() call runs is spanned
    `<how>-working` before that call and `<how>-settling` after it, its own time split at it
    where it straddles it, by how it opened (Opening). Each place the shell's thread ran from the
    opening's frame's sync until it heard the frame swapped, as long as its line says that was,
    is spanned `<how>-unheard`. Each opening is said too, its spans as the trace times them.
    """
    root = ET.parse(trace).getroot()  # noqa: S314, the trace this run wrote
    places: dict[str, str] = {}
    kinds: dict[str, str] = {}
    for event in root.iter("event"):
        where = Path(event.findtext("filename") or "").name
        kind = event.findtext("type") or ""
        details = (event.findtext("details") or "").replace("\n", " ")[:60]
        places[event.get("index", "")] = f"{kind} {where}:{event.findtext('line')} {details}"
        kinds[event.get("index", "")] = kind
    windows, said = windowed(openings(root, fresh=fresh))
    return settled_spans(root, places, kinds, windows), said


def windowed(opened: list[Opening]) -> tuple[list[Window], list[str]]:
    """Give each opening's windows, its toggled() call and its frame's wait, and its line said."""
    windows: list[Window] = []
    said: list[str] = []
    for one in opened:
        windows.append((one.how, one.start, one.ends, one.done))
        swapped = SWAPPED.search(one.line)
        unheard = int(swapped.group(1)) * 1_000_000 if swapped and one.heard >= 0 else -1
        if unheard >= 0:
            windows.append((f"{one.how}-unheard", one.heard - unheard, one.heard, None))
        said.append(
            f"{one.how}: toggled {(one.ends - one.start) / 1e6:.2f} ms, working"
            f" {(one.done - one.start) / 1e6:.2f}, settling {(one.ends - one.done) / 1e6:.2f},"
            f" heard {(one.heard - one.ends) / 1e6 if one.heard >= 0 else -1:.2f} after,"
            f" its line {one.line.split('opening ', 1)[-1][:400]!r}"
        )
    return windows, said


def tallied_in(
    spans: dict[str, dict[str, Spent]], entry: list, place: str, window: Window | None
) -> None:
    """Add a range, held as `entry`, to its window's spans, its own time split where it's split."""
    if window is None:
        return
    _, _, before, after, start, took = entry
    name, _, _, done = window
    first = name if done is None else f"{name}-{'working' if start < done else 'settling'}"
    spent = spans[first][place]
    spent.count += 1
    spent.whole += took / 1e6
    if done is None:
        spent.own += before / 1e6
        return
    spans[f"{name}-working"][place].own += before / 1e6
    spans[f"{name}-settling"][place].own += after / 1e6


def settled_spans(
    root: ET.Element, places: dict[str, str], kinds: dict[str, str], windows: list[Window]
) -> dict[str, dict[str, Spent]]:
    """Give each window's places, by their own time and their whole, as settled says."""
    spans: dict[str, dict[str, Spent]] = defaultdict(lambda: defaultdict(Spent))
    # The ranges still open where the next starts, each with its place, its window and its own
    # time on either side of where the window's working ends, what it called being taken out.
    held: list[tuple[str, Window | None, list]] = []
    for start, ends, index in ranged(root):
        if kinds.get(index) not in SCRIPTED:
            continue
        while held and held[-1][2][0] <= start:
            tallied_in(spans, *popped(held))
        window = next((one for one in windows if one[1] <= start < one[2]), None)
        before, after = portions(start, ends, window[3] if window else None)
        if held and window is not None and held[-1][1] is window:
            held[-1][2][2] -= before
            held[-1][2][3] -= after
        held.append(
            (places.get(index, "?"), window, [ends, index, before, after, start, ends - start])
        )
    while held:
        tallied_in(spans, *popped(held))
    return spans


def popped(held: list[tuple[str, Window | None, list]]) -> tuple[list, str, Window | None]:
    """Take the last range held, as tallied_in takes it."""
    place, window, entry = held.pop()
    return entry, place, window


class Parts:
    """Which part of a launcher's search a range of the trace falls in, as it's read in order.

    Each call of its search is a span, `search`, and so is each of its parts, `search-<part>`:
    `before` it gathers its lists, `lists` while gathering them, `rows` until it composes its
    answers, and `answers` from then on, as the key's line splits it. A step into a place
    gathers nothing: its rows are the place's, so it's `step` from when it lives them on, which
    is its showing as the step's line says it.
    """

    def __init__(self) -> None:
        """Begin outside any search, knowing none of its functions yet."""
        self.functions: dict[str, set[str]] = {
            "search": set(),
            "gathering": set(),
            "compose": set(),
            "live": set(),
        }
        self.ends = 0
        self.part = ""
        self.gathered = 0
        self.count = 0

    def at(self, index: str, start: int, ends: int) -> list[str]:
        """Give the spans a range of this event, starting and ending so, falls in."""
        if start >= self.ends:
            if index in self.functions["search"]:
                self.ends, self.part = ends, "before"
                self.count += 1
            return []
        if index in self.functions["gathering"] and self.part == "before":
            self.part, self.gathered = "lists", ends
        elif self.part == "lists" and start >= self.gathered:
            self.part = "rows"
        if index in self.functions["compose"] and self.part in {"lists", "rows"}:
            self.part = "answers"
        if index in self.functions["live"] and self.part == "before":
            self.part = "step"
        # Each search's time before it gathers is a span of its own too, `before#<n>`, so the
        # slowest are named apart from the rest.
        alone = [f"before#{self.count}"] if self.part == "before" else []
        return ["search", f"search-{self.part}", *alone]


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


def merged(into: dict[str, dict[str, Spent]], spans: dict[str, dict[str, Spent]]) -> None:
    """Add each place's tally in `spans` to its own in `into`, span by span."""
    for name, places in spans.items():
        for place, spent in places.items():
            tally = into[name][place]
            tally.count += spent.count
            tally.own += spent.own
            tally.whole += spent.whole


def say_packed(trace: Path) -> None:
    """Print the trace packed, a line of PACKED characters at a time, so it outlives the run."""
    packed = base64.b64encode(gzip.compress(trace.read_bytes())).decode()
    for at in range(0, len(packed), PACKED):
        print(f"lab-trace {trace.name} {packed[at : at + PACKED]}", flush=True)


def gathered(
    traces: list[tuple[Path, bool]],
) -> tuple[dict[str, dict[str, Spent]], list[str]]:
    """Give every trace's spans added together, and each of their openings said."""
    spans: dict[str, dict[str, Spent]] = defaultdict(lambda: defaultdict(Spent))
    opened: list[str] = []
    for trace, fresh in traces:
        merged(spans, profiled(trace))
        split, lines = settled(trace, fresh=fresh)
        merged(spans, split)
        opened.extend(f"{trace.name} {line}" for line in lines)
    return spans, opened


def say_held(traces: list[tuple[Path, bool]], opened: list[str]) -> None:
    """Print what each trace holds, the trace itself given LAB_TRACES, and each opening."""
    for trace, _ in traces:
        for line in census(trace):
            print(f"lab-profile held {trace.name} {line}", flush=True)
        if os.environ.get("LAB_TRACES"):
            say_packed(trace)
    for line in opened:
        print(f"lab-opening {line}", flush=True)


def say_profiled(
    traces: list[tuple[Path, bool]], said: list[str], capsys: pytest.CaptureFixture[str]
) -> None:
    """Print each span's costliest places by their own time, or what the traces held instead.

    Each trace is a life of the shell's, and their spans are added together. With no span, what
    the profiler and the shell said of the debugger is printed too, as what's said leaves the
    run with it. Given LAB_TRACES, each trace is printed too, packed.
    """
    written = [(trace, fresh) for trace, fresh in traces if trace.exists()]
    spans, opened = gathered(written)
    with capsys.disabled():
        for name in [name for name in within_asked() if f"within-{name}" not in spans]:
            print(f"lab-profile within-{name}: never called while profiled", flush=True)
        say_held(written, opened)
        if not spans:
            for line in said:
                print(f"lab-profile said: {line}", flush=True)
            # The traces this run wrote, as the others are.
            ranges = sum(
                len(list(ET.parse(trace).getroot().iter("range")))  # noqa: S314
                for trace, _ in written
            )
            print(f"lab-profile: no search profiled, of {ranges} ranges", flush=True)
        each = sorted(
            (span for span in spans if span.startswith("before#")),
            key=lambda span: sum(spent.own for spent in spans[span].values()),
            reverse=True,
        )
        for span in [span for span in spans if "#" not in span] + each[:SLOWEST]:
            places = spans[span]
            ordered = sorted(places.items(), key=lambda one: one[1].own, reverse=True)
            for place, spent in ordered[: COSTLIEST if "#" not in span else COSTLIEST // 2]:
                print(
                    f"lab-profile {span}: own {spent.own:.2f} ms, whole {spent.whole:.2f}"
                    f" over {spent.count}, {place}",
                    flush=True,
                )
            # A step's showing and each named function's span are said by whole time too, so
            # what holds the rest is named, as keep holds laying the rows out and each row's
            # acts made within it.
            if span == "search-step" or span.startswith("within-"):
                wholes = sorted(places.items(), key=lambda one: one[1].whole, reverse=True)
                for place, spent in wholes[:COSTLIEST]:
                    print(
                        f"lab-profile {span} whole: {spent.whole:.2f} ms over {spent.count},"
                        f" own {spent.own:.2f}, {place}",
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


def within_asked() -> list[str]:
    """Give the launcher's functions LAB_WITHIN names, by spaces or commas."""
    return [name for name in re.split(r"[\s,]+", os.environ.get("LAB_WITHIN", "")) if name]


def debugged(given: str) -> bool:
    """Say whether QS_GIVEN opens the shell to the profiler's port, read as the shell gets it.

    launcher_on_niri gives the shell QS_GIVEN split on whitespace, so `--debug 37680` opens
    another port, and two spaces between the words still open this one. qs takes the port as
    `--debug 3768` or `--debug=3768`, and refuses to start when given `--debug` twice.
    """
    words = given.split()
    ports = [
        port if word == "--debug" else word.removeprefix("--debug=")
        for word, port in itertools.pairwise([*words, ""])
        if word == "--debug" or word.startswith("--debug=")
    ]
    return ports == [PORT]


def lab_asks_known(launcher: Path) -> None:
    """Fail at once on a LAB_WITHIN name that's no function of the launcher's, or no profile.

    Such a name matches nothing in the trace, so its span would be left out without a word, at
    the end of a run of minutes, and so would every name given without LAB_PROFILE. A shell not
    given the profiler's port (QS_GIVEN's `--debug`) never lets it connect, and two runs into
    Now profiled nothing over 4 and 9 minutes, failing at the end (2026-11-22).
    """
    asked = within_asked()
    source = launcher.read_text() if asked else ""
    unknown = [
        name
        for name in asked
        if not re.search(rf"^\s*function {re.escape(name)}\(", source, re.MULTILINE)
    ]
    refused = (
        f"LAB_WITHIN names no function of {launcher.name}'s: {unknown}"
        if unknown
        else "LAB_WITHIN profiles only with LAB_PROFILE"
        if asked and not os.environ.get("LAB_PROFILE")
        else f'LAB_PROFILE profiles only a shell given QS_GIVEN = "--debug {PORT}"'
        if os.environ.get("LAB_PROFILE") and not debugged(os.environ.get("QS_GIVEN", ""))
        else "LAB_WHOLE is first, or not given"
        if os.environ.get("LAB_WHOLE", "") not in {"", "first"}
        else None
    )
    if refused:
        raise AssertionError(refused)


def say_heard(log: Path, capsys: pytest.CaptureFixture[str]) -> None:
    """Print what else the shell said that LAB_HEARD asks for, where it asks for any.

    The collector's own category says each run it makes, for one, when the shell is given
    `--log-rules qt.qml.gc.statistics=true`.
    """
    if asked := os.environ.get("LAB_HEARD"):
        logged = log.read_text(errors="replace").splitlines()
        with capsys.disabled():
            for line in [line for line in logged if re.search(asked, line)][:HEARD]:
                print(f"lab-heard {line}", flush=True)


class Profiling:
    """Qt's profiler on the shell, a trace for each of its lives, as a restart ends one."""

    def __init__(self, folder: Path) -> None:
        """Begin with no trace, each to be written in `folder`, all saying into one log."""
        self.folder = folder
        self.log = folder / "profiler.log"
        # Each trace, and whether its profiler began before the shell did, so its first
        # opening is the first since the shell started.
        self.traces: list[tuple[Path, bool]] = []
        self.ended: list[int] = []
        self.process: subprocess.Popen[bytes] | None = None

    def begin(self, *, fresh: bool) -> None:
        """Start a profiler writing a trace of its own."""
        trace = self.folder / f"keys-{len(self.traces)}.qtd"
        self.traces.append((trace, fresh))
        self.process = profiler_started(trace, self.log)

    def written(self) -> None:
        """Have the profiler write its trace and quit, while the shell still runs."""
        if self.process is None:
            return
        flushed(self.process, self.traces[-1][0])
        self.ended.append(self.process.wait(timeout=60))
        self.process = None


def profiler_started(trace: Path, profiling: Path) -> subprocess.Popen[bytes]:
    """Start qmlprofiler on the shell's debug port, writing its trace to `trace`.

    It's asked for the scene graph's frames too, so an opening's frame is found by its sync.
    """
    return subprocess.Popen(
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
            "binding,javascript,handlingsignal,debugmessages,scenegraph",
        ],
        stdin=subprocess.PIPE,
        stdout=profiling.open("ab"),
        stderr=subprocess.STDOUT,
    )


def chorded(shell: Shell, chord: str, profiling: Profiling | None) -> tuple[str | None, str | None]:
    """Open LAB_CHORD's place and step out of it, or home by its toggle, given both lines.

    Home steps out of nothing, so its second is None; LAB_WHOLE says how it opens (opened_home).
    """
    if chord != "home":
        return chorded_out(shell, chord)
    return opened_home(shell, os.environ.get("LAB_WHOLE", ""), profiling), None


def warmed(shell: Shell, step: str, step_key: str, profiling: Profiling | None) -> None:
    """Step into `step` and out of LAB_CHORD's place once untimed, as the words are typed once."""
    if step:
        stepped_from_home(shell, step, step_key)
    if chord := os.environ.get("LAB_CHORD"):
        chorded(shell, chord, profiling)


def stepped_round(
    shell: Shell,
    round_: int,
    step: tuple[str, str],
    profiling: Profiling | None,
    capsys: pytest.CaptureFixture[str],
) -> None:
    """Step into `step` by its key and back out, then out of LAB_CHORD's place, printing each."""
    if step[0]:
        line, out = stepped_from_home(shell, *step)
        with capsys.disabled():
            print(f"lab-step {round_} {step[0]}: {line or 'under the felt'}", flush=True)
            if os.environ.get("LAB_BACK"):
                print(f"lab-back {round_}: {out or 'under the felt'}", flush=True)
    if chord := os.environ.get("LAB_CHORD"):
        opening, out = chorded(shell, chord, profiling)
        with capsys.disabled():
            print(f"lab-into {round_}: {opening or 'never said'}", flush=True)
            if chord != "home":
                print(f"lab-chord {round_}: {out or 'under the felt'}", flush=True)


def test_each_home_key_is_timed(capsys: pytest.CaptureFixture[str], tmp_path: Path) -> None:
    """Type each word from home, round after round, and print each key's line and medians.

    They're printed past pytest's capture, so the build's log holds them when it passes.
    """
    words = os.environ.get("LAB_WORDS", " ".join(WORDS)).split()
    rounds = int(os.environ.get("LAB_ROUNDS", str(ROUNDS)))
    profiling = Profiling(tmp_path) if os.environ.get("LAB_PROFILE") else None
    parts: dict[str, dict[str, list[int]]] = defaultdict(lambda: defaultdict(list))
    step = os.environ.get("LAB_STEP", "")
    if os.environ.get("LAB_BACK") and not step:
        refused = "LAB_BACK steps out of LAB_STEP's place, and none was given"
        raise AssertionError(refused)
    step_key = os.environ.get("LAB_STEP_KEY", "ret")
    launcher_on_niri.installed(int(os.environ.get("LAB_PROGRAMS", "0")))
    named(os.environ.get("LAB_NAMED", ""), step, step_key)
    lab_asks_known(Path(os.environ["SHELL_FOLDER"]) / "Launcher.qml")
    agents = agents_served(int(os.environ.get("LAB_AGENTS", "0")))
    with agents as moved, launcher_on_niri.running() as shell:
        shell.unshared()
        shell.launcher_until(lambda shown: shown.get("kept"))
        # Windows to list are opened before the profiler listens, so it hears only the keys.
        for nth in range(int(os.environ.get("LAB_WINDOWS", "0"))):
            shell.opened(f"Lab {nth}", app="foot")
        if profiling is not None:
            profiling.begin(fresh=False)
        # Typed once untimed first, as the openings' lab opens once, for what a fresh shell
        # pays only once: its first search compiles what Joel's has run since he logged in.
        typed_from_home(shell, words[0])
        rounds = changed_instead(shell, words[0], rounds, tmp_path, capsys)
        warmed(shell, step, step_key, profiling)
        for round_ in range(rounds):
            for word in words[round_ % len(words) :] + words[: round_ % len(words)]:
                tallied(round_, word, typed_from_home(shell, word), parts, capsys)
            # A session moves while home's shut, as Joel's do between his looks, so Agents
            # has something new to say once the step wants its rows; with no LAB_AGENTS, none.
            moved()
            stepped_round(shell, round_, (step, step_key), profiling, capsys)
        if profiling is not None:
            profiling.written()
    heard: list[str] = []
    if profiling is not None:
        heard.extend(f"the profiler ended {ended}" for ended in profiling.ended)
        heard.extend(profiling.log.read_text(errors="replace").splitlines())
        logged = shell.log.read_text(errors="replace").splitlines()
        heard.extend(line for line in logged if "ebug" in line or "qmltooling" in line)
    for typed, each in parts.items():
        medians = ", ".join(f"{name} {statistics.median(spans):g}" for name, spans in each.items())
        with capsys.disabled():
            print(f"lab-keys {typed!r}: {len(each['took'])} typed, medians {medians}", flush=True)
    say_heard(shell.log, capsys)
    if profiling is not None:
        say_profiled(profiling.traces, heard, capsys)
