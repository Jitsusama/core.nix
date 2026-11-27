"""What the launcher's journal says of an ask from outside while reports are off, on niri.

modules/quickshell/checks/core.nix runs this through pytest inside tests/lib/nested-niri.sh, with
the shell's reports off. A link, a terminal's `launcher do` or a catalogue run hands the launcher
words someone else wrote, a page's or a source's, so with reports off each line says them by kind
and how many letters, as the journal's other lines do (launcher/ActsSaid.js), and never whole.
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
    from launcher_on_niri import Json, Shell

# Words no row has, each one the line would say whole while reports were on.
MARKER = "HARBOURMARK"
# How a line says the marker with reports off.
COUNTED = f"{len(MARKER)} letters"
# How long a line has to reach the shell's log after its call answers, in seconds.
WRITTEN_WITHIN = 5.0
# How long what a link opens drops every key (Prompt.qml's linkQuiet), and a little more.
LINK_QUIET = 0.8
# The launcher's own kinds of key, said whole where an id follows (KeysSaid.js's keysOwn and
# ownId), and how many letters of such a key a line says.
KEYS_OWN = ("place:", "category:", "collection:")
OWN_ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]*")
OWN_LENGTH = 64


# The shell, started once for the file's cases.
shell = launcher_on_niri.shell


def asked(shell: Shell, start: str, *words: str) -> tuple[str, str]:
    """Call the launcher's IPC with these words, and give its answer and the log it left.

    The log is read until it holds the line that starts so, since the shell's log may reach the
    file a moment after the call answers, or the time to wait is up.
    """
    mark = shell.mark()
    answer = shell.as_account(shell.ipc(*(shlex.quote(word) for word in words)))
    deadline = time.monotonic() + WRITTEN_WITHIN
    said = launcher_on_niri.logged_since(shell, mark)
    while f"launcher: {start}" not in said and time.monotonic() < deadline:
        time.sleep(0.1)
        said = launcher_on_niri.logged_since(shell, mark)
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
    assert missing not in answer, f"this shell has no place for {call}, so nothing's counted"
    assert line_of(said, start).endswith(COUNTED), said
    close_after_link(shell)


def held_whole(key: str) -> bool:
    """Whether a line says this key whole: an own kind's, holding an id after its colon."""
    kind, _, rest = key.partition(":")
    return f"{kind}:" in KEYS_OWN and OWN_ID.fullmatch(rest) is not None


def first_place(shell: Shell) -> Json:
    """Give a place at the launcher's top keyed by an own kind and an id its name isn't in.

    Only such a key is said whole, so the walk's line can be held to exactly what it says;
    every other key's line, a source's said by its kind alone, is pinned in tests/pacing.qml.
    Such a key is the same filtered or not, so a walk to it proves the line follows the reports
    switch, not that the launcher's two walk lines go through keyShown at all.
    """
    listed = cast("list[Json]", json.loads(shell.as_account(shell.ipc("list"))))
    places = [
        one
        for one in listed
        if one["kind"] == "place"
        and one["in"] == ""
        and held_whole(str(one["id"]))
        and str(one["words"]) not in str(one["id"])
    ]
    tops = [
        (one["id"], one["words"]) for one in listed if one["kind"] == "place" and one["in"] == ""
    ]
    assert places, f"no top place keyed by an own kind and an id its name isn't in: {tops}"
    return places[0]


def assert_walk_said_by_key(line: str, start: str, place: Json) -> None:
    """Check a one-row walk's line says its place by its key, and not its name.

    The place is keyed by an own kind and an id, as `category:<id>` (first_place), said whole
    to its 64th letter, so the line says exactly that much of it, never only its start.
    """
    step = line.removeprefix(f"launcher: {start}")
    assert str(place["words"]) not in line, f"the walk said its row's name: {line}"
    assert step, f"the walk said no row: {line}"
    assert "\u203a" not in step, f"a walk of one row said more than one: {line}"
    wanted = str(place["id"])[:OWN_LENGTH]
    assert step == wanted, f"the walk said another row than its own: {line}"


def test_a_catalogue_walk_says_its_row_by_key(shell: Shell) -> None:
    """A walk from outside says its place by its key rather than its name, which a source gave."""
    place = first_place(shell)
    start = "asked from outside to place "
    _, said = asked(shell, start, "run", str(place["id"]))
    assert_walk_said_by_key(line_of(said, start), start, place)
    shell.close_launcher()


def test_a_terminals_walk_says_its_row_by_key(shell: Shell) -> None:
    """A terminal's `launcher do` says the place it took by key, as a walk from outside does."""
    place = first_place(shell)
    start = "asked from a terminal to place "
    _, said = asked(shell, start, "doing", json.dumps({"steps": [place["words"]]}))
    assert_walk_said_by_key(line_of(said, start), start, place)
    shell.close_launcher()


def test_a_terminals_failure_says_its_kind(shell: Shell) -> None:
    """The shell's own failure is said by its error's name, since its words may quote the ask."""
    start = "a terminal's do failed: "
    answer, said = asked(shell, start, "doing", f"not json {MARKER}")
    assert json.loads(answer) == {"failed": True}, answer
    line = line_of(said, start)
    assert MARKER not in said, f"the failure said the terminal's words: {line}"
    assert re.fullmatch(r"launcher: a terminal's do failed: [A-Z][A-Za-z]*", line), line
