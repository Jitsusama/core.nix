"""Check that every why the shell gives a press it can't answer has words in the foot.

    foot-words AGENTS SESSIONS LAUNCHER

AGENTS is the vocabulary's own file for agents (launcher/words/agents.json),
SESSIONS the shell's Sessions.qml and LAUNCHER the launcher's Agents.qml. Each
why Sessions says a session by (`unhosted(session, "why")`, one or a choice
of two) and each key the launcher names to its foot (`elsewhere("key", ...)`)
must have words in agents:machine's `said`: the launcher reads them there and
says nothing for a key it hasn't, so a press would be met with an empty foot
(Joel, 2026-11-15: a press must never do nothing). Keys the launcher reads off
agents:machine by name elsewhere, as SessionRow.js's unsteered does, aren't
read here. A call is read to its closing paren, however it's wrapped. Each why
or key missing, or worded "", fails, printed, and so does a why or key that can
come to anything but a word, as one held in a name, which can't be read; only
the why the launcher's onUnhosted is given, inside that handler, may be a
name, and that handler must pass it on to elsewhere, or every why the shell
gives would meet an empty foot. A word a choice is made by is no why. The
check first proves it finds a why in a choice or a wrapped call and refuses a
why with no words, so a rule that stopped finding or refusing fails it too.
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
# The launcher's handler of the shell's unhosted, whose second parameter
# holds the why the shell gave, up to its body's opening brace.
HANDLER = re.compile(r"\bfunction onUnhosted\(\s*\w+\s*,\s*(\w+)\s*\)[^{]*\{")
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


def top_level(text: str, wanted: str) -> list[int]:
    """Give where `wanted` stands in `text` outside brackets and quotes.

    A ? is wanted alone, never as part of ?? or ?.
    """
    found: list[int] = []
    depth = 0
    quote = ""
    at = 0
    while at < len(text):
        one = text[at]
        if quote:
            if one == "\\":
                at += 1
            elif one == quote:
                quote = ""
        elif one in QUOTES:
            quote = one
        elif one in OPENS:
            depth += 1
        elif one in CLOSES:
            depth -= 1
        elif depth == 0 and text.startswith(wanted, at):
            alone = wanted != "?" or (
                text[at + 1 : at + 2] not in "?." and (at == 0 or text[at - 1] != "?")
            )
            if alone:
                found.append(at)
                at += len(wanted) - 1
        at += 1
    return found


def outcomes(why: str) -> list[str]:
    """Give what a why can come to: each side of ?? or ||, or each choice a ? makes.

    What a choice is made by is none of them, so a word compared there is
    no why.
    """
    why = why.strip()
    for either in ("??", "||"):
        splits = top_level(why, either)
        if splits:
            edges = [0, *(at + len(either) for at in splits)]
            ends = [*splits, len(why)]
            return [
                part
                for start, end in zip(edges, ends, strict=True)
                for part in outcomes(why[start:end])
            ]
    asks = top_level(why, "?")
    if asks:
        rest = why[asks[0] + 1 :]
        # The : that ends this choice's first side, past any it holds.
        nested = 0
        for at in sorted(top_level(rest, "?") + top_level(rest, ":")):
            if rest[at] == "?":
                nested += 1
            elif nested:
                nested -= 1
            else:
                return outcomes(rest[:at]) + outcomes(rest[at + 1 :])
    return [why]


def whys(source: str) -> set[str]:
    """Give each why a call of unhosted gives, every word its second argument can come to."""
    found: set[str] = set()
    for why in argument(source, UNHOSTED, 1):
        found.update(said for outcome in outcomes(why) for said in WORD.findall(outcome) if said)
    return found


def unread(source: str) -> list[str]:
    """Give each call's why that can come to anything but a word of its own, so can't be read."""
    return [
        why
        for why in argument(source, UNHOSTED, 1)
        if not all(WORD.fullmatch(outcome) for outcome in outcomes(why))
    ]


def keys(source: str) -> set[str]:
    """Give each key the launcher's elsewhere is asked for by name, its first argument's.

    One passed on by name is a why the shell gave, so whys reads it there.
    """
    found: set[str] = set()
    for first in argument(source, ELSEWHERE, 0):
        found.update(said for said in WORD.findall(first) if said)
    return found


def handler(source: str) -> tuple[str, int, int] | None:
    """Give the name onUnhosted keeps the shell's why in, and where its body begins and ends.

    A brace inside a string or a comment is none of the body's, and a quote
    inside a comment opens no string, so neither moves where it ends.
    """
    found = HANDLER.search(source)
    if found is None:
        return None
    depth, quote, at = 1, "", found.end()
    while depth and at < len(source):
        one = source[at]
        if quote:
            if one == "\\":
                at += 1
            elif one == quote:
                quote = ""
        elif source.startswith("//", at):
            at = source.find("\n", at)
            at = len(source) if at < 0 else at
        elif source.startswith("/*", at):
            at = source.find("*/", at)
            at = len(source) if at < 0 else at + 1
        elif one in QUOTES:
            quote = one
        else:
            depth += {"{": 1, "}": -1}.get(one, 0)
        at += 1
    return found.group(1), found.end(), at


def firsts(source: str) -> list[tuple[int, str]]:
    """Give where each call of elsewhere stands, with its first argument."""
    found: list[tuple[int, str]] = []
    for one in ELSEWHERE.finditer(source):
        given = given_to(source, one.end())
        found.append((one.start(), given[0] if given else ""))
    return found


def shells(held: tuple[str, int, int] | None, at: int, first: str) -> bool:
    """Say whether a call at `at` passes on the shell's why, inside the handler given it."""
    return held is not None and first.strip() == held[0] and held[1] <= at < held[2]


