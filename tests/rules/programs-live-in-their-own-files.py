"""Check that no program past five lines is written as a string in Nix.

    programs-live-in-their-own-files TREE WAITING

A program in a Nix string is read by no linter, type checker or editor, and
the values spliced into it are where quoting breaks, so a program past five
lines lives in a file of its own language, read in or built into the string.
This reads every `''` string of every .nix file in TREE and counts as a
program each one that runs as shell, given to a builder that runs it
(runCommand, writeShellScript, writeShellApplication and their kin) or set as
a script a build or a unit runs (a phase, a hook, `script`, `buildCommand`,
an activation), and each one that opens as QML, Python or polkit's
JavaScript. Prose and configuration don't count. A shell string of at most ten
lines with at most one branch, loop, pipe or `$(...)` is a splice of values,
a wrapper calling a program with paths, and may stay; so may a string whose
comment above says, in a sentence beginning "Kept in Nix because", why.
Lines count without blanks, and what `${...}` splices in counts on its own.

WAITING is JSON naming, for each file that held programs before the rule, how
many it holds while they move out. A file holding more fails, naming each; a
file holding fewer fails until its count is lowered, so the list only shrinks.
Every failure is printed before the check exits 1.
"""

import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Iterator

MOST_LINES = 5
MOST_SPLICE_LINES = 10
MOST_SPLICE_TURNS = 1

# Builders that run the string they're given as shell.
SHELL_BUILDERS = re.compile(
    r"\b(runCommand\w*|writeShellScript(Bin)?|writeShellApplication|writeScript(Bin)?"
    r"|writers\.write(Bash|Dash)\w*|activationScripts|entryAfter|entryBefore|entryAnywhere)\b"
)
# Attributes whose string a build or a unit runs as shell.
SHELL_ATTRIBUTES = re.compile(
    r"^(\w+Phase|(pre|post)[A-Z]\w*|script|preStart|postStart|preStop|postStop|reload"
    r"|buildCommand|shellHook|extraCommands)$"
)
# How a string that's a program of another language opens.
OPENINGS = {
    "QML": re.compile(r"^(pragma Singleton|import (Qt|Quickshell)\w*|(readonly )?property \w)"),
    "Python": re.compile(r"^(import \w[\w.]*(, \w[\w.]*)*$|from [\w.]+ import )"),
    "JavaScript": re.compile(r"^polkit\.addRule\("),
    # Shell given to a helper of a module's own opens as only shell would.
    "shell": re.compile(
        r"^(export \w+=|set -[eux]+$|\w+\(\) \{|cd |mkdir |cp |install |for \w+ in |if \["
        r"|\w+=\$\(|\($)"
    ),
}
# Attributes whose string is prose, however it opens.
PROSE = {"description", "longDescription", "message", "example"}
# What makes shell more than a splice of values: a branch, a loop, a pipe, a
# command's output spliced in, or a function.
TURNS = re.compile(
    r"(^\s*|[;&|(]\s*)(if|for|while|until|case)\b|\$\(|&&|\|\||(?<![|>])\|(?!\|)|\w+\s*\(\)\s*\{"
)
KEPT = re.compile(r"(^|[.:;!?] )Kept in Nix because\b")


@dataclass
class Block:
    """One `''` string: where it opens, and its text with what's spliced in as `${}`."""

    start: int
    body: str


class Lexer:
    """Reads a Nix file for its `''` strings, nested ones included.

    Beside the text it keeps a skeleton: the same text with every comment and
    every string's own characters blanked, so what code a string is given to
    can be read back without a string or a comment misleading it.
    """

    def __init__(self, text: str) -> None:
        """Read the text for its strings."""
        self.text = text
        self.skeleton = list(text)
        self.blocks: list[Block] = []
        self.code(0, until_brace=False)
        self.blocks.sort(key=lambda block: block.start)

    def blank(self, start: int, end: int) -> None:
        """Blank the skeleton from start to end, keeping its lines."""
        for at in range(start, min(end, len(self.text))):
            if self.text[at] != "\n":
                self.skeleton[at] = " "

    def code(self, at: int, *, until_brace: bool) -> int:
        """Read Nix code from at to the unmatched `}`, or the end; give where it stops."""
        text, depth = self.text, 0
        while at < len(text):
            if text[at] == "#":
                end = text.find("\n", at)
                end = len(text) if end == -1 else end
                self.blank(at, end)
                at = end
            elif text.startswith("/*", at):
                end = text.find("*/", at + 2)
                end = len(text) if end == -1 else end + 2
                self.blank(at, end)
                at = end
            elif text[at] == '"':
                at = self.quoted(at + 1)
            elif text.startswith("''", at):
                at = self.indented(at + 2)
            elif text[at] == "{":
                depth += 1
                at += 1
            elif text[at] == "}":
                if depth == 0 and until_brace:
                    return at + 1
                depth -= 1
                at += 1
            else:
                at += 1
        return at

    def quoted(self, at: int) -> int:
        """Read a "..." string from after its quote; give where it ends."""
        text = self.text
        while at < len(text):
            if text[at] == "\\":
                self.blank(at, at + 2)
                at += 2
            elif text[at] == '"':
                return at + 1
            elif text.startswith("${", at):
                at = self.code(at + 2, until_brace=True)
            else:
                self.blank(at, at + 1)
                at += 1
        return at

    def indented(self, at: int) -> int:
        """Read a `''` string from after its quotes; keep it with splices as `${}`."""
        text, start, body = self.text, at - 2, []
        while at < len(text):
            if text.startswith(("'''", "''$"), at):
                body.append(text[at + 1 :][:2] if text[at + 2] == "'" else "$")
                self.blank(at, at + 3)
                at += 3
            elif text.startswith("''\\", at):
                body.append(text[at + 3 : at + 4])
                self.blank(at, at + 4)
                at += 4
            elif text.startswith("''", at):
                self.blocks.append(Block(start, "".join(body)))
                return at + 2
            elif text.startswith("${", at):
                body.append("${}")
                at = self.code(at + 2, until_brace=True)
            else:
                body.append(text[at])
                self.blank(at, at + 1)
                at += 1
        return at


