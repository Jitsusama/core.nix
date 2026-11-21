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

# Who said what was heard, a subject and its verb, as the launcher's wait gives it.
WHO = "it said"


def test_a_wait_never_passed_says_the_last_state_it_heard() -> None:
    # The launcher's reads join their parts with a middle dot, which the message keeps as it
    # is rather than as an escape.
    def hear(passes: Callable[[str], object]) -> None:
        for said in ('{"rows": ["Firmware"]}', '{"rows": ["Install 1.13.0 \\u00b7 ready"]}'):
            assert not passes(said), f"{said} passed a test it fails"
        message = "never heard rows of Restart"
        raise AssertionError(message)

    with pytest.raises(AssertionError) as never:
        until_heard({"rows": []}, lambda now: now["rows"] == ["Restart"], hear, WHO)
    wanted = (
        "never heard rows of Restart\n"
        'it said, last heard: {"rows": ["Install 1.13.0 \u00b7 ready"]}'
    )
    assert str(never.value) == wanted, f"the wait that never passed said: {never.value}"


def test_a_wait_that_hears_nothing_says_the_state_it_first_read() -> None:
    def hear(_passes: Callable[[str], object]) -> None:
        message = "never heard rows of Restart"
        raise AssertionError(message)

    with pytest.raises(AssertionError) as never:
        until_heard({"rows": ["Install"]}, lambda now: now["rows"] == ["Restart"], hear, WHO)
    wanted = 'never heard rows of Restart\nit said, last heard: {"rows": ["Install"]}'
    assert str(never.value) == wanted, f"the wait that heard nothing said: {never.value}"


def test_a_tests_own_assertion_fails_the_wait_and_says_the_last_state_heard() -> None:
    # A test may assert what must never show, and its failure is the wait's, said the same
    # way, with the state that tripped it.
    def holds(now: dict[str, list[str]]) -> bool:
        assert now["rows"] != ["Install"], "the rows went back to Install"
        return now["rows"] == ["Restart"]

    def hear(passes: Callable[[str], object]) -> None:
        passes('{"rows": ["Firmware"]}')
        passes('{"rows": ["Install"]}')

    with pytest.raises(AssertionError) as never:
        until_heard({"rows": []}, holds, hear, WHO)
    # The assert's own message runs on with what pytest adds to it, so its ends are read.
    said = str(never.value).splitlines()
    wanted = ["the rows went back to Install", 'it said, last heard: {"rows": ["Install"]}']
    assert [said[0], said[-1]] == wanted, f"the wait whose test asserted said: {never.value}"


def test_only_the_change_that_passes_is_said_to_pass_and_is_given_back() -> None:
    # What hears stops at the first change the wait says passed, as reported does, so this
    # holds that the wait says so of the change that passes and of no other; that the
    # hearer stops is reported's to keep, on the VM.
    heard: list[str] = []
    said_in_turn = ('{"rows": ["Install"]}', '{"rows": ["Restart"]}', '{"rows": []}')

    def hear(passes: Callable[[str], object]) -> None:
        for said in said_in_turn:
            heard.append(said)
            if passes(said):
                return
        message = "no change passed"
        raise AssertionError(message)

    passed = until_heard({"rows": []}, lambda now: now["rows"] == ["Restart"], hear, WHO)
    wanted = list(said_in_turn[:2])
    assert heard == wanted, f"the wait heard on past the change that passed: {heard}"
    assert passed == {"rows": ["Restart"]}, f"the wait gave back {passed}, not what passed"


def test_a_test_that_trips_keeps_its_error_and_says_the_last_state_heard() -> None:
    # A state with no rows trips a test that reads its first row, as many launcher waits do.
    def hear(passes: Callable[[str], object]) -> None:
        passes('{"rows": []}')

    with pytest.raises(IndexError) as tripped:
        until_heard({"rows": ["Install"]}, lambda now: now["rows"][0] == "Restart", hear, WHO)
    notes = getattr(tripped.value, "__notes__", [])
    assert notes == ['it said, last heard: {"rows": []}'], f"the tripped wait noted: {notes}"


def test_a_test_that_trips_on_the_first_read_keeps_its_error_and_says_that_read() -> None:
    def hear(_passes: Callable[[str], object]) -> None:
        message = "heard though the first read tripped"
        raise AssertionError(message)

    with pytest.raises(IndexError) as tripped:
        until_heard({"rows": []}, lambda now: now["rows"][0] == "Restart", hear, WHO)
    notes = getattr(tripped.value, "__notes__", [])
    wanted = ['it said, last heard: {"rows": []}']
    assert notes == wanted, f"the first read's trip noted: {notes}"


def test_a_wait_that_stops_hearing_before_a_state_passes_fails() -> None:
    # Only a hearer that stops at a pass, as reported does, may end the wait without an error,
    # so one that stops early can't hand back a state its test refused as though it passed.
    def hear(passes: Callable[[str], object]) -> None:
        passes('{"rows": ["Install"]}')

    with pytest.raises(AssertionError) as never:
        until_heard(
            {"rows": []}, lambda now: now["rows"] == ["Restart"], hear, "the launcher showed"
        )
    wanted = (
        "the launcher showed nothing that passed before the wait stopped hearing\n"
        'the launcher showed, last heard: {"rows": ["Install"]}'
    )
    assert str(never.value) == wanted, f"the wait that stopped early said: {never.value}"


def test_a_wait_whose_hearer_heard_nothing_fails_on_the_first_read() -> None:
    def hear(_passes: Callable[[str], object]) -> None:
        return

    with pytest.raises(AssertionError) as never:
        until_heard({"rows": []}, lambda now: now["rows"] == ["Restart"], hear, WHO)
    wanted = (
        "it said nothing that passed before the wait stopped hearing\n"
        'it said, last heard: {"rows": []}'
    )
    assert str(never.value) == wanted, f"the wait that heard nothing said: {never.value}"


def test_only_the_last_change_heard_counts() -> None:
    # A state that passed and then one refused, before the hearer stopped, is the refused one.
    def hear(passes: Callable[[str], object]) -> None:
        passes('{"rows": ["Restart"]}')
        passes('{"rows": ["Install"]}')

    with pytest.raises(AssertionError) as never:
        until_heard({"rows": []}, lambda now: now["rows"] == ["Restart"], hear, WHO)
    assert str(never.value).endswith('last heard: {"rows": ["Install"]}'), str(never.value)


def test_a_line_that_isnt_json_is_said_as_it_came() -> None:
    # A journal can split a long line, and the half heard is what explains the failure, not
    # the state before it.
    def hear(passes: Callable[[str], object]) -> None:
        passes('{"rows": ["Inst')

    with pytest.raises(ValueError, match="Unterminated") as broke:
        until_heard({"rows": ["Install"]}, lambda now: now["rows"] == ["Restart"], hear, WHO)
    notes = getattr(broke.value, "__notes__", [])
    wanted = ['it said, last heard, not as JSON: \'{"rows": ["Inst\'']
    assert notes == wanted, f"the split line noted: {notes}"


def test_a_state_passing_at_first_read_hears_nothing() -> None:
    def hear(_passes: Callable[[str], object]) -> None:
        message = "heard though the first read passed"
        raise AssertionError(message)

    restart = {"rows": ["Restart"]}
    passed = until_heard(restart, lambda now: now == restart, hear, WHO)
    assert passed is restart, f"the wait gave back {passed}, not the first read that passed"
