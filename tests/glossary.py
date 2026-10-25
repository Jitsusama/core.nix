"""Check the glossary names each thing once.

    glossary GLOSSARY

GLOSSARY is docs/glossary.md. Each of its tables headed Word, Means, Also,
Not and In code holds things, a row each: the thing's name, what it is, the
other words that find it and the names it replaces, each list split by
commas. A word that is the Word, an Also or a Not of two things fails, and
each clash is printed, as does a thing's own name among the names it
replaces or the words that find it. So does a thing with no name or nothing
said of what it means, a table that starts with Word but has other columns,
and a glossary with no things at all. What code spells is never read: In
code may name anything. And so does the check itself, should one of its
rules stop refusing what it's there for, or refuse what sits beside it.
"""

import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

COLUMNS = ["Word", "Means", "Also", "Not", "In code"]


@dataclass(frozen=True)
class Thing:
    """One row of the glossary: a name, what it means, and its other words."""

    word: str
    means: str
    also: tuple[str, ...]
    not_: tuple[str, ...]


def plainly(text: str) -> str:
    """Say a word as the glossary compares it: in lower case, its spaces single."""
    return " ".join(text.lower().split())


def cells(line: str) -> list[str]:
    """Give a table row's cells, without the bars at either end."""
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def words(cell: str) -> tuple[str, ...]:
    """Give the words a cell lists, split by commas, leaving out empty ones."""
    return tuple(plainly(word) for word in cell.split(",") if word.strip())


def read(text: str) -> tuple[list[Thing], list[str]]:
    """Give every thing the glossary's tables hold, and what's wrong with their shape."""
    things: list[Thing] = []
    wrong: list[str] = []
    in_table = False
    for number, line in enumerate(text.splitlines(), start=1):
        if not line.lstrip().startswith("|"):
            in_table = False
            continue
        row = cells(line)
        if set("".join(row)) <= set("-: "):
            continue
        if row and row[0] == "Word":
            in_table = row == COLUMNS
            if not in_table:
                wrong.append(f"Line {number}: a table of words has columns {row}, not {COLUMNS}.")
            continue
        if not in_table:
            continue
        if len(row) != len(COLUMNS):
            wrong.append(f"Line {number}: a row has {len(row)} cells, not {len(COLUMNS)}.")
            continue
        word, means, also, not_, _ = row
        if not word or not means:
            wrong.append(f"Line {number}: a thing needs a name and what it means: {row}.")
            continue
        things.append(Thing(plainly(word), means, words(also), words(not_)))
    if not things and not wrong:
        wrong.append("The glossary names no things.")
    return things, wrong


def clashes(things: list[Thing]) -> list[str]:
    """Say each word that names two things, or a thing's own name it also replaces."""
    # Each row is a thing of its own, so a name written twice is two things.
    named: dict[str, list[str]] = defaultdict(list)
    out: list[str] = []
    for thing in things:
        for word in {thing.word, *thing.also, *thing.not_}:
            named[word].append(thing.word)
        if thing.word in thing.also or thing.word in thing.not_:
            out.append(f'"{thing.word}" is its own other word or a name it replaces.')
        both = sorted(set(thing.also) & set(thing.not_))
        if both:
            out.append(f'"{thing.word}" is both found by and replaces {", ".join(both)}.')
    out.extend(
        f'"{word}" names {len(owners)} things: {", ".join(owners)}.'
        for word, owners in sorted(named.items())
        if len(owners) > 1
    )
    return out


def settle(text: str) -> list[str]:
    """Say everything in a glossary to settle."""
    things, wrong = read(text)
    return wrong + clashes(things)


HEADER = "| Word | Means | Also | Not | In code |\n| --- | --- | --- | --- | --- |\n"

# A stand-in glossary for each rule, breaking it once.
REFUSED: list[tuple[str, str]] = [
    ("a name twice", HEADER + "| disk | a drive | | | |\n| Disk | a store | | | |\n"),
    (
        "another's name as a word",
        HEADER + "| disk | a store | drive | | |\n| drive | a car | | | |\n",
    ),
    (
        "another's word as a replaced name",
        HEADER + "| machine | one | | host | |\n| server | two | host | | |\n",
    ),
    ("its own name replaced", HEADER + "| notice | a card | | notice | |\n"),
    ("a word both found and replaced", HEADER + "| notice | a card | toast | toast | |\n"),
    ("a thing with no meaning", HEADER + "| notice | | | | |\n"),
    ("a row short of cells", HEADER + "| notice | a card |\n"),
    ("a table of other columns", "| Word | Means |\n| --- | --- |\n| notice | a card |\n"),
    ("no things at all", "# Glossary\n\nNothing yet.\n"),
]

# What sits beside each rule's edge and isn't refused: a name inside
# another's, the same word in code, and a table that isn't one of words.
ALLOWED = (
    HEADER
    + "| disk | a store | drive | | `disk` |\n"
    + "| disk guard | keeps the disk from filling | | | `disk-guard`, `disk` |\n"
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
