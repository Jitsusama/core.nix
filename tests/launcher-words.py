"""Check the launcher's vocabulary says each word of one thing only.

    launcher-words WORDS GLYPHS

WORDS is the vocabulary (modules/quickshell/launcher/words.json), GLYPHS a
JSON list of the theme's glyph names. A shown name, another word or an
answer's question that finds two different things at the same strength, in
the same place, fails, unless the vocabulary lists it as meant to, and each
clash is printed; an answer is asked from anywhere, so its questions clash
wherever the thing is. So does a word listed as meant that finds only one, a
word listed as asked by its start that doesn't ask it whole, a category's
word that names a thing it doesn't hold, a name for something the
vocabulary doesn't know, a glyph the theme hasn't got, a thing with no
place: not in a category, nor a part of one, nor found inside another, and
a name or an empty list's words in another case than the rest: a
category's or a thing's name starts with a capital, as every row's does, and
what an empty list says is a sentence. So does anything shown that Joel
wouldn't say: a program's or the system's own name (niri, PipeWire, the
kernel), a name that reads its value first ("10 bit" rather than "Colour
depth"), a unit written another way than the launcher's ("10-bit", "KB",
"Mbit/s", "1000 a second" for "1000 Hz"), a thing's words said beside it
in capitals, or a verb longer than a word but for a turn ("turn off").
"""

import json
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

Entry = dict[str, Any]

# How strongly each kind of word finds its thing: a shown name typed whole,
# another word or an old name, a question an answer is asked by, or a
# category's own word. Only words at the same strength can clash.
NAME = "name"
WORD = "word"
ANSWER = "answer"
CATEGORY = "category"


def plainly(text: str) -> str:
    """Say a word as the launcher reads it: in lower case, its spaces single."""
    return " ".join(text.lower().split())


def spoken(entry: Entry) -> list[tuple[str, str]]:
    """Give every string a thing is found by, in any state, with its strength."""
    states = [entry, *entry.get("states", {}).values()]
    said: list[tuple[str, str]] = []
    for state in states:
        if "name" in state:
            said.append((NAME, state["name"]))
        said.extend((WORD, word) for word in state.get("hidden", []))
    # A collection's noun finds it as its words do (Launcher.qml's named).
    nouns = [entry["noun"]] if "noun" in entry else []
    said.extend((WORD, word) for word in [*entry.get("words", []), *nouns])
    said.extend((ANSWER, word) for word in entry.get("answers", []))
    return [(strength, plainly(word)) for strength, word in said]


def clashes(vocabulary: Entry) -> list[str]:
    """Say each word that finds two things at once, and each meant that needn't be."""
    finds: dict[tuple[str, str, str], set[str]] = defaultdict(set)
    for entry in vocabulary["things"]:
        for strength, word in spoken(entry):
            # A question asks its answer from the top, wherever the thing is.
            scope = "" if strength == ANSWER else entry.get("in", "")
            finds[scope, strength, word].add(entry["id"])
    for category in vocabulary["categories"]:
        for word in [category["id"], *category.get("words", [])]:
            finds["", CATEGORY, plainly(word)].add(f"category:{category['id']}")
    for command in vocabulary["commands"]:
        name = plainly(f"{command['start']}{command['form']}")
        finds["commands", NAME, name].add(command["id"])
        for word in command.get("words", []):
            finds["commands", WORD, plainly(word)].add(command["id"])
    meant = {plainly(m["word"]): set(m["things"]) for m in vocabulary["meant"]}
    used: set[str] = set()
    out: list[str] = []
    for (scope, strength, word), ids in sorted(finds.items()):
        if len(ids) < 2:  # noqa: PLR2004 (a clash is two things or more)
            continue
        where = f" inside {scope}" if scope else ""
        if word in meant and ids <= meant[word]:
            used.add(word)
            continue
        out.append(f'"{word}" finds {", ".join(sorted(ids))} as a {strength}{where}.')
    out.extend(
        f'"{word}" is listed as meant to find several things, but finds one.'
        for word in sorted(set(meant) - used)
    )
    return out


def stray(category: Entry, things: dict[str, Entry]) -> list[str]:
    """Say what a category holds that isn't named, and words of its that others say."""
    holds = category.get("holds", [])
    out = [f"The {category['id']} category holds {held}, which isn't named." for held in holds
           if held not in things]  # fmt: skip
    # A word of one it holds would list the category beside it.
    said = {word: held for held in holds if held in things for _, word in spoken(things[held])}
    out.extend(
        f"The {category['id']} category's word {word} finds {said[plainly(word)]} already."
        for word in category.get("words", [])
        if plainly(word) in said
    )
    shown = {plainly(things[held].get("name", "")) for held in holds if held in things}
    out.extend(
        f"The {category['id']} category is named as {entry['id']} is."
        for entry in things.values()
        if plainly(entry.get("name", "")) == category["id"]
        and category["id"] not in shown
        and "in" not in entry
    )
    return out


def misplaced(vocabulary: Entry, glyphs: set[str]) -> list[str]:
    """Say what names an unknown thing or glyph, and what has no place."""
    things = {entry["id"]: entry for entry in vocabulary["things"]}
    out: list[str] = []
    placed: set[str] = set()
    for category in vocabulary["categories"]:
        placed.update(category.get("holds", []))
        out.extend(stray(category, things))
    for entry in things.values():
        for part in entry.get("parts", []):
            placed.add(part)
            if part not in things:
                out.append(f"{entry['id']} has {part} as a part, which isn't named.")
        states = [entry, *entry.get("states", {}).values()]
        out.extend(
            f"{entry['id']} wears {state['glyph']}, a glyph the theme hasn't got."
            for state in states
            if "glyph" in state and state["glyph"] not in glyphs
        )
    for entry in things.values():
        homed = {"in", "of", "by"} & entry.keys()
        if entry["id"] not in placed and not homed:
            out.append(f"{entry['id']} has no place: no category holds it, nor anything else.")
        if entry.get("by") and entry["by"] not in placed:
            out.append(f"{entry['id']} is given by {entry['by']}, which has no place.")
        asked = {plainly(word) for word in entry.get("answers", [])}
        out.extend(
            f"{entry['id']} is asked by the start of {word!r}, which doesn't ask it whole."
            for word in entry.get("started", [])
            if plainly(word) not in asked
        )
    return out


