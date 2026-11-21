"""How a desktop area waits on what a program reports, so one that never passes says why.

The launcher reports what it shows each time that changes, and a wait reads it once and then
hears every change (desktop_session.launcher_until). It lives apart from desktop_session so its
cases run off any VM (heard-test.py), and apart from vm_helpers, which every VM test loads, so a
change to it moves only the desktop's areas.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Callable

# What reports (tests/desktop.nix) ends with when its time runs out, kept apart from the 1
# a failed read gives.
TIMED_OUT = 3
# What the driver's own limit on a command ends it with (desktop-starts-clean pins it). One
# past reports' own end is the wait running out on a starved guest, not a reader stopping.
CUT_SHORT = 124


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
    failure it's read to explain. So a read that stops says its status after what it said,
    and one that raises says what it raised, rather than being taken for a program that said
    nothing.
    """
    try:
        status, said = execute()
    except Exception as broke:  # noqa: BLE001 # What it raised is the diagnosis.
        return f"(unread, since it raised {broke!r})"
    return said if status == 0 else f"{said}(unread past here, stopping with {status})"


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
