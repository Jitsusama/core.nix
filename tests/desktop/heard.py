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
    asking again, when the program may have moved on since.

    Hear calls what it's given with each change said, as JSON, until that passes, and raises
    AssertionError when none does. That error then says what was last heard, since a journal
    holds a line only for each change, and a state settled wrong before the wait began says
    none. It's kept as it's heard rather than asked for again, since asking a program that
    hung or went is what would fail then, in place of the failure it's meant to explain.

    Anything else raised on the way, a test tripping on a state shaped other than it expects or
    a line the journal split, keeps its own kind, with a note saying what was last heard.
    """
    heard = [first]
    if test(first):
        return first

    def passes(said: str) -> object:
        heard[0] = json.loads(said)
        return test(heard[0])

    try:
        hear(passes)
    except AssertionError as never:
        last = json.dumps(heard[0], ensure_ascii=False)
        message = f"{never}; {who}, last heard: {last}"
        raise AssertionError(message) from never
    except Exception as broke:
        broke.add_note(f"{who}, last heard: {json.dumps(heard[0], ensure_ascii=False)}")
        raise
    return heard[0]
