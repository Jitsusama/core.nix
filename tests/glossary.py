"""Check the glossary names each thing once in its context.

    glossary GLOSSARY

GLOSSARY is docs/glossary.md. Each `## ` section is a bounded context, and
each of its tables headed Word, Means, Not and In code holds things, a row
each: the thing's term, what it is, and the names it replaces, split by
commas. A term that names two things in one context fails, and each clash is
printed; the same term in two contexts is two things, each in its own. A
replaced name is never written for anything, so one that is any thing's term,
in any context, or that two things both replace, fails too, as does a thing's
own term among the names it replaces. So does a thing with no term or nothing
said of what it means, a table that starts with Word but has other columns, a
table before any context, and a glossary with no things at all. What code
spells is never read: In code may name anything. And so does the check
itself, should one of its rules stop refusing what it's there for, or refuse
what sits beside it.
"""

import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

COLUMNS = ["Word", "Means", "Not", "In code"]


@dataclass(frozen=True)
class Thing:
    """One row of the glossary: its context, its term, what it means, what it replaces."""

    context: str
    word: str
    means: str
    not_: tuple[str, ...]
    code: str = ""


def plainly(text: str) -> str:
    """Say a word as the glossary compares it: in lower case, its spaces single."""
    return " ".join(text.lower().split())


def cells(line: str) -> list[str]:
    """Give a table row's cells, without the bars at either end."""
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def words(cell: str) -> tuple[str, ...]:
    """Give the words a cell lists, split by commas, leaving out empty ones."""
    return tuple(plainly(word) for word in cell.split(",") if word.strip())


def heading(row: list[str], number: int, context: str) -> str | None:
    """Say what's wrong with a table of words' heading row, if anything."""
    if row != COLUMNS:
        return f"Line {number}: a table of words has columns {row}, not {COLUMNS}."
    if not context:
        return f"Line {number}: a table of words sits in no context."
    return None


def thing(row: list[str], number: int, context: str) -> Thing | str:
    """Give the thing a row of a table of words holds, or what's wrong with it."""
    if len(row) != len(COLUMNS):
        return f"Line {number}: a row has {len(row)} cells, not {len(COLUMNS)}."
    word, means, not_, code = row
    if not word or not means:
        return f"Line {number}: a thing needs a name and what it means: {row}."
    return Thing(context, plainly(word), means, words(not_), code)


def read(text: str) -> tuple[list[Thing], list[str]]:
    """Give every thing the glossary's tables hold, and what's wrong with their shape."""
    things: list[Thing] = []
    wrong: list[str] = []
    context = ""
    in_table = False
    for number, line in enumerate(text.splitlines(), start=1):
        if line.startswith("## "):
            context = line[3:].strip()
        row = cells(line) if line.lstrip().startswith("|") else []
        if not row:
            in_table = False
        elif row[0] == "Word":
            said = heading(row, number, context)
            in_table = said is None
            wrong.extend([said] if said else [])
        elif in_table and not set("".join(row)) <= set("-: "):
            found = thing(row, number, context)
            if isinstance(found, Thing):
                things.append(found)
            else:
                wrong.append(found)
    if not things and not wrong:
        wrong.append("The glossary names no things.")
    return things, wrong


def clashes(things: list[Thing]) -> list[str]:
    """Say each term that names two things in a context, and each replaced name written."""
    # Each row is a thing of its own, so a term written twice in a context is two things.
    named: dict[tuple[str, str], int] = defaultdict(int)
    terms: dict[str, list[str]] = defaultdict(list)
    replaced: dict[str, list[str]] = defaultdict(list)
    out: list[str] = []
    for thing in things:
        named[(thing.context, thing.word)] += 1
        terms[thing.word].append(thing.context)
        for word in set(thing.not_):
            replaced[word].append(thing.word)
        if thing.word in thing.not_:
            out.append(f'"{thing.word}" replaces its own name.')
    out.extend(
        f'"{word}" names {count} things in {context}.'
        for (context, word), count in sorted(named.items())
        if count > 1
    )
    out.extend(
        f'"{word}" is replaced by {len(owners)} things: {", ".join(owners)}.'
        for word, owners in sorted(replaced.items())
        if len(owners) > 1
    )
    out.extend(
        f'"{word}" is replaced by {", ".join(replaced[word])} but is a term in '
        f"{', '.join(terms[word])}."
        for word in sorted(set(replaced) & set(terms))
    )
    return out


def settle(text: str) -> list[str]:
    """Say everything in a glossary to settle."""
    things, wrong = read(text)
    return wrong + clashes(things)


HEADER = "| Word | Means | Not | In code |\n| --- | --- | --- | --- |\n"
DISK = "## Disk\n\n" + HEADER
AGENTS = "\n## Agents\n\n" + HEADER

# A stand-in glossary for each rule, breaking it once.
REFUSED: list[tuple[str, str]] = [
    ("a term twice in a context", DISK + "| disk | a drive | | |\n| Disk | a store | | |\n"),
    (
        "a replaced name that is a term in another context",
        DISK + "| disk | a store | drive | |\n" + AGENTS + "| drive | a push | | |\n",
    ),
    (
        "a name two things replace",
        DISK + "| machine | one | host | |\n" + AGENTS + "| server | two | host | |\n",
    ),
    ("its own name replaced", DISK + "| notice | a card | notice | |\n"),
    ("a thing with no meaning", DISK + "| notice | | | |\n"),
    ("a row short of cells", DISK + "| notice | a card |\n"),
    (
        "a table of other columns",
        "## Disk\n\n| Word | Means |\n| --- | --- |\n| notice | a card |\n",
    ),
    ("a table in no context", HEADER + "| notice | a card | | |\n"),
    ("no things at all", "# Glossary\n\nNothing yet.\n"),
]

# What sits beside each rule's edge and isn't refused: one term in two
# contexts, a name inside another's, the same word in code, and a table
# that isn't one of words.
ALLOWED = (
    DISK
    + "| snapshot | a filesystem's copy | | `disk` |\n"
    + "| disk | a store | drive | `disk` |\n"
    + "| disk guard | keeps the disk from filling | | `disk-guard`, `disk` |\n"
    + AGENTS
    + "| snapshot | a tree pinned at a commit | | |\n"
    + "\n| Key | Does |\n| --- | --- |\n| Tab | completes |\n"
)


def refuses() -> list[str]:
    """Say each rule that lets through what it's there to refuse."""
    out = [f"The check lets {why} through." for why, text in REFUSED if not settle(text)]
    out.extend(f"The check refuses what it shouldn't: {said}" for said in settle(ALLOWED))
    return out


def main(argv: list[str]) -> None:
    """Print every word to settle, and fail if there are any."""
    found = refuses() + settle(Path(argv[0]).read_text())
    for said in found:
        print(said)
    if found:
        sys.exit(f"The glossary has {len(found)} words to settle.")


if __name__ == "__main__":
    main(sys.argv[1:])
