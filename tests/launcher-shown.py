"""Check that what the launcher shows says the glossary's words, not the names it replaces.

    launcher-shown GLOSSARY WORDS

GLOSSARY is docs/glossary.md, WORDS the launcher's vocabulary
(modules/quickshell/launcher/words.json). Everything a row shows of a thing,
a place or a command is read: its name, noun, what, ask, empty, said, sure,
part and verb, each state's name, and a command's form and what it says. Any
that says a name the glossary replaces fails, printed with the word that
replaces it, matched as tests/names.py matches names, unless its thing is
listed below: as another sense of the word, or as a rename the launcher
hasn't made yet. A listed thing that no longer says one fails too, so the
lists only shrink. The words that only find a thing, never shown, are left
to the launcher's words step, but for the names a thing had (`once`): each
must be a name the glossary replaces, or its plural, so a rename keeps its
row and no new synonym slips in that way. And so does the check itself,
should one of its rules stop refusing what it's there for, or refuse what
sits beside it.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import TYPE_CHECKING

from glossary import Thing, plainly, read
from names import replaced

if TYPE_CHECKING:
    from collections.abc import Iterator

# Fields a row shows as they're written.
SHOWN = ("name", "noun", "what", "ask", "empty", "said", "sure", "part", "verb", "form", "says")

# Things whose word means something the glossary doesn't replace.
OTHER_SENSE = {
    "drive:drive": "Google Drive, as Google names its file store, not a disk",
    "drive:drives": "Google Drive's shared drives, as Google names them, not disks",
    "drive:mine": "Google Drive's My Drive, as Google names it, not a disk",
    "heat:display-engine": "the processor's part that drives the screens, as Intel names it",
    "machine:cpu:kept-by-display-engine": "Heat's Display Engine, as Intel names it",
    "power:limit": "a battery's charge limit, not an allowance",
    "shortcuts:neovim:editor:commands": "Neovim's command line, not a connection",
}

# Things waiting on a rename, each with the quest that owns it.
WAITING: dict[str, str] = {}


def shown(entry: dict[str, object]) -> Iterator[str]:
    """Give everything a row shows of a thing, a place or a command."""
    for field in SHOWN:
        value = entry.get(field)
        if isinstance(value, str):
            yield value
    # The sentences a drawer tells, by what each is for.
    tells = entry.get("tells")
    if isinstance(tells, dict):
        yield from (told for told in tells.values() if isinstance(told, str))
    states = entry.get("states")
    if isinstance(states, dict):
        for state in states.values():
            if isinstance(state, dict) and isinstance(state.get("name"), str):
                yield state["name"]


def entries(words: dict[str, list[dict[str, object]]]) -> list[dict[str, object]]:
    """Give every thing, place and command the vocabulary holds."""
    return [*words.get("things", []), *words.get("categories", []), *words.get("commands", [])]


def drift(things: list[Thing], words: dict[str, list[dict[str, object]]]) -> list[str]:
    """Say each shown word the glossary replaces, and each listed thing that no longer says one."""
    kept = set(OTHER_SENSE) | set(WAITING)
    out: list[str] = []
    saying: set[str] = set()
    for entry in entries(words):
        ident = str(entry.get("id", ""))
        said = [found for text in shown(entry) for found in replaced(text, things)]
        if said:
            saying.add(ident)
        if ident not in kept:
            out.extend(f"{ident}: {found}" for found in said)
    out.extend(
        f'"{ident}" is listed in tests/launcher-shown.py but says no replaced word: take it off.'
        for ident in sorted(kept - saying)
    )
    return out


GLOSSARY = (
    "## Machine\n\n| Word | Means | Not | In code |\n| --- | --- | --- | --- |\n"
    "| sleep | the machine resting | suspend | |\n"
    "| status line | the line under a session | status bar | |\n"
    "| connection | the link out | line | |\n"
)


def invented(things: list[Thing], words: dict[str, list[dict[str, object]]]) -> list[str]:
    """Say each name a thing keeps as one it had that the glossary replaces for nothing."""
    olds = {plainly(old) for thing in things for old in thing.not_}
    out: list[str] = []
    for entry in entries(words):
        kept = entry.get("once", [])
        for name in kept if isinstance(kept, list) else []:
            plain = plainly(str(name))
            if plain not in olds and plain.removesuffix("s") not in olds:
                out.append(
                    f'{entry.get("id")} keeps "{name}" as a name it had, but the glossary '
                    "replaces no such name, so it would be a synonym."
                )
    return out


def vocabulary(*things: dict[str, object]) -> dict[str, list[dict[str, object]]]:
    """Give a stand-in vocabulary holding things, beside one for every listed one."""
    listed: list[dict[str, object]] = [
        {"id": ident, "name": "Suspend"} for ident in sorted(set(OTHER_SENSE) | set(WAITING))
    ]
    return {"things": [*things, *listed]}


# A stand-in vocabulary for each rule, breaking it once.
REFUSED: list[tuple[str, dict[str, list[dict[str, object]]]]] = [
    ("a replaced name in a row's name", vocabulary({"id": "nap", "name": "Suspend"})),
    ("a replaced name in what an empty list says", vocabulary({"id": "nap", "empty": "No lines."})),
    (
        "a replaced name in a state's name",
        vocabulary({"id": "nap", "states": {"on": {"name": "Line"}}}),
    ),
    ("a replaced name in what a command says", {"commands": [{"id": "c", "says": "suspends"}]}),
    ("a listed thing no longer saying one", {"things": []}),
]

# What sits beside each rule's edge and isn't refused: a replaced name inside
# a term, and a replaced name only among the words that find a thing.
ALLOWED = vocabulary(
    {"id": "under", "name": "Status line"},
    {"id": "nap", "name": "Sleep", "words": ["suspend"], "hidden": ["line"]},
)


def refuses() -> list[str]:
    """Say each rule that lets through what it's there to refuse."""
    things, _ = read(GLOSSARY)
    out = [f"The check lets {why} through." for why, words in REFUSED if not drift(things, words)]
    out.extend(f"The check refuses what it shouldn't: {said}" for said in drift(things, ALLOWED))
    made_up: dict[str, list[dict[str, object]]] = {
        "things": [{"id": "nap", "name": "Sleep", "once": ["Snooze"]}]
    }
    if not invented(things, made_up):
        out.append("The check lets a name no thing had through as one it had.")
    kept: dict[str, list[dict[str, object]]] = {
        "things": [{"id": "nap", "name": "Sleep", "once": ["Suspend", "Lines"]}]
    }
    out.extend(f"The check refuses what it shouldn't: {said}" for said in invented(things, kept))
    return out


def main(argv: list[str]) -> None:
    """Print every shown word the glossary replaces, and fail if there are any."""
    things, _ = read(Path(argv[0]).read_text())
    words = json.loads(Path(argv[1]).read_text())
    found = refuses() + drift(things, words) + invented(things, words)
    for said in found:
        print(said)
    if found:
        sys.exit(f"The launcher shows {len(found)} words the glossary replaces.")


if __name__ == "__main__":
    main(sys.argv[1:])
