"""Check every launcher verb names the command the : line runs it by.

    launcher-commands WORDS LANGUAGE

WORDS is the vocabulary (modules/quickshell/launcher/words/, joined), LANGUAGE
the key language (modules/keys/language.toml). Each verb, a thing whose id
begins verb:, says its command by its whole name, one an act in the
language has, and what follows it on the line where it needs more: an
argument, as :new . or :answer yes, which for :set is the whole word the
row sets, since switches and levels are options, as :set nowifi and :set
volume+=5; a bang, the other way, as :skip! goes back and :w! replaces;
or a range of every row, %, as :%forg, only on an act that takes rows.
A verb Return alone does, as Open the Marked on what's gathered, says so
as the key language's do and names no command, since a command that
lists, as :args, can't also open. A row that's an act itself, as Lock
or Wi-Fi Off, names its command the same way where the language has one
for it, and is held as a verb is. A row knows its state, so a :set
names the state it sets, as :set wifi, never a toggle, as wifi! or
invwifi, and a switch whose words have states names each state's own,
since they turn it opposite ways. Fails, naming each, on a verb with no
command or one the language hasn't got, a :set with nothing to set or a
toggle, a switch's state naming nothing of its own, an argument with
spaces around it, a range on an act that takes none, a
verb Return alone does that names a command too, and two verbs on one
row that the line can't tell apart, since two acts on a row differ by
what follows the command, never by order (VKCF3Z's spec, The Command
Line). And so does the check itself, should a rule stop refusing what
it's there for, or refuse what sits beside it.
"""

import json
import re
import sys
import tomllib
from pathlib import Path
from typing import Any

Entry = dict[str, Any]
# Each command the language names, and whether its act takes rows.
Commands = dict[str, bool]

# The category of what's on: each of its verbs belongs to its own piece,
# named by its id's third part, as verb:on:waiting:go is the waiting one's.
ON = "on"
# The range of every row, as :%d is.
EVERY_ROW = "%"
# The key language's act for Return, which a verb it alone does names.
RETURN = "do"
# Vim's ways of toggling a switch, :set wifi! and :set invwifi, which say
# nothing of which way it goes.
TOGGLES = re.compile(r"^inv[a-z]+$|!$")
# A switch turned one way, :set wifi or :set nowifi, which a row in its
# other state turns the other.
SWITCH = re.compile(r"^[a-z]+$")
# What sets an option: the machine's, or with :setl that of what it's on.
SETS = {"set", "setlocal"}


def commands_of(language: Entry) -> Commands:
    """Read each command the language names, and whether it takes rows."""
    return {
        act["command"]: bool(act.get("range"))
        for act in language["acts"].values()
        if "command" in act
    }


def row_of(verb: Entry) -> str:
    """Say which row a verb is on, as the line reads them."""
    place = str(verb.get("in", verb.get("of", verb["id"])))
    if place == ON:
        return f"{ON}:{verb['id'].split(':')[2]}"
    return place


def line_of(verb: Entry) -> str:
    """Write the line a verb's command makes, as it's typed after the colon."""
    command = verb["command"]
    rest = [verb["argument"]] if "argument" in verb else []
    bang = "!" if verb.get("bang") else ""
    return " ".join([verb.get("range", "") + command + bang, *rest])


def strays(verb: Entry, commands: Commands) -> list[str]:
    """Say how one verb's command strays."""
    name = verb["id"]
    if verb.get("act") == RETURN:
        if "command" in verb:
            return [f"{name} is Return's alone, yet names :{verb['command']}."]
        return []
    if "command" not in verb:
        return [f"{name} names no command."]
    command = verb["command"]
    out = []
    if command not in commands:
        out.append(f"{name}'s :{command} is no act's in the key language.")
    if command in SETS and "argument" not in verb:
        out.append(f"{name}'s :{command} sets nothing.")
    argument = verb.get("argument")
    if argument is not None and (argument != argument.strip() or not argument):
        out.append(f"{name}'s argument {argument!r} isn't one word or more, spaces trimmed.")
    if "range" in verb and (verb["range"] != EVERY_ROW or not commands.get(command)):
        out.append(f"{name}'s range {verb['range']!r} isn't every row of an act that takes rows.")
    if command in SETS:
        out += toggles(verb)
    return out


def toggles(verb: Entry) -> list[str]:
    """Say where a :set or :setl row toggles, or a switch's state names nothing of its own."""
    name = verb["id"]
    command = verb["command"]
    said = {"": verb, **verb.get("states", {})}
    out = [
        f"{name}{f' {state}' if state else ''} toggles, :{command} {words['argument']}."
        for state, words in said.items()
        if TOGGLES.search(str(words.get("argument", "")))
    ]
    out += [
        f"{name} {state} names no :{command} of its own."
        for state, words in verb.get("states", {}).items()
        if "argument" not in words and SWITCH.match(str(verb.get("argument", "")))
    ]
    return out


def unnamed(vocabulary: Entry, commands: Commands) -> list[str]:
    """Say every verb whose command strays, and two a row can't tell apart."""
    verbs = [t for t in vocabulary["things"] if str(t.get("id", "")).startswith("verb:")]
    out = [said for verb in verbs for said in strays(verb, commands)]
    # A row that's an act itself, as Lock, names its command where the
    # language has one for it, held as a verb's is.
    rows = [t for t in vocabulary["things"] if t not in verbs and "command" in t]
    out += [said for row in rows for said in strays(row, commands)]
    seen: dict[tuple[str, str], str] = {}
    for verb in verbs:
        if "command" not in verb:
            continue
        line = line_of(verb)
        other = seen.setdefault((row_of(verb), line), verb["id"])
        if other != verb["id"]:
            out.append(f"{other} and {verb['id']} are both :{line} on {row_of(verb)}.")
    return out


