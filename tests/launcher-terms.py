"""Check that each launcher row naming a glossary thing is called by its term.

    launcher-terms GLOSSARY WORDS

GLOSSARY is docs/glossary.md, WORDS the launcher's vocabulary
(modules/quickshell/launcher/words.json). A thing, a place or a command that
names a glossary thing says which with its term, "Context/word": the context
a `## ` section of the glossary, the word one of its things. A term naming no
thing in that context fails, as does one not written as Context/word. The
row's name, noun and what each say that word, in any case and singular or
plural, or the row fails, printed with what it says against its term. A
part, named short inside its thing, may say it after its thing's name, so a
battery's Health is battery health. A row
with no term names something the glossary doesn't, such as a program or a
network, and is left alone. And so does the check itself, should one of its
rules stop refusing what it's there for, or refuse what sits beside it.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

from glossary import Thing, plainly, read

# Fields a row calls its thing by.
CALLED = ("name", "noun", "what")


def entries(words: dict[str, list[dict[str, object]]]) -> list[dict[str, object]]:
    """Give every thing, place and command the vocabulary holds."""
    return [*words.get("things", []), *words.get("categories", []), *words.get("commands", [])]


def says(text: str, word: str) -> bool:
    """Say whether text is the word, in any case, singular or plural."""
    said = plainly(text)
    return said in (word, f"{word}s", f"{word}es")


def called(entry: dict[str, object], field: str, word: str, names: dict[str, str]) -> bool:
    """Say whether a row's field says the word, alone or, for a part, after its thing's name."""
    text = str(entry[field])
    owner = names.get(str(entry.get("in", "")))
    return says(text, word) or (owner is not None and says(f"{owner} {text}", word))


def wrong(
    entry: dict[str, object], terms: set[tuple[str, str]], names: dict[str, str]
) -> list[str]:
    """Say what's wrong with how one row names its glossary thing, if anything."""
    ident = str(entry.get("id", ""))
    term = entry.get("term")
    if term is None:
        return []
    if not isinstance(term, str) or term.count("/") != 1:
        return [f'{ident}: its term {term!r} isn\'t written as "Context/word".']
    context, word = (part.strip() for part in term.split("/"))
    word = plainly(word)
    if (context, word) not in terms:
        return [f'{ident}: its term "{term}" names no thing in the glossary\'s {context}.']
    return [
        f'{ident}: its {field} says "{entry[field]}", but its term is "{word}".'
        for field in CALLED
        if isinstance(entry.get(field), str) and not called(entry, field, word, names)
    ]


def drift(things: list[Thing], words: dict[str, list[dict[str, object]]]) -> list[str]:
    """Say each row whose term is wrong or whose name isn't its term."""
    terms = {(thing.context, thing.word) for thing in things}
    rows = entries(words)
    names = {
        str(entry["id"]): str(entry["name"])
        for entry in rows
        if isinstance(entry.get("id"), str) and isinstance(entry.get("name"), str)
    }
    return [said for entry in rows for said in wrong(entry, terms, names)]


GLOSSARY = (
    "## Agents\n\n| Word | Means | Not | In code |\n| --- | --- | --- | --- |\n"
    "| agent | a model at work | bot | |\n"
    "| This window | the window in focus | | |\n"
    "## Desktop\n\n| Word | Means | Not | In code |\n| --- | --- | --- | --- |\n"
    "| notice | a note on the screen | notification | |\n"
    "## Machine\n\n| Word | Means | Not | In code |\n| --- | --- | --- | --- |\n"
    "| battery | what the machine runs on unplugged | | |\n"
    "| battery health | how much a battery holds against new | | |\n"
)

# A stand-in vocabulary for each rule, breaking it once.
REFUSED: list[tuple[str, dict[str, list[dict[str, object]]]]] = [
    ("a term not written as Context/word", {"things": [{"id": "a", "term": "agent"}]}),
    ("a term in no context", {"things": [{"id": "a", "term": "Programs/agent"}]}),
    ("a term the context doesn't hold", {"things": [{"id": "a", "term": "Agents/notice"}]}),
    (
        "a name that isn't its term",
        {"things": [{"id": "a", "name": "Bots", "term": "Agents/agent"}]},
    ),
    (
        "a noun that isn't its term",
        {"categories": [{"id": "a", "noun": "ai", "term": "Agents/agent"}]},
    ),
    (
        "a what that isn't its term",
        {"commands": [{"id": "a", "what": "bot", "term": "Agents/agent"}]},
    ),
    (
        "a part short for another thing's term",
        {
            "things": [
                {"id": "battery", "name": "Battery", "term": "Machine/battery"},
                {"id": "agents", "name": "Agents"},
                {
                    "id": "agents:health",
                    "in": "agents",
                    "name": "Health",
                    "term": "Machine/battery health",
                },
            ]
        },
    ),
]

# What sits beside each rule's edge and isn't refused: a term's plural and
# its capital, a term of two words, a part named short inside its thing, and
# a row naming no glossary thing.
ALLOWED: dict[str, list[dict[str, object]]] = {
    "things": [
        {
            "id": "agents",
            "name": "Agents",
            "noun": "agents",
            "what": "agent",
            "term": "Agents/agent",
        },
        {"id": "notices", "name": "Notices", "term": "Desktop/notice"},
        {"id": "window", "name": "This window", "term": "Agents/This window"},
        {"id": "battery", "name": "Battery", "term": "Machine/battery"},
        {
            "id": "battery:health",
            "in": "battery",
            "name": "Health",
            "term": "Machine/battery health",
        },
        {"id": "firefox", "name": "Firefox"},
    ]
}


def refuses() -> list[str]:
    """Say each rule that lets through what it's there to refuse."""
    things, _ = read(GLOSSARY)
    out = [f"The check lets {why} through." for why, words in REFUSED if not drift(things, words)]
    out.extend(f"The check refuses what it shouldn't: {said}" for said in drift(things, ALLOWED))
    return out


def main(argv: list[str]) -> None:
    """Print every row not called by its term, and fail if there are any."""
    things, _ = read(Path(argv[0]).read_text())
    found = refuses() + drift(things, json.loads(Path(argv[1]).read_text()))
    for said in found:
        print(said)
    if found:
        sys.exit(f"{len(found)} launcher rows aren't called by their glossary term.")


if __name__ == "__main__":
    main(sys.argv[1:])
