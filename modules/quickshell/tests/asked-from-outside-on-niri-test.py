"""What the launcher's journal says of an ask from outside while reports are off, on niri.

modules/quickshell/checks/core.nix runs this through pytest inside tests/nested-niri.sh, with the
shell's reports off. A link, a terminal's `launcher do` or a catalogue run hands the launcher words
someone else wrote, a page's or a source's, so with reports off each line says them by kind and
how many letters, as the journal's other lines do (launcher/Presented.js), and never whole.
"""

from __future__ import annotations

import json
import re
import shlex
import time
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

# Words no row has, each one the line would say whole while reports were on.
MARKER = "HARBOURMARK"
# How a line says the marker with reports off.
COUNTED = f"{len(MARKER)} letters"
# How long a line has to reach the shell's log after its call answers, in seconds.
WRITTEN_WITHIN = 5.0
# How long what a link opens drops every key (Prompt.qml's linkQuiet), and a little more.
LINK_QUIET = 0.8


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once for the file's cases."""
    with launcher_on_niri.running() as started:
        yield started


def logged_since(shell: Shell, mark: int) -> str:
    """Give what the shell's log says since the mark, whole lines only."""
    with shell.log.open("rb") as log:
        log.seek(mark)
        written = log.read()
    return written[: written.rfind(b"\n") + 1].decode(errors="replace")


def asked(shell: Shell, start: str, *words: str) -> tuple[str, str]:
    """Call the launcher's IPC with these words, and give its answer and the log it left.

    The log is read until it holds the line that starts so, since the shell's log may reach the
    file a moment after the call answers, or the time to wait is up.
    """
    mark = shell.mark()
    answer = shell.as_account(shell.ipc(*(shlex.quote(word) for word in words)))
    deadline = time.monotonic() + WRITTEN_WITHIN
    said = logged_since(shell, mark)
    while f"launcher: {start}" not in said and time.monotonic() < deadline:
        time.sleep(0.1)
        said = logged_since(shell, mark)
    return answer, said


def line_of(said: str, start: str) -> str:
    """Give the launcher's line that starts so, or fail saying what it said instead."""
    lines = [line for line in said.splitlines() if f"launcher: {start}" in line]
    assert lines, f"no line saying {start!r} in:\n{said}"
    return lines[-1][lines[-1].index("launcher: ") :]


def close_after_link(shell: Shell) -> None:
    """Close what a link opened, once its quiet is over, since an Escape inside it is dropped."""
    time.sleep(LINK_QUIET)
    shell.close_launcher()


@pytest.mark.parametrize(
    ("call", "start"),
    [
        ("talkTo", "asked from outside to talk to "),
        ("goTo", "asked from outside to go to "),
        ("network", "asked from outside to show the network "),
    ],
)
def test_a_links_words_are_counted(shell: Shell, call: str, start: str) -> None:
    """A link's name is said by how many letters, since a page wrote it."""
    _, said = asked(shell, start, call, MARKER)
    line = line_of(said, start)
    assert MARKER not in said, f"{call} said the link's words: {line}"
    assert line.endswith(COUNTED), f"{call} didn't count the link's words: {line}"
    close_after_link(shell)


@pytest.mark.parametrize(
    ("call", "start", "missing"),
    [
        ("machine", "asked from outside to show the machine ", "the launcher has no Machines"),
        ("code", "asked from outside to show the repository ", "the launcher has no Code"),
    ],
)
def test_a_links_row_is_counted(shell: Shell, call: str, start: str, missing: str) -> None:
    """A link to a machine or a repository says what it named by letters, where it's listed."""
    answer, said = asked(shell, start, call, MARKER)
    assert MARKER not in said, f"{call} said the link's words:\n{said}"
    if missing in answer:
        pytest.skip(f"this shell has no place for {call}")
    assert line_of(said, start).endswith(COUNTED), said
    close_after_link(shell)


def first_place(shell: Shell) -> Json:
    """Give a place at the launcher's top whose name its id doesn't hold."""
    listed = cast("list[Json]", json.loads(shell.as_account(shell.ipc("list"))))
    places = [
        one
        for one in listed
        if one["kind"] == "place" and one["in"] == "" and str(one["words"]) not in str(one["id"])
    ]
    assert places, f"no place to walk to among {[one['words'] for one in listed]}"
    return places[0]


def test_a_catalogue_walk_says_its_rows_by_kind(shell: Shell) -> None:
    """A walk from outside says each row by its kind rather than its name, which a source gave."""
    place = first_place(shell)
    start = "asked from outside to place "
    _, said = asked(shell, start, "run", str(place["id"]))
    line = line_of(said, start)
    assert str(place["words"]) not in line, f"the walk said its row's name: {line}"
    shell.close_launcher()


def test_a_terminals_walk_says_its_rows_by_kind(shell: Shell) -> None:
    """A terminal's `launcher do` says the rows it took by kind, as a walk from outside does."""
    place = first_place(shell)
    start = "asked from a terminal to place "
    _, said = asked(shell, start, "doing", json.dumps({"steps": [place["words"]]}))
    line = line_of(said, start)
    assert str(place["words"]) not in line, f"the terminal's walk said its row's name: {line}"
    shell.close_launcher()


def test_a_terminals_failure_says_its_kind(shell: Shell) -> None:
    """The shell's own failure is said by its error's name, since its words may quote the ask."""
    start = "a terminal's do failed: "
    answer, said = asked(shell, start, "doing", f"not json {MARKER}")
    assert json.loads(answer) == {"failed": True}, answer
    line = line_of(said, start)
    assert MARKER not in said, f"the failure said the terminal's words: {line}"
    assert re.fullmatch(r"launcher: a terminal's do failed: [A-Z][A-Za-z]*", line), line
