"""How a desktop area waits on what a program reports, so one that never passes says why.

The launcher reports what it shows each time that changes, and a wait reads it once and then
hears every change (desktop_session.launcher_until). It lives apart from desktop_session so its
cases run off any VM (heard-test.py), and apart from vm_helpers, which every VM test loads, so a
change to it moves only the desktop's areas.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any, cast

if TYPE_CHECKING:
    from collections.abc import Callable

# What reports (reports.sh) ends with when its time runs out, kept apart from the 1 a failed
# read gives; heard-test.py runs it to hold the two together.
TIMED_OUT = 3
# What the driver's own limit on a command ends it with (desktop-starts-clean pins it). One
# past reports' own end is the wait running out on a starved guest, not a reader stopping.
CUT_SHORT = 124


def said_by(read: str) -> list[tuple[str, str]]:
    """Give what a read by reports said, each line as its kind and what follows the tab.

    It splits only where the reader does, at a newline: what the launcher shows is JSON, which
    keeps a line or paragraph separator someone typed as it is, and Python's splitlines would
    break a report there into pieces that aren't.
    """
    lines = (line.split("\t", 1) for line in read.split("\n") if line)
    return [(kind, what) for kind, what in lines]


def ended(status: int, never: Callable[[], str]) -> None:
    """Say how a read of what's reported ended, by the status it stopped with.

    It says nothing where the read did, so the wait goes on with what it said. Its time running
    out, by reports' 3 or the driver's cut-off, fails by AssertionError, which until_heard
    reads as never hearing what's wanted. The driver's cut-off counts whatever made the run
    overrun, a reader blocked past it too, since the guest's status can't tell them apart. Any
    other status means a reader stopped, so it fails by RuntimeError, which until_heard only
    notes, since what it last heard then is no sign of where the program settled. Never says
    why, and is asked only when the read failed.
    """
    if status in {TIMED_OUT, CUT_SHORT}:
        raise AssertionError(never())
    if status != 0:
        message = f"{never()}, since reports stopped with {status}"
        raise RuntimeError(message)


def diagnosed(execute: Callable[[], tuple[int, str]]) -> str:
    """Give what a read for a failure's diagnosis said, never raising.

    A session gone bad is when its read fails too, and that error would stand in for the
    failure it's read to explain. So a read the driver cut short says so after what it said,
    one that fails says how it ended after what it said, without calling the rest unread,
    since it may have read all there was, and one that raises says what it raised, rather
    than being taken for a program that said nothing. Either note starts a line of its own,
    since a read cut off mid-line would run on into it.
    """
    try:
        status, said = execute()
    except Exception as broke:  # noqa: BLE001 # What it raised is the diagnosis.
        return f"(unread, since it raised {broke!r})"
    if status == 0:
        return said
    if said and not said.endswith("\n"):
        said += "\n"
    stopped = "unread past here, stopping with" if status == CUT_SHORT else "ended with"
    return f"{said}({stopped} {status})"


def shown(
    answer: str,
    why: Callable[[], str],
    who: str,
) -> Any:  # noqa: ANN401 # Whatever JSON the state is, as it was said.
    """Give the state a program's call for it answered, failing where that says nothing of it.

    A call once answered an empty line, which failed vm-desktop-looks as a bare JSONDecodeError
    and kept nothing of why (e73f6fe8, 22 Nov); a throw is the likeliest cause, unproven. So an
    answer that isn't JSON, or one naming what threw, as the launcher's showing call says it
    (launcher/Held.js's answered), fails saying what came, then why, the program's journal,
    read only then since each read costs a paced call. It fails by RuntimeError, as a reader
    that stopped does (ended), never AssertionError, which the waits that press again catch
    as time running out and would press past. A call that exits other than 0 fails before
    this, as the account's command. Who is a subject and its verb, "the launcher's showing
    call answered".
    """
    try:
        state = json.loads(answer)
    except ValueError:
        message = f"{who} {answer!r}, not a state as JSON\n{why()}"
        raise RuntimeError(message) from None
    if isinstance(state, dict) and "threw" in state:
        message = f"{who} that its read threw {state['threw']}\n{why()}"
        raise RuntimeError(message)
    return state


def until_heard(
    first: Any,  # noqa: ANN401 # Whatever JSON the state is, as it was said.
    test: Callable[[Any], object],
    hear: Callable[[Callable[[str], object]], object],
    who: str,
) -> Any:  # noqa: ANN401 # The state that passed, as it was said.
    """Wait until a state passes the test, from the first read and each change heard after.

    It gives back the state that passed, so a caller can act on what was seen rather than
    asking again, when the program may have moved on since. A change said just before the
    first read is heard after it, so the state given back can be older than the first read;
    acting on it is safe for a test no later state can undo. Who is a subject and its verb,
    "the launcher showed", since the messages say what it did and didn't.

    Hear calls what it's given with each change said, as JSON, until that passes, and raises
    AssertionError when none does. That error then says what was last heard, on a line of its
    own after whatever the error said, since a journal holds a line only for each change,
    and a state settled wrong before the wait began says none. It's kept as it's heard
    rather than asked for again, since asking a program that hung or went is what would
    fail then, in place of the failure it's meant to explain. A test's own AssertionError reads
    the same way, as the wait failing, and a hear that returns before any change passed fails
    too, so a state the test refused is never given back as though it passed.

    Anything else raised on the way, a test tripping on a state shaped other than it expects or
    a line the journal split, keeps its own kind, with a note saying what was last heard: the
    first read too, and a line that isn't JSON as it came, since the state before it explains
    nothing.
    """
    heard = [first]
    unread: list[str] = []
    passed: list[object] = []

    def passes(said: str) -> object:
        unread[:] = [said]
        heard[0] = json.loads(said)
        unread.clear()
        passed[:] = [test(heard[0])]
        return passed[0]

    def last() -> str:
        if unread:
            return f"last heard, not as JSON: {unread[0]!r}"
        return f"last heard: {json.dumps(heard[0], ensure_ascii=False)}"

    try:
        if test(first):
            return first
        hear(passes)
    except AssertionError as never:
        message = f"{never}\n{who}, {last()}"
        raise AssertionError(message) from never
    except Exception as broke:
        broke.add_note(f"{who}, {last()}")
        raise
    if passed and passed[0]:
        return heard[0]
    message = f"{who} nothing that passed before the wait stopped hearing\n{who}, {last()}"
    raise AssertionError(message)


# The fewest reads that can show a word moving, and the most moved words a shot's
# failure names before saying how many more (moved_over).
TWO_READS = 2
MOVED_NAMED = 30


def flattened(value: object, at: str = "") -> dict[str, object]:
    """Give each leaf of a JSON value by its path, as rows[2].words[0].text, to compare two."""
    if isinstance(value, dict):
        pairs = cast("dict[str, object]", value).items()
        return {k: v for key, one in pairs for k, v in flattened(one, f"{at}.{key}").items()}
    if isinstance(value, list):
        items = enumerate(cast("list[object]", value))
        return {k: v for n, one in items for k, v in flattened(one, f"{at}[{n}]").items()}
    return {at.lstrip("."): value}


def whole(said: str) -> object:
    """Give what a read said as JSON, or nothing for one empty, cut short or thrown.

    The launcher answers a call that threw with {"threw": why} (Held.js), which is no
    reading of what it shows, so it's passed over as one cut short is, rather than kept as a
    read whose every path then reads as moved (round 2 on #6998, 39).
    """
    try:
        read = json.loads(said)
    except ValueError:
        return {}
    return {} if isinstance(read, dict) and "threw" in read else read


def moved_over(seen: list[dict[str, object]]) -> str:
    """Say which paths' values differ between any two of `seen`, each a read flattened.

    A path one read lacks, as a list grown shorter, says "absent" there, apart from one that
    is null; each moved path says its values in the order they were read, once each.
    """
    if len(seen) < TWO_READS:
        return "the launcher's laidOut couldn't be read twice to say what moved"
    paths = sorted({path for one in seen for path in one})

    def worded(one: dict[str, object], path: str) -> str:
        return repr(one[path]) if path in one else "absent"

    moved = [
        f"{path}: {' / '.join(dict.fromkeys(worded(one, path) for one in seen))}"
        for path in paths
        if len({worded(one, path) for one in seen}) > 1
    ]
    if not moved:
        return f"none of the launcher's laid-out words moved over {len(seen)} reads"
    shown = "; ".join(moved[:MOVED_NAMED])
    more = f" and {len(moved) - MOVED_NAMED} more" if len(moved) > MOVED_NAMED else ""
    return f"the launcher's laid-out words that moved over {len(seen)} reads: {shown}{more}"
