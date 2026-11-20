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
    # The launcher's reads join their parts with a middle dot, which the message keeps as it
    # is rather than as an escape.
    def hear(passes: Callable[[str], object]) -> None:
        for said in ('{"rows": ["Firmware"]}', '{"rows": ["Install 1.13.0 \\u00b7 ready"]}'):
            assert not passes(said), f"{said} passed a test it fails"
        message = "never heard rows of Restart"
        raise AssertionError(message)

    with pytest.raises(AssertionError) as never:
        until_heard({"rows": []}, lambda now: now["rows"] == ["Restart"], hear, "it")
    wanted = (
        'never heard rows of Restart; it, last heard: {"rows": ["Install 1.13.0 \u00b7 ready"]}'
    )
    assert str(never.value) == wanted, f"the wait that never passed said: {never.value}"


def test_a_wait_that_hears_nothing_says_the_state_it_first_read() -> None:
    def hear(_passes: Callable[[str], object]) -> None:
        message = "never heard rows of Restart"
        raise AssertionError(message)

    with pytest.raises(AssertionError) as never:
        until_heard({"rows": ["Install"]}, lambda now: now["rows"] == ["Restart"], hear, "it")
    wanted = 'never heard rows of Restart; it, last heard: {"rows": ["Install"]}'
    assert str(never.value) == wanted, f"the wait that heard nothing said: {never.value}"


def test_a_change_that_passes_is_told_so() -> None:
    # What hears stops at the first change the wait says passed, as reported does, so this
    # holds that the wait says so of the change that passes and of no other.
    heard: list[str] = []
    said_in_turn = ('{"rows": ["Install"]}', '{"rows": ["Restart"]}', '{"rows": []}')

    def hear(passes: Callable[[str], object]) -> None:
        for said in said_in_turn:
            heard.append(said)
            if passes(said):
                return
        message = "no change passed"
        raise AssertionError(message)

    until_heard({"rows": []}, lambda now: now["rows"] == ["Restart"], hear, "it")
    wanted = list(said_in_turn[:2])
    assert heard == wanted, f"the wait heard on past the change that passed: {heard}"


def test_a_state_passing_at_first_read_hears_nothing() -> None:
    def hear(_passes: Callable[[str], object]) -> None:
        message = "heard though the first read passed"
        raise AssertionError(message)

    restart = {"rows": ["Restart"]}
    until_heard(restart, lambda now: now == restart, hear, "it")
