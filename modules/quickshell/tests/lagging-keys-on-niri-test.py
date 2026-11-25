"""What the launcher's journal says of a lagging key while reports are off, on niri.

modules/quickshell/checks/core.nix runs this through pytest inside tests/lib/nested-niri.sh, with
the shell's reports off. A key slower to its frame than launcher/Trials.js's FELT_MS is said in the
journal by what it was (Presented.timedShown): words typed by how many letters, a step by its
arrow and the letters of the row it named, and a key the launcher names itself, an arrow, whole,
since nobody wrote it. Each of Launcher.qml's callers of keyTimed says which its key is, which
no case in Qt's engine reaches, so these press each kind and read its line. A key listed
without a daemon's answer, and the answer that comes late, count its letters the same way, and
a lagging program's drawer is said by its desktop entry's id (Presented.rowShown), never the
name its window shows, so each program's drawer is timed apart.
"""

from __future__ import annotations

import re
import time
from typing import TYPE_CHECKING

import launcher_on_niri

if TYPE_CHECKING:
    from collections.abc import Callable

    from launcher_on_niri import Json, Shell

# How many times a kind of key is pressed for one of them to be slow enough to be said: the
# nested niri's software frames take longer than FELT_MS for nearly every key.
TRIES = 6
# How long a key's line has to reach the shell's log after its frame, in seconds.
WRITTEN_WITHIN = 2.0
# A lagging key's line, holding what it said of the key.
TOOK = re.compile(r"launcher: (.+?) took \d+ ms to its frame")
# An arrow as its line says it whole, quoted as Presented.typedShown quotes.
ARROWS = {'"\u2193"', '"\u2191"'}
# A step as its line says it with reports off: its arrow and the letters of the row it named.
STEP = re.compile(r"[\u2192\u2190] \d+ letters?")
# How a step in and a step out begin.
STEPS = {"\u2192 ", "\u2190 "}
# What a key listed without an answer, and the answer late, say of the key's words.
LISTED = re.compile(r"launcher: (.+?) listed without ")
LATE = re.compile(r"launcher: .+? answered (.+?) \d+ ms after its key")
# Words typed with nothing else named by them, so no line says them but by the letters.
UNNAMED = "zygodactyl"
# What a program's drawer line says of its row, keyed `program:<its desktop entry's id>`: the id,
# shaped as an app's (KeysSaid.js's appWord), so RTOIMQ can time Chrome's drawer apart from
# kitty's (2026-11-24), and never more of the key.
PROGRAMS_DRAWER = re.compile(r'drawer for "program:([A-Za-z0-9][A-Za-z0-9.-]*)"')
# Any program's drawer line, shaped as PROGRAMS_DRAWER or said by its kind alone, but not one of
# another kind that begins with program's word.
A_PROGRAMS_DRAWER = re.compile(r'drawer for "program[:"]')

# The shell, started once for the file's cases.
shell = launcher_on_niri.shell


def keys_said(shell: Shell, mark: int) -> list[str]:
    """Give what each lagging key's line since the mark said of its key, oldest first."""
    return [found.group(1) for found in TOOK.finditer(launcher_on_niri.logged_since(shell, mark))]


def said_by(
    shell: Shell, press: Callable[[], None], enough: Callable[[list[str]], bool]
) -> list[str]:
    """Press as `press` does, TRIES times at most, until the keys said are `enough`.

    Give every key said since the first press, which a case reads whole, so a key said wrongly
    fails it whichever try said it.
    """
    mark = shell.mark()
    said: list[str] = []
    for _ in range(TRIES):
        press()
        deadline = time.monotonic() + WRITTEN_WITHIN
        while time.monotonic() < deadline:
            said = keys_said(shell, mark)
            if enough(said):
                return said
            time.sleep(launcher_on_niri.POLL)
    return said


def test_an_arrow_is_said_whole(shell: Shell) -> None:
    """An arrow is said as itself, since the launcher names it and nobody typed it."""
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()

    def down_and_up() -> None:
        shell.send_key("down")
        shell.send_key("up")

    said = said_by(shell, down_and_up, lambda keys: bool(ARROWS & set(keys)))
    shell.close_launcher()
    assert ARROWS & set(said), f"no arrow was said as itself: {said}"
    assert "1 letter" not in said, f"an arrow was counted as typed: {said}"


