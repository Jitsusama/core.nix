"""How a desktop area's wait on a program's reports ends (heard.py), with no machine to load.

A wait that never passes has to say what it last heard, since that's all a failed net run
keeps of what the launcher showed.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from heard import until_heard

if TYPE_CHECKING:
    from collections.abc import Callable


def test_a_wait_never_passed_says_the_last_state_it_heard() -> None:
    def hear(passes: Callable[[str], object]) -> None:
        for said in ('{"rows": ["Firmware"]}', '{"rows": ["Install 1.13.0"]}'):
            passes(said)
        message = "never heard rows of Restart"
        raise AssertionError(message)

    with pytest.raises(AssertionError) as never:
        until_heard({"rows": []}, lambda now: now["rows"] == ["Restart"], hear, "it")
    wanted = 'never heard rows of Restart; it, last heard: {"rows": ["Install 1.13.0"]}'
    assert str(never.value) == wanted, f"the wait that never passed said: {never.value}"


def test_a_state_passing_at_first_read_hears_nothing() -> None:
    def hear(_passes: Callable[[str], object]) -> None:
        message = "heard though the first read passed"
        raise AssertionError(message)

    restart = {"rows": ["Restart"]}
    until_heard(restart, lambda now: now == restart, hear, "it")