def unkeyed(source: str) -> list[str]:
    """Give each key elsewhere is asked for that can come to anything but a word.

    A key held in the name onUnhosted keeps the shell's why in, inside that
    handler, is read where the shell gives it (whys), so only it may be a
    name.
    """
    held = handler(source)
    return [
        first
        for at, first in firsts(source)
        if not shells(held, at, first)
        and not all(WORD.fullmatch(outcome) for outcome in outcomes(first))
    ]


def passes_on(source: str) -> bool:
    """Say whether onUnhosted passes the shell's why on to elsewhere, so it reaches the foot."""
    held = handler(source)
    return any(shells(held, at, first) for at, first in firsts(source))


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
    choice = 'root.unhosted(footed, hostly ? "unviewable" : "unviewableHost");'
    wrapped = 'root.unhosted(footed,\n    hostly ? "unviewable" : "unviewableHost");'
    vocabulary = {"things": [{"id": "agents:machine", "said": {"hosted": "x", "opening": ""}}]}
    # What each rule gives a source made to test it, what it should, and
    # the gap it lets through when the two differ.
    held: list[tuple[object, object, str]] = [
        (whys(choice), {"unviewable", "unviewableHost"}, "a why given as a choice of two"),
        (whys("signal unhosted(var session, string why)"), set(), "the signal's own declaration"),
        (
            whys(wrapped),
            {"unviewable", "unviewableHost"},
            "a why wrapped onto the call's next line",
        ),
        (
            whys('unhosted(said, "unviewable-host");'),
            {"unviewable-host"},
            "a why that isn't letters alone",
        ),
        (
            whys('unhosted({ address: quest, machine: "" }, "hostless");'),
            {"hostless"},
            "a why after a first argument holding a comma",
        ),
        (unread("unhosted(said, why);"), ["why"], "a why held in a name, which can't be read"),
        (
            unread('unhosted(said, why ?? "hosted");'),
            ['why ?? "hosted"'],
            "a why held half in a name",
        ),
        (
            whys('unhosted(said, named === "x" ? "a" : "b");'),
            {"a", "b"},
            "a word a choice is made by, read as a why",
        ),
        (
            keys('provider.elsewhere("unopened", session), Theme.alert]]);'),
            {"unopened"},
            "a key the launcher names",
        ),
        (
            unkeyed(
                "function onUnhosted(session, why) { tell(elsewhere(why, session)); }\n"
                'elsewhere(reason, session); elsewhere(why, s); elsewhere("a", s);'
            ),
            ["reason", "why"],
            "a key held in a name, but the shell's why in its own handler",
        ),
        (
            passes_on("function onUnhosted(session, why) { tell(elsewhere(why, session)); }"),
            True,
            "a handler passing the shell's why on, taken for none",
        ),
        (
            passes_on(
                'function onUnhosted(session, why) { tell(elsewhere("x", session)); }\n'
                "elsewhere(why, session);"
            ),
            False,
            "a handler not passing the shell's why on, its name used elsewhere",
        ),
        (
            passes_on(
                'function onUnhosted(session, why) { tell("{", elsewhere("x", session)); }\n'
                "elsewhere(why, session);"
            ),
            False,
            "a brace in a string in the handler, read as one of its blocks",
        ),
        (
            passes_on(
                "function onUnhosted(session, why) {\n    // isn't it\n"
                '    tell(elsewhere("x", session)); }\nelsewhere(why, session);'
            ),
            False,
            "a quote in a comment in the handler, read as opening a string",
        ),
        (
            unworded({"hosted", "opening", "unnamed"}, said(vocabulary)),
            ["opening", "unnamed"],
            "a why worded empty or not at all",
        ),
    ]
    return [rule for given, wanted, rule in held if given != wanted]


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
    launcher = Path(arguments[2]).read_text(encoding="utf-8")
    asked = keys(launcher)
    unasked = unkeyed(launcher)
    for key in unasked:
        print(f"foot-words: a key given as {key or 'nothing'} can't be read", file=sys.stderr)
    passed = passes_on(launcher)
    if not passed:
        print(
            f"foot-words: {arguments[2]} has no onUnhosted passing the shell's why to elsewhere,"
            " so a why the shell gives would meet an empty foot",
            file=sys.stderr,
        )
    if not given:
        print(
            f"foot-words: {arguments[1]} gives no why, so the check reads it wrong", file=sys.stderr
        )
    missing = unworded(given | asked, said(vocabulary))
    for key in missing:
        print(f"foot-words: {key} has no words in agents:machine's said", file=sys.stderr)
    unheld = let_through or missing or named or unasked or not passed or not given
    return 1 if unheld else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