def test_a_step_in_and_out_says_its_arrow_and_counts_the_row(shell: Shell) -> None:
    """Return into a row and - out of it say their arrow and count the row's name.

    Either said as typed would count the arrow among the letters, and said whole would say a
    row's name, which a source may have written.
    """

    def in_and_out() -> None:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.type_in("programs", selecting="Programs")
        shell.step_in()
        shell.normal()
        shell.send_chars("-")
        shell.launcher_until(lambda shown: not shown["held"])
        shell.close_launcher()

    said = said_by(shell, in_and_out, lambda keys: {key[:2] for key in keys} >= STEPS)
    steps = [key for key in said if key[:2] in STEPS]
    assert {key[:2] for key in steps} == STEPS, f"a step wasn't said: {said}"
    assert all(STEP.fullmatch(key) for key in steps), f"a step said more than letters: {said}"
    assert not any("rograms" in key for key in said), f"a key said the row's name: {said}"


def test_a_word_and_a_space_says_the_step_it_took(shell: Shell) -> None:
    """A word and a space that step into a place say the step, its row counted, as Return's does.

    The space's key is timed as the step it took rather than as the letters typed.
    """

    def stepped_in() -> None:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.send_chars("network ")
        shell.launcher_until(lambda shown: shown["path"][-1:] == ["Network"])
        shell.close_launcher()

    said = said_by(shell, stepped_in, lambda keys: any(key[:2] == "\u2192 " for key in keys))
    steps = [key for key in said if key.startswith("\u2192 ")]
    assert steps, f"the step a space took wasn't said: {said}"
    assert all(STEP.fullmatch(key) for key in steps), f"the step said more than letters: {said}"
    assert not any("etwork" in key for key in said), f"a key said the row's name: {said}"


def logged_until(shell: Shell, mark: int, wanted: re.Pattern[str]) -> list[str]:
    """Give what each line since the mark matching `wanted` says, waiting until one does."""
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    while time.monotonic() < deadline:
        logged = launcher_on_niri.logged_since(shell, mark)
        said = [found.group(1) for found in wanted.finditer(logged)]
        if said:
            return said
        time.sleep(launcher_on_niri.POLL)
    failed = f"no line said {wanted.pattern}:\n{shell.tail()}"
    raise AssertionError(failed)


def test_a_key_listed_without_an_answer_and_the_late_answer_count_its_letters(
    shell: Shell,
) -> None:
    """A key listed without the notices' answer, and their answer late, count what was typed.

    Said as typed, either would write the words, which may be a name or a password's.
    """
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    mark = shell.mark()
    with launcher_on_niri.notices_stopped():
        shell.type_in(UNNAMED)
        listed = logged_until(shell, mark, LISTED)
    late = logged_until(shell, mark, LATE)
    shell.close_launcher()
    keys = listed + late
    assert all(re.fullmatch(r"\d+ letters?", key) for key in keys), f"words were said: {keys}"


def program_timed_since(shown: Json, before: int) -> bool:
    """Say whether a program's drawer is the one timed since the launcher had timed `before`."""
    timed = shown["drawer"]["timed"]
    return timed["count"] > before and str(timed["key"]).startswith("program:")


def test_a_programs_lagging_drawer_is_said_by_its_id(shell: Shell) -> None:
    """A program's drawer lagging to its frame is said by its desktop entry's id, and no more.

    Launcher.qml's drawerMade hands Trials.drawerTimed the row as Presented.rowShown says it,
    which keeps a program's id as it keeps a window's app, so each program's drawer is timed
    apart, and drops any part past it; said by its kind alone, the line names the program the
    launcher timed nowhere.
    """
    keyed: set[str] = set()

    def drawn() -> None:
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        before = shell.showing()["drawer"]["timed"]["count"]
        shell.type_in("programs", selecting="Programs")
        shell.step_in()
        shell.launcher_until(lambda shown: program_timed_since(shown, before))
        keyed.add(str(shell.showing()["drawer"]["timed"]["key"]))
        shell.close_launcher()

    said = said_by(shell, drawn, lambda keys: any(PROGRAMS_DRAWER.fullmatch(key) for key in keys))
    programs = [key for key in said if A_PROGRAMS_DRAWER.match(key)]
    assert programs, f"no program's drawer was said: {said}"
    unshaped = [key for key in programs if not PROGRAMS_DRAWER.fullmatch(key)]
    assert not unshaped, f"a program's drawer was said without its id, or with more: {unshaped}"
    timed = {f'drawer for "{key}"' for key in keyed}
    assert timed & set(programs), f"no drawer said the program timed, {sorted(keyed)}: {programs}"