# A language of three acts, two taking rows, for the check's own strays.
OWN: Commands = {"forget": True, "set": False, "setlocal": True}

# What each rule is there to refuse, one stray each, so a rule that stops
# refusing fails the check rather than passing everything.
REFUSED: list[tuple[str, list[Entry]]] = [
    ("a verb with no command", [{"id": "verb:bad", "in": "x"}]),
    ("a command no act has", [{"id": "verb:bad", "in": "x", "command": "frob"}]),
    ("a :set setting nothing", [{"id": "verb:bad", "in": "x", "command": "set"}]),
    (
        "an argument with a space before it",
        [{"id": "verb:bad", "in": "x", "command": "forget", "argument": " all"}],
    ),
    (
        "a range of some rows",
        [{"id": "verb:bad", "in": "x", "command": "forget", "range": "1,3"}],
    ),
    (
        "a range on an act taking none",
        [{"id": "verb:bad", "in": "x", "command": "set", "argument": "wifi", "range": "%"}],
    ),
    (
        "a verb Return alone does naming a command",
        [{"id": "verb:bad", "in": "x", "act": "do", "command": "forget"}],
    ),
    (
        "a row that's an act naming a command no act has",
        [{"id": "session:bad", "command": "frob"}],
    ),
    (
        "a row's :set setting nothing",
        [{"id": "session:bad", "command": "set"}],
    ),
    (
        "a :set toggling",
        [{"id": "verb:bad", "in": "x", "command": "set", "argument": "wifi!"}],
    ),
    (
        "a :setl toggling",
        [{"id": "verb:bad", "in": "x", "command": "setlocal", "argument": "mute!"}],
    ),
    (
        "a :set toggling as inv does",
        [{"id": "session:bad", "command": "set", "argument": "invwifi"}],
    ),
    (
        "a state's :set toggling",
        [
            {
                "id": "session:bad",
                "command": "set",
                "argument": "nowifi",
                "states": {"off": {"argument": "wifi!"}},
            }
        ],
    ),
    (
        "a :set row whose state names nothing of its own",
        [
            {
                "id": "session:bad",
                "command": "set",
                "argument": "nowifi",
                "states": {"off": {"name": "Wi-Fi On"}},
            }
        ],
    ),
    (
        "two verbs alike on a row",
        [
            {"id": "verb:one", "in": "x", "command": "forget"},
            {"id": "verb:two", "in": "x", "command": "forget"},
        ],
    ),
    (
        "two verbs alike on one piece of what's on",
        [
            {"id": "verb:on:waiting:one", "in": ON, "command": "forget"},
            {"id": "verb:on:waiting:two", "in": ON, "command": "forget"},
        ],
    ),
]

# What sits beside each rule's edge and isn't refused: one command on two
# rows, or on two pieces of what's on; one told apart on a row by an
# argument, a bang or every row; one Return alone does, beside one with
# no argument; and a thing that isn't a verb.
ALLOWED: list[Entry] = [
    {"id": "verb:a", "in": "x", "command": "forget"},
    {"id": "verb:b", "in": "y", "command": "forget"},
    {"id": "verb:c", "in": "x", "command": "forget", "argument": "seen"},
    {"id": "verb:d", "in": "x", "command": "forget", "range": "%"},
    {"id": "verb:g", "in": "x", "command": "forget", "bang": True},
    {"id": "verb:e", "in": "x", "command": "set", "argument": "wifi"},
    {"id": "verb:f", "in": "x", "command": "set", "argument": "volume+=5"},
    {"id": "verb:h", "in": "x", "act": "do"},
    {"id": "verb:on:waiting:go", "in": ON, "command": "forget"},
    {"id": "verb:on:playing:go", "in": ON, "command": "forget"},
    {"id": "thing:not-a-verb", "in": "x"},
    {
        "id": "session:good",
        "command": "set",
        "argument": "nowifi",
        "states": {"off": {"argument": "wifi"}},
    },
    {"id": "session:invite", "command": "set", "argument": "invite=all"},
    {
        "id": "verb:i",
        "in": "x",
        "command": "set",
        "argument": "bluetooth+={name}",
        "states": {"unpaired": {"name": "Pair and Connect It"}},
    },
    {"id": "verb:skip", "in": "x", "command": "forget", "bang": True, "argument": "all"},
]


def refuses() -> list[str]:
    """Say each rule that lets through what it's there to refuse."""
    out = [
        f"The check lets {why} through: {verbs}."
        for why, verbs in REFUSED
        if len(unnamed({"things": verbs}, OWN)) != 1
    ]
    out.extend(
        f"The check refuses what it shouldn't: {said}" for said in unnamed({"things": ALLOWED}, OWN)
    )
    return out


def main(argv: list[str]) -> None:
    """Print every verb the line can't name, and fail if there are any."""
    vocabulary = json.loads(Path(argv[0]).read_text())
    commands = commands_of(tomllib.loads(Path(argv[1]).read_text()))
    found = refuses() + unnamed(vocabulary, commands)
    for said in found:
        print(said)
    if found:
        sys.exit(f"{len(found)} of the launcher's verbs name no command the : line runs.")


if __name__ == "__main__":
    main(sys.argv[1:])
