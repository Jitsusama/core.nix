"""Check that every why the shell gives a press it can't answer has words in the foot.

    foot-words AGENTS SESSIONS LAUNCHER

AGENTS is the vocabulary's own file for agents (launcher/words/agents.json),
SESSIONS the shell's Sessions.qml and LAUNCHER the launcher's Agents.qml. Each
why Sessions says a session by (`unhosted(session, "why")`, one or a choice
of two) and each key the launcher says in its foot (`elsewhere("key", ...)`)
must have words in agents:machine's `said`: the launcher reads them there and
says nothing for a key it hasn't, so a press would be met with an empty foot
(Joel, 2026-11-15: a press must never do nothing). A call is read to its
closing paren, however it's wrapped. Each why or key missing, or worded "",
fails, printed, and so does a why given by a name, which can't be read. The
check first proves it finds a why in a choice or a wrapped call and refuses
a why with no words, so a rule that stopped finding or refusing fails it too.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

# The vocabulary's file, Sessions.qml and Agents.qml.
ARGUMENTS = 3

# A call of the signal, not its declaration.
UNHOSTED = re.compile(r"(?<!signal )\bunhosted\(")
# The launcher saying a key of agents:machine's in its foot, not the
# function's own declaration.
ELSEWHERE = re.compile(r"(?<!function )\belsewhere\(")
# A string in double quotes, as each why and key is said, "" too, so the
# quote that closes one is never read as opening the next.
WORD = re.compile(r'"((?:[^"\\]|\\.)*)"')
OPENS = "([{"
CLOSES = ")]}"
QUOTES = "\"'`"


def given_to(source: str, start: int) -> list[str]:
    """Give the arguments of the call whose paren closes after `start`, across lines.

    Commas inside brackets or quotes are an argument's own, as in an object
    literal's, so only the call's own split it.
    """
    given: list[str] = []
    depth = 0
    quote = ""
    begun = start
    at = start
    while at < len(source):
        one = source[at]
        if quote:
            if one == "\\":
                at += 1
            elif one == quote:
                quote = ""
        elif one in QUOTES:
            quote = one
        elif one in OPENS:
            depth += 1
        elif one in CLOSES and depth > 0:
            depth -= 1
        elif one in CLOSES or (one == "," and depth == 0):
            given.append(source[begun:at].strip())
            if one in CLOSES:
                return given
            begun = at + 1
        at += 1
    return given


def argument(source: str, call: re.Pattern[str], which: int) -> list[str]:
    """Give argument `which` of each call `call` finds, or "" for a call with fewer."""
    found: list[str] = []
    for one in call.finditer(source):
        given = given_to(source, one.end())
        found.append(given[which] if len(given) > which else "")
    return found


def whys(source: str) -> set[str]:
    """Give each why a call of unhosted gives, every word of its second argument."""
    found: set[str] = set()
    for why in argument(source, UNHOSTED, 1):
        found.update(said for said in WORD.findall(why) if said)
    return found


def unread(source: str) -> list[str]:
    """Give each call's why that names no word of its own, so can't be read."""
    return [why for why in argument(source, UNHOSTED, 1) if not any(WORD.findall(why))]


def keys(source: str) -> set[str]:
    """Give each key the launcher's elsewhere is asked for by name, its first argument's.

    One passed on by name is a why the shell gave, so whys reads it there.
    """
    found: set[str] = set()
    for first in argument(source, ELSEWHERE, 0):
        found.update(said for said in WORD.findall(first) if said)
    return found


def said(vocabulary: object) -> dict[str, object]:
    """Give agents:machine's `said`, or nothing where the vocabulary has none."""
    things = vocabulary.get("things", []) if isinstance(vocabulary, dict) else []
    for thing in things:
        if isinstance(thing, dict) and thing.get("id") == "agents:machine":
            words = thing.get("said", {})
            return words if isinstance(words, dict) else {}
    return {}


def unworded(wanted: set[str], words: dict[str, object]) -> list[str]:
    """Give each wanted key with no words, in order."""
    return sorted(key for key in wanted if not isinstance(words.get(key), str) or not words[key])


def refuses_what_it_should() -> list[str]:
    """Give each rule that let a gap through, from sources made to have one."""
    let_through: list[str] = []
    choice = 'root.unhosted(footed, hostly ? "unviewable" : "unviewableHost");'
    if whys(choice) != {"unviewable", "unviewableHost"}:
        let_through.append("a why given as a choice of two")
    if whys("signal unhosted(var session, string why)"):
        let_through.append("the signal's own declaration")
    wrapped = 'root.unhosted(footed,\n    hostly ? "unviewable" : "unviewableHost");'
    if whys(wrapped) != {"unviewable", "unviewableHost"}:
        let_through.append("a why wrapped onto the call's next line")
    if whys('unhosted(said, "unviewable-host");') != {"unviewable-host"}:
        let_through.append("a why that isn't letters alone")
    if whys('unhosted({ address: quest, machine: "" }, "hostless");') != {"hostless"}:
        let_through.append("a why after a first argument holding a comma")
    if unread("unhosted(said, why);") != ["why"]:
        let_through.append("a why held in a name, which can't be read")
    if unread('unhosted(said, why ?? "hosted");') != ['why ?? "hosted"']:
        let_through.append("a why held half in a name")
    if whys('unhosted(said, named === "x" ? "a" : "b");') != {"a", "b"}:
        let_through.append("a word a choice is made by, read as a why")
    if keys('provider.elsewhere("unopened", session), Theme.alert]]);') != {"unopened"}:
        let_through.append("a key the launcher names")
    vocabulary = {"things": [{"id": "agents:machine", "said": {"hosted": "x", "opening": ""}}]}
    if unworded({"hosted", "opening", "unnamed"}, said(vocabulary)) != ["opening", "unnamed"]:
        let_through.append("a why worded empty or not at all")
    return let_through


def main(arguments: list[str]) -> int:
    """Check each why and key against agents:machine's words, saying each with none."""
    if len(arguments) != ARGUMENTS:
        print("usage: foot-words AGENTS SESSIONS LAUNCHER", file=sys.stderr)
        return 64
    let_through = refuses_what_it_should()
    for rule in let_through:
        print(f"foot-words: the check lets {rule} through", file=sys.stderr)
    vocabulary = json.loads(Path(arguments[0]).read_text(encoding="utf-8"))
    sessions = Path(arguments[1]).read_text(encoding="utf-8")
    given = whys(sessions)
    named = unread(sessions)
    for why in named:
        print(f"foot-words: a why given as {why or 'nothing'} can't be read", file=sys.stderr)
    asked = keys(Path(arguments[2]).read_text(encoding="utf-8"))
    if not given:
        print(
            f"foot-words: {arguments[1]} gives no why, so the check reads it wrong", file=sys.stderr
        )
    missing = unworded(given | asked, said(vocabulary))
    for key in missing:
        print(f"foot-words: {key} has no words in agents:machine's said", file=sys.stderr)
    return 1 if let_through or missing or named or not given else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
