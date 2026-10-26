"""Check that a repository's names say the glossary's words, not the names it replaces.

    names GLOSSARY NAMES KEPT

GLOSSARY is docs/glossary.md, read as glossary.py reads it. NAMES is a JSON
list of names: every check's, and every option's. KEPT is a JSON list of the
names the repository keeps, each another sense of its word or a rename its
owner hasn't made yet, as tests/names.nix is given them. A name is split into
words at hyphens, dots and capitals, and a name it says that the glossary
replaces fails, printed with the word that replaces it, unless one of the
glossary's terms holds it there ("line" inside "status line"), the row that
replaces it spells that word on its own as its code's (`pressure` for filling),
or the name is kept. Another row's code doesn't count: the `backlight` program
is the screen's brightness, so a name saying backlight for the keyboard's light
still fails, and the program's own names are kept as another sense. An option
is kept by its path or a part of it. A kept name
the check no longer finds fails too, so what's kept only shrinks.
And so does the check itself, should one of its rules stop refusing what it's
there for, or refuse what sits beside it.
"""

import json
import re
import sys
from pathlib import Path

from glossary import Thing, read


def tokens(text: str) -> list[str]:
    """Split a name or a term into its words, in lower case."""
    spaced = re.sub(r"([a-z0-9])([A-Z])", r"\1 \2", text)
    return [word for word in re.split(r"[^A-Za-z0-9']+", spaced.lower()) if word]


def same(words: list[str], old: list[str]) -> bool:
    """Say whether words say old, a plural counting as its word."""
    return len(words) == len(old) and all(
        word in (was, was + "s", was + "es") for word, was in zip(words, old, strict=True)
    )


def held(words: list[str], start: int, length: int, terms: list[list[str]]) -> bool:
    """Say whether a longer term covers the words from start, as status line covers line."""
    for term in terms:
        if len(term) <= length:
            continue
        first = max(0, start + length - len(term))
        for at in range(first, start + 1):
            if same(words[at : at + len(term)], term):
                return True
    return False


def spelled(things: list[Thing]) -> list[list[str]]:
    """Give the words of each span the glossary's In code puts in backticks."""
    return [tokens(span) for thing in things for span in re.findall(r"`([^`]+)`", thing.code)]


def listed(name: str, names: set[str]) -> bool:
    """Say whether a name is among names, or an option below one of them."""
    return any(name == entry or name.startswith(entry + ".") for entry in names)


def replaced(name: str, things: list[Thing]) -> list[str]:
    """Say each name the glossary replaces that a name says, with the word that replaces it."""
    words = tokens(name)
    terms = [tokens(thing.word) for thing in things]
    out: list[str] = []
    for thing in things:
        code = spelled([thing])
        for old in thing.not_:
            old_words = tokens(old)
            if any(same(span, old_words) for span in code):
                continue
            for start in range(len(words) - len(old_words) + 1):
                if not same(words[start : start + len(old_words)], old_words):
                    continue
                if not held(words, start, len(old_words), terms):
                    out.append(f'"{name}" says "{old}", which the glossary calls "{thing.word}".')
    return out


def drift(things: list[Thing], names: list[str], kept: set[str]) -> list[str]:
    """Say each name that says a replaced name, and each kept name no longer found."""
    out: list[str] = []
    for name in sorted(set(names)):
        if not listed(name, kept):
            out.extend(replaced(name, things))
    out.extend(
        f'"{entry}" is kept but no longer found: take it off.'
        for entry in sorted(kept)
        if not any(listed(name, {entry}) for name in names)
    )
    return out


GLOSSARY = (
    "## Network\n\n| Word | Means | Not | In code |\n| --- | --- | --- | --- |\n"
    "| connection | the link out | line | `jitsusama.wan.lineRate` |\n"
    "| status line | the line under a session | status bar | |\n"
    "| person | the human | user | |\n"
    "| light | the keyboard's light | backlight | |\n"
    "| brightness | how bright a screen is | | the `backlight` program |\n"
    "| filling | a disk filling up | pressure | `pressure` |\n"
)

# Stand-in kept names, one another sense and one waiting rename, and a
# stand-in name for each rule, breaking it once, beside the kept names so
# that what's kept isn't what fails.
KEPT = {"backlight-passes-clippy", "jitsusama.theme.syntax.operator", "vm-the-line-stays-quick"}
LISTED = sorted(KEPT)
REFUSED: list[tuple[str, list[str]]] = [
    ("a replaced name in a check's name", ["the-line-stays-quick", *LISTED]),
    ("a replaced name made plural", ["the-users-sign-in", *LISTED]),
    ("a replaced name in an option's name", ["jitsusama.shaper.userRate", *LISTED]),
    ("a replaced name in a name the glossary spells whole", ["jitsusama.wan.lineRate", *LISTED]),
    ("a replaced name another row spells as code", ["the-keyboards-backlight-dims", *LISTED]),
    ("a kept name no longer found", LISTED[1:]),
]

# What sits beside each rule's edge and isn't refused: a replaced name inside
# a term, a word the row replacing it spells on its own in code, a word inside
# another, and an option below a kept one.
ALLOWED = [
    "the-status-line-says-its-quest",
    "the-pressure-band-holds",
    "userspace-keeps-its-homes",
    "jitsusama.theme.syntax.operator.bold",
    *LISTED,
]


def refuses() -> list[str]:
    """Say each rule that lets through what it's there to refuse."""
    things, _ = read(GLOSSARY)
    out = [
        f"The check lets {why} through." for why, names in REFUSED if not drift(things, names, KEPT)
    ]
    out.extend(
        f"The check refuses what it shouldn't: {said}" for said in drift(things, ALLOWED, KEPT)
    )
    return out


def main(argv: list[str]) -> None:
    """Print every name that says a replaced name, and fail if there are any."""
    things, _ = read(Path(argv[0]).read_text())
    names = json.loads(Path(argv[1]).read_text())
    kept = set(json.loads(Path(argv[2]).read_text()))
    found = refuses() + drift(things, names, kept)
    for said in found:
        print(said)
    if found:
        sys.exit(f"{len(found)} names say what the glossary replaces.")


if __name__ == "__main__":
    main(sys.argv[1:])