# Where one expression ends and the next begins, once brackets have collapsed:
# a bracket still open, a binding's end, a list's comma or a keyword.
SEAMS = re.compile(r";|\{(?!\})|\((?!\))|\[(?!\])|,|\b(?:in|then|else|let)\b")


def head(skeleton: str) -> tuple[str, str]:
    """Give what a string is given to, and what the attribute set it's set in is given to.

    Strings and brackets before the string collapse, so the code just before it
    reads as `name = builder "" { }` or `attribute =`.
    """
    s = " ".join(skeleton[-4000:].split())
    last = None
    while last != s:
        last = s
        s = re.sub(r"\{[^{}]*\}", "{}", s)
        s = re.sub(r"\([^()]*\)", "()", s)
        s = re.sub(r"\[[^\[\]]*\]", "[]", s)
        s = re.sub(r"(\"|'')[ ${}]*(\"|'')", '""', s)
    given = SEAMS.split(s)[-1]
    opener = max(s.rfind("{"), 0)
    while opener and s[opener + 1 : opener + 2] == "}":
        opener = max(s.rfind("{", 0, opener), 0)
    outer = SEAMS.split(s[:opener])[-1] if opener else ""
    return given.strip(), outer.strip()


def kind(block: Block, skeleton: str) -> str | None:
    """Say what language a string's program is in, or None for prose and configuration."""
    given, outer = head(skeleton[: block.start])
    attribute = re.match(r"^(?:[\w.\"]+\.)?(\w+)\s*=", given)
    name = attribute.group(1) if attribute else ""
    if name in PROSE:
        return None
    if SHELL_BUILDERS.search(given) or SHELL_ATTRIBUTES.match(name):
        return "shell"
    # A builder's attribute set: writeShellApplication's text, or the string
    # runCommand is given after its set.
    if (name == "text" or given in ("", "}")) and SHELL_BUILDERS.search(outer):
        return "shell"
    first = next((line.strip() for line in block.body.splitlines() if line.strip()), "")
    return next((language for language, opens in OPENINGS.items() if opens.match(first)), None)


def is_splice(lines: list[str]) -> bool:
    """Whether shell lines are a splice of values: short, with at most one turn."""
    turns = sum(len(TURNS.findall(re.sub(r"'[^'\n]*'", "''", line))) for line in lines)
    return len(lines) <= MOST_SPLICE_LINES and turns <= MOST_SPLICE_TURNS


def kept(block: Block, text: str) -> bool:
    """Whether the comment above a string says why it's kept in Nix."""
    above: list[str] = []
    lines = text[: block.start].splitlines()
    for line in reversed(lines[:-1]):
        if not line.strip().startswith("#"):
            break
        above.insert(0, line.strip().lstrip("#"))
    return KEPT.search(" ".join(" ".join(above).split())) is not None


def programs(tree: Path) -> Iterator[tuple[str, int, str, int]]:
    """Give each program past its limit: its file, line, language and length."""
    for path in sorted(tree.rglob("*.nix")):
        if ".git" in path.relative_to(tree).parts:
            continue
        lexer = Lexer(path.read_text(errors="replace"))
        skeleton = "".join(lexer.skeleton)
        for block in lexer.blocks:
            lines = [line for line in block.body.splitlines() if line.strip()]
            language = kind(block, skeleton) if len(lines) > MOST_LINES else None
            if language is None or kept(block, lexer.text):
                continue
            if language == "shell" and is_splice(lines):
                continue
            line = lexer.text.count("\n", 0, block.start) + 1
            yield path.relative_to(tree).as_posix(), line, language, len(lines)


def programs_in_strings(count: int) -> str:
    """Say how many programs are in strings, in words that agree with the count."""
    return f"{count} program{'' if count == 1 else 's'} in strings"


def main() -> None:
    """Say each file holding more programs than it waits for, and each holding fewer."""
    try:
        tree, waiting_file = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: programs-live-in-their-own-files TREE WAITING")
    waiting: dict[str, int] = json.loads(waiting_file.read_text())
    found: dict[str, list[str]] = {}
    for path, line, language, length in programs(tree):
        found.setdefault(path, []).append(f"{path}:{line}: {length} lines of {language}")
    said: list[str] = []
    for path, held in sorted(found.items()):
        if len(held) > waiting.get(path, 0):
            said.append(
                f"{path} holds {programs_in_strings(len(held))}, {waiting.get(path, 0)} of them "
                "waiting: move the new one into a file of its own language, or say in a comment "
                'above it, in a sentence beginning "Kept in Nix because", why it stays.'
            )
            said.extend(f"  {program}" for program in held)
    said.extend(
        f"{path} waits with {programs_in_strings(count)} but holds {len(found.get(path, []))}: "
        "lower its count on the waiting list, or take it off."
        for path, count in sorted(waiting.items())
        if len(found.get(path, [])) < count
    )
    for failure in said:
        print(failure, file=sys.stderr)
    total = sum(len(held) for held in found.values())
    files = f"{len(found)} file{'' if len(found) == 1 else 's'}"
    print(f"{programs_in_strings(total)}, in {files}.")
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()