def capitalised(said: str) -> bool:
    """Say whether words start with a capital, or with no letter at all (a sum's form)."""
    return not said[:1].isalpha() or said[:1].isupper()


def miscased(vocabulary: Entry) -> list[str]:
    """Say what's shown in another case than the rest of the rows."""
    out = [
        f"The {category['id']} category's name {category.get('name', '')!r} isn't capitalised."
        for category in vocabulary["categories"]
        if not category.get("name") or not capitalised(category["name"])
    ]
    for entry in vocabulary["things"]:
        out.extend(
            f"{entry['id']}'s name {state['name']!r} isn't capitalised."
            for state in [entry, *entry.get("states", {}).values()]
            if state.get("name") and not capitalised(state["name"])
        )
        empty = entry.get("empty", "")
        if empty and not (capitalised(empty) and empty.endswith(".")):
            out.append(f"{entry['id']}'s empty {empty!r} isn't a sentence.")
    return out


# What Joel never reads, being the machinery's names rather than his: the
# programs and daemons behind a row, and the system's own words. Hidden
# words may hold them, as what he typed before still finds a row.
MACHINERY = [
    "bluetoothctl",
    "bluez",
    "claude",
    "desk-facts",
    "hwmon",
    "iw",
    "kernel",
    "lag-watch",
    "niri",
    "nix",
    "pi",
    "pipewire",
    "quickshell",
    "spend-watch",
    "sysfs",
    "systemd",
    "tuple",
    "tzdata",
    "upower",
    "v4l2",
    "wifid",
    "wireplumber",
]
MACHINE_NAME = re.compile(
    r"(?<![\w-])(" + "|".join(re.escape(name) for name in MACHINERY) + r")(?![\w-])",
    re.IGNORECASE,
)

# Units the launcher writes one way everywhere, and the ways it doesn't:
# "10 bit" not "10-bit", decimal "kB" not "KB", bits a second as "Mb/s",
# and a rate in hertz rather than "a second".
STRAY_UNITS = [
    (re.compile(r"(\d|\})-bit\b"), 'a bit count is written "10 bit"'),
    (re.compile(r"\bKB\b"), 'a kilobyte is written "kB"'),
    (re.compile(r"\b[kKMG]bits?/s\b|\b[kKMG]Bit/s\b|\b[kKMG]bps\b"), 'bits a second are "Mb/s"'),
    (re.compile(r"(\d|\}) a second\b"), "a rate is written in Hz"),
]

# A verb of two words is a turn, which says the way it goes.
TURN = "turn"


def shown(entry: Entry) -> list[tuple[str, str]]:
    """Give every string of a thing Joel reads, in any state, with what it is."""
    said: list[tuple[str, str]] = []
    for state in [entry, *entry.get("states", {}).values()]:
        said.extend((key, state[key]) for key in ("name", "empty", "sure", "ask")
                    if isinstance(state.get(key), str))  # fmt: skip
        words = state.get("said", {})
        said.extend(("said", word) for word in
                    (words.values() if isinstance(words, dict) else [words]))  # fmt: skip
    return said


def unsaid(vocabulary: Entry) -> list[str]:
    """Say what's shown that Joel wouldn't say, or says another way elsewhere."""
    out: list[str] = []
    strings = [(entry["id"], key, text) for entry in vocabulary["things"]
               for key, text in shown(entry)]  # fmt: skip
    strings.extend((command["id"], key, command.get(key, ""))
                   for command in vocabulary["commands"] for key in ("form", "says"))  # fmt: skip
    for thing, key, text in strings:
        named = MACHINE_NAME.search(text)
        if named:
            out.append(f"{thing}'s {key} {text!r} names {named.group(1)}, which Joel never reads.")
        out.extend(f"{thing}'s {key} {text!r}: {why}." for unit, why in STRAY_UNITS
                   if unit.search(text))  # fmt: skip
        if key == "name" and text[:1].isdigit():
            out.append(f"{thing}'s name {text!r} reads its value first.")
        if key == "said" and text[:1].isupper():
            out.append(f"{thing} says {text!r} beside it with a capital, as no value starts.")
    for entry in vocabulary["things"]:
        for state in [entry, *entry.get("states", {}).values()]:
            verb = state.get("verb", "").split()
            if len(verb) > 1 and verb[0] != TURN:
                out.append(f"{entry['id']}'s verb {' '.join(verb)!r} is more than a word.")
    return out


def main(argv: list[str]) -> None:
    """Print every ambiguous word and stray name, and fail if there are any."""
    vocabulary = json.loads(Path(argv[0]).read_text())
    glyphs = set(json.loads(Path(argv[1]).read_text()))
    found = (
        clashes(vocabulary)
        + misplaced(vocabulary, glyphs)
        + miscased(vocabulary)
        + unsaid(vocabulary)
    )
    for said in found:
        print(said)
    if found:
        sys.exit(f"The launcher's vocabulary has {len(found)} words to settle.")


if __name__ == "__main__":
    main(sys.argv[1:])
