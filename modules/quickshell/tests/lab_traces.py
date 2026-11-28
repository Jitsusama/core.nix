"""What the keys lab's profiled traces say the launcher spent, place by place, for its LAB_PROFILE.

Qt's profiler writes a trace of each life of the shell's (launcher-keys-lab-on-niri.py runs it);
this reads them: what each binding, handler and function took inside each part of a search, and
each opening's toggled() call split into its own work and its settling. The lab says what's read.
"""

from __future__ import annotations

import re
import xml.etree.ElementTree as ET
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

# The kinds of the trace's ranges that are the shell's own script, as against the scene graph's
# frames, which the profiler times apart.
SCRIPTED = frozenset(("Binding", "HandlingSignal", "Javascript"))

# What an opening's line says of its frame's time from its sync until the shell heard it swapped,
# as launcher/Thread.js says it.
SWAPPED = re.compile(r"(\d+) before the shell heard it swapped")


class Spent:
    """What one place took inside a span: how often it ran, its own time and its whole."""

    def __init__(self) -> None:
        """Begin with nothing spent."""
        self.count = 0
        self.own = 0.0
        self.whole = 0.0


Spans = dict[str, dict[str, Spent]]


def spans_made() -> Spans:
    """Give spans to tally into, each place in each span beginning with nothing spent."""
    return defaultdict(lambda: defaultdict(Spent))


def profiled(trace: Path, asked: set[str]) -> Spans:
    """Give what each place took inside each span of the profiler's trace, by the span's name.

    A place is a binding, handler or function by its kind, file and line. Its own time leaves out
    what it called that the profiler recorded too, so a handler holding the whole search is not
    named for it; its whole is all of it. The launcher's functions `asked` for are spans of
    their own, `within-<name>`, each call holding itself and what ran inside it, so the
    function's own place in its span says what its calls took.
    """
    root = ET.parse(trace).getroot()  # noqa: S314, the trace this run wrote
    places: dict[str, str] = {}
    parts = Parts()
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
    spans = spans_made()
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


@dataclass
class Held:
    """A range still open where the next starts, as settled_spans reads them in order.

    Its own time either side of where its window's working ends has what it called taken out as
    each call is read, so what's left is its own.
    """

    place: str
    window: Window | None
    start: int
    ends: int
    before: int
    after: int


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


def settled(trace: Path, *, fresh: bool) -> tuple[Spans, list[str]]:
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


def tallied_in(spans: Spans, one: Held) -> None:
    """Add a range read to its window's spans, its own time split where its window is split."""
    if one.window is None:
        return
    name, _, _, done = one.window
    first = name if done is None else f"{name}-{'working' if one.start < done else 'settling'}"
    spent = spans[first][one.place]
    spent.count += 1
    spent.whole += (one.ends - one.start) / 1e6
    if done is None:
        spent.own += one.before / 1e6
        return
    spans[f"{name}-working"][one.place].own += one.before / 1e6
    spans[f"{name}-settling"][one.place].own += one.after / 1e6


def settled_spans(
    root: ET.Element, places: dict[str, str], kinds: dict[str, str], windows: list[Window]
) -> Spans:
    """Give each window's places, by their own time and their whole, as settled says."""
    spans = spans_made()
    held: list[Held] = []
    for start, ends, index in ranged(root):
        if kinds.get(index) not in SCRIPTED:
            continue
        while held and held[-1].ends <= start:
            tallied_in(spans, held.pop())
        window = next((one for one in windows if one[1] <= start < one[2]), None)
        before, after = portions(start, ends, window[3] if window else None)
        if held and window is not None and held[-1].window is window:
            held[-1].before -= before
            held[-1].after -= after
        held.append(Held(places.get(index, "?"), window, start, ends, before, after))
    while held:
        tallied_in(spans, held.pop())
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


def ranges_in(trace: Path) -> int:
    """Give how many ranges the trace holds."""
    root = ET.parse(trace).getroot()  # noqa: S314, the trace this run wrote
    return len(list(root.iter("range")))


def merged(into: Spans, spans: Spans) -> None:
    """Add each place's tally in `spans` to its own in `into`, span by span."""
    for name, places in spans.items():
        for place, spent in places.items():
            tally = into[name][place]
            tally.count += spent.count
            tally.own += spent.own
            tally.whole += spent.whole


def gathered(traces: list[tuple[Path, bool]], asked: set[str]) -> tuple[Spans, list[str]]:
    """Give every trace's spans added together, and each of their openings said.

    Each trace is a life of the shell's, and whether its profiler began with the shell.
    """
    spans = spans_made()
    opened: list[str] = []
    for trace, fresh in traces:
        merged(spans, profiled(trace, asked))
        split, lines = settled(trace, fresh=fresh)
        merged(spans, split)
        opened.extend(f"{trace.name} {line}" for line in lines)
    return spans, opened
