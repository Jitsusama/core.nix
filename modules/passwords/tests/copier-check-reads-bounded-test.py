"""Checks the copier check's reads of how a copier ended are bounded, with fake copiers.

copier-keeps-a-secret-in-memory.py reads what a copier said, how it ended and whether it
read its standard input, each for as long as its PATIENCE and no longer, so a copier that
holds on, or leaves a child holding its standard error open, fails a case rather than hanging
the check. Its own run needs a display, so these reads are pinned here with copiers that are
bash scripts: each ending one way, saying one thing or never ending at all. PATIENCE is set
to 2 s, so a read that waits past it plus a second fails its case.

usage: copier-check-reads-bounded-test.py <bash> <copier-keeps-a-secret-in-memory.py>
"""

import contextlib
import importlib.util
import os
import signal
import subprocess
import sys
import time
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator
    from types import ModuleType

# How long each read is given, in seconds, set on the copier check as its PATIENCE.
PATIENCE = 2.0
# How long past PATIENCE a bounded read may take to answer, in seconds.
SLACK = 1.0
# The data fed to a reader, of a MiB, the copier's cap, so it outgrows a pipe's buffer.
DATA = b"x" * (1024 * 1024)
# Why a fake copier refused, for the refusals it's judged by.
WHY = "wl-copy: refused a sensitive copy: it is past 1 MiB"


def loaded(path: str) -> ModuleType:
    """Load the copier check from its file, its PATIENCE cut to this test's."""
    spec = importlib.util.spec_from_file_location("copier_check", path)
    if spec is None or spec.loader is None:
        why = f"{path} isn't a module Python can load"
        raise SystemExit(why)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    # A module's type reads any name but sets none, so the patience is set in its namespace.
    vars(module)["PATIENCE"] = PATIENCE
    return module


BASH = sys.argv[1]
CHECK = loaded(sys.argv[2])


@contextlib.contextmanager
def copier(script: str) -> Iterator[subprocess.Popen[bytes]]:
    """Start a fake copier running `script`, and kill it and whatever it left once done.

    It starts in a session of its own, so a child it left behind is killed with its group.
    """
    started = subprocess.Popen(
        [BASH, "-c", script],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=True,
    )
    try:
        yield started
    finally:
        with contextlib.suppress(ProcessLookupError):
            os.killpg(started.pid, signal.SIGKILL)
        started.wait()
        for pipe in (started.stdout, started.stderr):
            if pipe is not None:
                pipe.close()


def timed[T](read: Callable[[], T]) -> tuple[T, float]:
    """Answer what `read` answers and how long it took, in seconds."""
    began = time.monotonic()
    answer = read()
    return answer, time.monotonic() - began


def a_running_copier_has_not_refused() -> None:
    """refused_instead answers None for a copier still running."""
    with copier("exec sleep 30") as running:
        refused = CHECK.refused_instead(running)
    assert refused is None, f"a running copier was said to have refused: {refused!r}"


def a_failing_copier_is_refused_with_what_it_said() -> None:
    """refused_instead names how a copier ended and what it said on its standard error."""
    with copier("echo nope >&2; exit 3") as failing:
        failing.wait()
        refused = CHECK.refused_instead(failing)
    wanted = "it ended with 3, saying 'nope'"
    assert refused == wanted, f"a copier ending with 3 was said {refused!r}, not {wanted!r}"


def a_copier_ending_well_is_said_to_have_copied() -> None:
    """refused_instead says a copier ending with 0 ended as if it had copied."""
    with copier("exit 0") as ending_well:
        ending_well.wait()
        refused = CHECK.refused_instead(ending_well)
    wanted = "it ended with 0, as if it had copied, yet nothing pasted"
    assert refused == wanted, f"a copier ending with 0 was said {refused!r}, not {wanted!r}"


def a_child_holding_standard_error_open_is_waited_on_no_longer_than_patience() -> None:
    """refused_instead reads a copier whose child holds its standard error for PATIENCE alone.

    It still says how the copier ended and what it said before it ended.
    """
    with copier("(sleep 30 >&2 &) ; echo gone >&2; exit 4") as leaving:
        leaving.wait()
        refused, took = timed(lambda: CHECK.refused_instead(leaving))
    assert took < PATIENCE + SLACK, f"a child holding standard error open held it {took:.1f} s"
    wanted = f"it ended with 4, its standard error still open after {PATIENCE:g} s"
    assert refused is not None, "a copier that ended with 4 was said not to have refused"
    assert refused.startswith(wanted), f"it was said {refused!r}, not {wanted!r}"
    assert "gone" in refused, f"it was said {refused!r}, without what it said, 'gone'"


