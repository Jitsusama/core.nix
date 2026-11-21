"""How a desktop area's wait on a program's reports ends (heard.py), with no machine to load.

A wait that never passes has to say what it last heard, since that's all a failed net run
keeps of what the launcher showed.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from heard import CUT_SHORT, TIMED_OUT, diagnosed, ended, until_heard

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


def test_a_wait_timing_out_on_nothing_heard_says_the_state_it_first_read() -> None:
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


def test_a_wait_whose_hearer_returns_having_heard_nothing_fails_on_the_first_read() -> None:
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
    # A line heard that isn't JSON, as when a journal splits a long line just after a "]" and
    # the reader passes its first half on, explains the failure, not the state before it.
    def hear(passes: Callable[[str], object]) -> None:
        passes('{"rows": ["Inst')

    with pytest.raises(ValueError, match="Unterminated") as broke:
        until_heard({"rows": ["Install"]}, lambda now: now["rows"] == ["Restart"], hear, WHO)
    notes = getattr(broke.value, "__notes__", [])
    wanted = ['it said, last heard, not as JSON: \'{"rows": ["Inst\'']
    assert notes == wanted, f"the split line noted: {notes}"


def never() -> str:
    return "never reported showing"


def test_a_read_that_stopped_with_nothing_wrong_ends_nothing() -> None:
    # Never isn't asked, since its read of the journal costs a paced read on the VM.
    def asked() -> str:
        message = "asked why though the read ended well"
        raise AssertionError(message)

    ended(0, asked)


@pytest.mark.parametrize("status", [TIMED_OUT, CUT_SHORT])
def test_a_read_whose_time_ran_out_fails_as_never_hearing(status: int) -> None:
    # Retry loops press again on an AssertionError, and until_heard rewrites it as the wait's.
    with pytest.raises(AssertionError) as never_heard:
        ended(status, never)
    assert str(never_heard.value) == never(), f"{status} ended as: {never_heard.value}"


def test_a_reader_that_stopped_otherwise_fails_as_no_waits_end() -> None:
    # A journal read failing gives 1, which isn't time running out, so no retry takes it.
    with pytest.raises(RuntimeError) as stopped:
        ended(1, never)
    wanted = "never reported showing, since reports stopped with 1"
    assert str(stopped.value) == wanted, f"a stopped reader ended as: {stopped.value}"


def test_a_diagnosis_read_says_what_it_read() -> None:
    said = diagnosed(lambda: (0, "report[showing {}]\n"))
    assert said == "report[showing {}]\n", f"a read that worked told: {said!r}"


def test_a_diagnosis_read_that_stops_says_so_after_what_it_read() -> None:
    said = diagnosed(lambda: (CUT_SHORT, "first line\n"))
    wanted = "first line\n(unread past here, stopping with 124)"
    assert said == wanted, f"a read cut short told: {said!r}"


def test_a_diagnosis_read_cut_off_mid_line_says_so_on_a_line_of_its_own() -> None:
    # The driver's limit cuts a read wherever it is, so its note would run on into a state.
    said = diagnosed(lambda: (CUT_SHORT, '{"rows": ["Inst'))
    wanted = '{"rows": ["Inst\n(unread past here, stopping with 124)'
    assert said == wanted, f"a read cut off mid-line told: {said!r}"


@pytest.mark.parametrize("said", ["", "first line\n"])
def test_a_diagnosis_read_that_fails_says_how_it_ended_after_what_it_read(said: str) -> None:
    # A journal read that failed may have read all there was, so nothing is called unread.
    told = diagnosed(lambda: (1, said))
    assert told == f"{said}(ended with 1)", f"a read that failed after {said!r} told: {told!r}"


def test_a_reader_stopping_in_a_wait_keeps_its_kind_and_says_the_last_state_heard() -> None:
    # Retry loops press again on an AssertionError, so a stopped reader must come out as itself.
    def hear(passes: Callable[[str], object]) -> None:
        passes('{"rows": ["Install"]}')
        ended(1, never)

    with pytest.raises(RuntimeError) as stopped:
        until_heard({"rows": []}, lambda now: now["rows"] == ["Restart"], hear, WHO)
    notes = getattr(stopped.value, "__notes__", [])
    wanted = ['it said, last heard: {"rows": ["Install"]}']
    assert notes == wanted, f"the stopped reader's wait noted: {notes}"


def reports(
    tmp_path: Path, journal: str, seconds: int = 5, since: str = ""
) -> subprocess.CompletedProcess[str]:
    """Run the reader the waits read through, for showing, on a stand-in journal.

    The stand-in journalctl writes what it was asked to a line of asked, then runs the journal
    as a shell's lines, and the reader runs on the shell and tools its check gives
    (checks.nix), as the VM's writeShellApplication runs it.
    """
    shell = os.environ["REPORTS_SHELL"]
    stand_in = tmp_path / "journalctl"
    stand_in.write_text(f"#!{shell}\nprintf '%s\\n' \"$*\" >>{tmp_path}/asked\n{journal}\n")
    stand_in.chmod(0o755)
    reader = Path(__file__).with_name("reports.sh")
    return subprocess.run(
        [shell, "-euo", "pipefail", reader, since, str(seconds), "showing"],
        capture_output=True,
        text=True,
        env={"PATH": f"{tmp_path}:{os.environ['REPORTS_TOOLS']}"},
        check=False,
    )


def asked(tmp_path: Path) -> list[str]:
    """Give what the stand-in journal was asked, a read to a line."""
    return (tmp_path / "asked").read_text().splitlines()


def test_the_reader_says_what_follows_a_lines_first_report_mark(tmp_path: Path) -> None:
    # What the launcher shows can hold the mark again, as typed words, which stay its words.
    # The line's prefix holds an n, which a bracket read as neither \ nor n would stop at.
    line = 'launcher qml: report[showing {"typed": "report[x]"}]'
    read = reports(tmp_path, f"echo '{line}'; echo '-- cursor: c1'")
    wanted = 'said\tshowing {"typed": "report[x]"}\ncursor\tc1\n'
    assert (read.returncode, read.stdout) == (0, wanted), f"the reader said: {read}"
    reads = asked(tmp_path)
    since_boot = "--user --unit=quickshell --no-pager --output=cat --lines=all --show-cursor --boot"
    assert reads == [since_boot], f"with no point the reader asked: {reads}"


def test_the_reader_reads_again_after_its_point_until_the_report_comes(tmp_path: Path) -> None:
    # The reader reads the journal again rather than following it, so a report a later read
    # finds is heard, and each read starts at the point it was given, which gives the cursor
    # back only when asked to show it.
    journal = (
        f'if [ "$(wc -l <{tmp_path}/asked)" -lt 2 ]; then state=hidden; else state=showing; fi\n'
        'echo "qml: report[$state {}]"\n'
        '[[ " $* " == *" --show-cursor "* ]] && echo "-- cursor: c2"'
    )
    read = reports(tmp_path, journal, since="c1")
    wanted = "said\tshowing {}\ncursor\tc2\n"
    assert (read.returncode, read.stdout) == (0, wanted), f"the reader said: {read}"
    reads = asked(tmp_path)
    after = "--user --unit=quickshell --no-pager --output=cat --lines=all --show-cursor"
    assert reads == [f"{after} --after-cursor=c1"] * 2, f"from c1 the reader asked: {reads}"


def test_the_readers_time_running_out_fails_as_never_hearing(tmp_path: Path) -> None:
    # The status ended reads as time running out is the one the reader gives, or retry loops
    # would stop retrying a wait that only ran out, and it comes only once it has read again.
    read = reports(tmp_path, "echo 'qml: report[hidden {}]'; echo '-- cursor: c1'", seconds=1)
    with pytest.raises(AssertionError):
        ended(read.returncode, never)
    assert read.returncode == TIMED_OUT, f"the reader whose time ran out gave: {read}"
    reads = len(asked(tmp_path))
    assert reads > 1, f"the reader whose time ran out read {reads} times"


def test_the_readers_failed_journal_read_fails_as_a_stopped_reader(tmp_path: Path) -> None:
    read = reports(tmp_path, "exit 1")
    with pytest.raises(RuntimeError):
        ended(read.returncode, never)


def test_a_diagnosis_read_that_raises_says_what_it_raised_rather_than_raising() -> None:
    # The driver raising, when the guest's shell has gone, is when a diagnosis is read.
    def execute() -> tuple[int, str]:
        message = "the guest's shell went"
        raise ConnectionError(message)

    said = diagnosed(execute)
    wanted = '(unread, since it raised ConnectionError("the guest\'s shell went"))'
    assert said == wanted, f"a read that raised told: {said!r}"


def test_a_state_passing_at_first_read_hears_nothing() -> None:
    def hear(_passes: Callable[[str], object]) -> None:
        message = "heard though the first read passed"
        raise AssertionError(message)

    restart = {"rows": ["Restart"]}
    passed = until_heard(restart, lambda now: now == restart, hear, WHO)
    assert passed is restart, f"the wait gave back {passed}, not the first read that passed"
