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
and steps out of it to home with nothing typed, as Joel's Super+Tab then - does.
Given LAB_WITHIN too, names of the launcher's functions, it says what their calls took and what
ran inside them. Given LAB_CHANGES, it
leaves the first word typed at home each round while a window holding it in its title takes that
many new titles, and prints what the shell says of those changes, as Joel's daemons change rows
under his typed words at 17 to 35 ms in the shell (2026-11-15). It fails when a key is never
shown, and at once when LAB_WITHIN names no function of the launcher's or comes without
LAB_PROFILE, LAB_PROFILE without QS_GIVEN's `--debug 3768`, or LAB_BACK without LAB_STEP.
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
    opening = opening_said(shell, since)
    out = stepped_out(shell)
    shell.close_launcher()
    shell.launcher_until(lambda shown: not shown["open"])
    return opening, out


def opening_said(shell: Shell, since: int) -> str | None:
    """Give the opening's line the shell has said since the mark, from its time on, or None."""
    with shell.log.open("rb") as log:
        log.seek(since)
        written = log.read().decode(errors="replace")
    said = [line for line in written.splitlines() if "launcher: opening took" in line]
    return said[0].split("launcher: opening ", 1)[1] if said else None


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
    for event in root.iter("event"):
        where = Path(event.findtext("filename") or "").name
        kind = event.findtext("type") or ""
        details = (event.findtext("details") or "").replace("\n", " ")[:60]
        places[event.get("index", "")] = f"{kind} {where}:{event.findtext('line')} {details}"
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


def say_profiled(trace: Path, said: list[str], capsys: pytest.CaptureFixture[str]) -> None:
    """Print each span's costliest places by their own time, or what the trace held instead.

    With no span, what the profiler and the shell said of the debugger is printed too, as what's
    said leaves the run with it.
    """
    spans = profiled(trace) if trace.exists() else {}
    with capsys.disabled():
        for name in [name for name in within_asked() if f"within-{name}" not in spans]:
            print(f"lab-profile within-{name}: never called while profiled", flush=True)
        for line in census(trace) if trace.exists() else []:
            print(f"lab-profile held {line}", flush=True)
        if not spans:
            for line in said:
                print(f"lab-profile said: {line}", flush=True)
            # The trace this run wrote, as the others are.
            whole = ET.parse(trace).getroot() if trace.exists() else None  # noqa: S314
            ranges = len(list(whole.iter("range"))) if whole is not None else 0
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


def within_known(launcher: Path) -> None:
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
        if os.environ.get("LAB_PROFILE") and f"--debug {PORT}" not in os.environ.get("QS_GIVEN", "")
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


def profiler_started(trace: Path, profiling: Path) -> subprocess.Popen[bytes]:
    """Start qmlprofiler on the shell's debug port, writing its trace to `trace`."""
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
            "binding,javascript,handlingsignal,debugmessages",
        ],
        stdin=subprocess.PIPE,
        stdout=profiling.open("wb"),
        stderr=subprocess.STDOUT,
    )


def warmed(shell: Shell, step: str, step_key: str) -> None:
    """Step into `step` and out of LAB_CHORD's place once untimed, as the words are typed once."""
    if step:
        stepped_from_home(shell, step, step_key)
    if chord := os.environ.get("LAB_CHORD"):
        chorded_out(shell, chord)


def stepped_round(
    shell: Shell, round_: int, step: tuple[str, str], capsys: pytest.CaptureFixture[str]
) -> None:
    """Step into `step` by its key and back out, then out of LAB_CHORD's place, printing each."""
    if step[0]:
        line, out = stepped_from_home(shell, *step)
        with capsys.disabled():
            print(f"lab-step {round_} {step[0]}: {line or 'under the felt'}", flush=True)
            if os.environ.get("LAB_BACK"):
                print(f"lab-back {round_}: {out or 'under the felt'}", flush=True)
    if chord := os.environ.get("LAB_CHORD"):
        opening, out = chorded_out(shell, chord)
        with capsys.disabled():
            print(f"lab-into {round_}: {opening or 'never said'}", flush=True)
            print(f"lab-chord {round_}: {out or 'under the felt'}", flush=True)


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
    step = os.environ.get("LAB_STEP", "")
    if os.environ.get("LAB_BACK") and not step:
        refused = "LAB_BACK steps out of LAB_STEP's place, and none was given"
        raise AssertionError(refused)
    step_key = os.environ.get("LAB_STEP_KEY", "ret")
    launcher_on_niri.installed(int(os.environ.get("LAB_PROGRAMS", "0")))
    named(os.environ.get("LAB_NAMED", ""), step, step_key)
    within_known(Path(os.environ["SHELL_FOLDER"]) / "Launcher.qml")
    agents = agents_served(int(os.environ.get("LAB_AGENTS", "0")))
    with agents as moved, launcher_on_niri.running() as shell:
        shell.unshared()
        shell.launcher_until(lambda shown: shown.get("kept"))
        # Windows to list are opened before the profiler listens, so it hears only the keys.
        for nth in range(int(os.environ.get("LAB_WINDOWS", "0"))):
            shell.opened(f"Lab {nth}", app="foot")
        if os.environ.get("LAB_PROFILE"):
            profiler = profiler_started(trace, profiling)
        # Typed once untimed first, as the openings' lab opens once, for what a fresh shell
        # pays only once: its first search compiles what Joel's has run since he logged in.
        typed_from_home(shell, words[0])
        rounds = changed_instead(shell, words[0], rounds, tmp_path, capsys)
        warmed(shell, step, step_key)
        for round_ in range(rounds):
            for word in words[round_ % len(words) :] + words[: round_ % len(words)]:
                tallied(round_, word, typed_from_home(shell, word), parts, capsys)
            # A session moves while home's shut, as Joel's do between his looks, so Agents
            # has something new to say once the step wants its rows; with no LAB_AGENTS, none.
            moved()
            stepped_round(shell, round_, (step, step_key), capsys)
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
    say_heard(shell.log, capsys)
    if profiler is not None:
        say_profiled(trace, heard, capsys)