def a_copier_never_ending_is_ended_after_patience() -> None:
    """ended_within answers None for a copier that never ends, and kills it, within PATIENCE."""
    with copier("exec sleep 30") as holding:
        (code, _, _), took = timed(lambda: CHECK.ended_within(holding))
        assert code is None, f"a copier that never ends was said to end with {code}"
        assert took < PATIENCE + SLACK, f"a copier that never ends held it {took:.1f} s"
        assert holding.poll() is not None, "a copier that never ends was left running"


def each_refusal_names_what_was_wrong() -> None:
    """Check refusal says a copier held on, ended well or said the wrong thing, alone."""
    held = CHECK.refusal(None, "", WHY)
    assert held == "it held the clipboard", f"a copier holding on was said {held!r}"
    copied = CHECK.refusal(0, "", WHY)
    assert copied == "it ended as if it had copied", f"one ending with 0 was said {copied!r}"
    other = CHECK.refusal(2, "other", WHY) or ""
    assert "other" in other, f"one saying 'other' was said {other!r}, without what it said"
    assert WHY in other, f"one saying 'other' was said {other!r}, without what it should say"
    refused = CHECK.refusal(2, WHY, WHY)
    assert refused is None, f"one refusing as it should was said {refused!r}"


def a_reader_never_reading_is_fed_no_longer_than_patience() -> None:
    """Check fed gives up on a reader that never reads within PATIENCE, saying what's left.

    It waits on the reader rather than trying to write over and over, so it spends under
    half of PATIENCE on the processor meanwhile.
    """
    reading, writing = os.pipe()
    try:
        spent = time.process_time()
        stalled, took = timed(lambda: CHECK.fed(writing, DATA))
        spent = time.process_time() - spent
    finally:
        os.close(reading)
    assert took < PATIENCE + SLACK, f"a reader that never reads held it {took:.1f} s"
    assert spent < PATIENCE / 2, f"it spent {spent:.1f} s on the processor feeding it"
    wanted = "it stopped reading with "
    assert stalled is not None, "a reader that never reads was said to read all of it"
    assert stalled.startswith(wanted), f"it was said {stalled!r}, not {wanted!r}"
    assert stalled.endswith(" bytes left"), f"it was said {stalled!r}, not what was left"
    left = int(stalled.removeprefix(wanted).removesuffix(" bytes left"))
    assert 0 < left < len(DATA), f"it was said {left} bytes were left, of {len(DATA)}"


def a_reader_that_closed_is_fed_without_fault() -> None:
    """Check fed answers None for a reader that closed its end, as it has read its last."""
    reading, writing = os.pipe()
    os.close(reading)
    stalled = CHECK.fed(writing, DATA)
    assert stalled is None, f"a reader that closed was said {stalled!r}"


def a_silent_copier_says_nothing_after_patience() -> None:
    """said_next answers nothing for a copier that says nothing, within PATIENCE."""
    with copier("exec sleep 30") as silent:
        said, took = timed(lambda: CHECK.said_next(silent))
    assert said == "", f"a copier saying nothing was said to say {said!r}"
    assert took < PATIENCE + SLACK, f"a copier saying nothing held it {took:.1f} s"


CASES = [
    a_running_copier_has_not_refused,
    a_failing_copier_is_refused_with_what_it_said,
    a_copier_ending_well_is_said_to_have_copied,
    a_child_holding_standard_error_open_is_waited_on_no_longer_than_patience,
    a_copier_never_ending_is_ended_after_patience,
    each_refusal_names_what_was_wrong,
    a_reader_never_reading_is_fed_no_longer_than_patience,
    a_reader_that_closed_is_fed_without_fault,
    a_silent_copier_says_nothing_after_patience,
]


def main() -> None:
    """Run every case, and fail naming each that didn't hold."""
    failed: list[str] = []
    for case in CASES:
        try:
            case()
        except AssertionError as wrong:
            failed.append(case.__name__)
            print(f"{case.__name__}: {wrong}", file=sys.stderr)
        else:
            print(f"{case.__name__}: held")
    if failed:
        print(f"failed: {', '.join(failed)}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
