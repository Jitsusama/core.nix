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
    acting on it is safe for a test no later state can undo.

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
