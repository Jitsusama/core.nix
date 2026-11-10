"""Check that every assertion that can't show what each side held says what was wrong.

    every-assert-says-what-was-wrong TREE WAITING

A failure that says only "assertion failed" makes the reader run the test again
to learn anything. Where the language shows what each side held, as Rust's
`assert_eq!` and a Python comparison under pytest do, nothing is added; where
it can't, the assertion carries a message. This reads the Rust and Python of
TREE for each assertion without one: Rust's `assert!` and `debug_assert!` with
no message after the condition, and Python's `assert` of anything but a
comparison with no message after it.

WAITING is JSON naming, for each file that held such assertions before the
rule, how many it holds while each gains its message. A file holding more
fails, naming each; a file holding fewer fails until its count is lowered, so
the list only shrinks. Every failure is printed before the check exits 1.
"""

import ast
import json
import re
import sys
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Iterator

# Where a Rust assertion that can't show its sides starts.
RUST_ASSERT = re.compile(r"\b(debug_)?assert!\s*\(")
# A raw string's opening, and a character literal, which a lifetime's `'a` isn't.
RAW_STRING = re.compile(r'b?r(#*)"')
CHARACTER = re.compile(r"'(\\.[^']*|[^\\'])'")
OPENS = "([{"
CLOSES = ")]}"


def rust_code(text: str) -> str:
    """Blank out a Rust file's comments, and fill its strings and characters, keeping lines.

    What's left is the code's brackets and commas where they were, so an
    assertion's arguments can be counted without a comma in a string or a
    comment being taken for one between them, while a string still counts as
    something given.
    """
    out: list[str] = []
    index = 0
    while index < len(text):
        char = text[index]
        if text.startswith("//", index):
            end = text.find("\n", index)
            end = len(text) if end < 0 else end
            out.append(" " * (end - index))
            index = end
        elif text.startswith("/*", index):
            end = text.find("*/", index + 2)
            end = len(text) if end < 0 else end + 2
            out.append(re.sub(r"[^\n]", " ", text[index:end]))
            index = end
        elif raw := RAW_STRING.match(text, index):
            close = '"' + raw.group(1)
            end = text.find(close, index + len(raw.group(0)))
            end = len(text) if end < 0 else end + len(close)
            out.append(re.sub(r"[^\n]", "s", text[index:end]))
            index = end
        elif char == '"':
            end = index + 1
            while end < len(text) and text[end] != '"':
                end += 2 if text[end] == "\\" else 1
            out.append(re.sub(r"[^\n]", "s", text[index : end + 1]))
            index = end + 1
        elif literal := CHARACTER.match(text, index):
            out.append("c" * len(literal.group(0)))
            index += len(literal.group(0))
        else:
            out.append(char)
            index += 1
    return "".join(out)


def rust_bare(text: str) -> Iterator[int]:
    """Give the line of each Rust `assert!` or `debug_assert!` with no message."""
    code = rust_code(text)
    for found in RUST_ASSERT.finditer(code):
        depth = 0
        commas = 0
        index = found.end()
        while index < len(code):
            char = code[index]
            if char in OPENS:
                depth += 1
            elif char in CLOSES:
                if depth == 0:
                    break
                depth -= 1
            elif char == "," and depth == 0:
                commas += 1
            index += 1
        # A comma only after the condition is the formatter's trailing one.
        given = code[found.end() : index].rstrip()
        if commas == 0 or (commas == 1 and given.endswith(",")):
            yield code.count("\n", 0, found.start()) + 1


def python_bare(text: str) -> Iterator[int]:
    """Give the line of each Python `assert` of anything but a comparison, with no message."""
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return
    for node in ast.walk(tree):
        if isinstance(node, ast.Assert) and not (node.msg or isinstance(node.test, ast.Compare)):
            yield node.lineno


READERS = {".rs": rust_bare, ".py": python_bare}


def bare_asserts(tree: Path) -> Iterator[tuple[str, int, str]]:
    """Give each assertion that can't show its sides and says nothing: its file, line and code."""
    for path in sorted(tree.rglob("*")):
        relative = path.relative_to(tree)
        if ".git" in relative.parts or path.suffix not in READERS or not path.is_file():
            continue
        text = path.read_text(errors="replace")
        lines = text.splitlines()
        for number in READERS[path.suffix](text):
            yield relative.as_posix(), number, lines[number - 1].strip()


def asserts(count: int) -> str:
    """Say how many assertions say nothing, in words that agree with the count."""
    return f"{count} assertion{'' if count == 1 else 's'} without a message"


def main() -> None:
    """Say each file holding more bare assertions than it waits with, and each holding fewer."""
    try:
        tree, waiting_file = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-assert-says-what-was-wrong TREE WAITING")
    waiting: dict[str, int] = json.loads(waiting_file.read_text())
    found: dict[str, list[str]] = {}
    for path, number, code in bare_asserts(tree):
        found.setdefault(path, []).append(f"{path}:{number}: {code}")
    said: list[str] = []
    for path, held in sorted(found.items()):
        if len(held) > waiting.get(path, 0):
            said.append(
                f"{path} holds {asserts(len(held))}, {waiting.get(path, 0)} of them waiting: "
                "give each a message saying what was expected, or compare with assert_eq! or "
                "==, which show what each side held."
            )
            said.extend(f"  {line}" for line in held)
    said.extend(
        f"{path} waits with {asserts(count)} but holds {len(found.get(path, []))}: lower its "
        "count on the waiting list, or take it off."
        for path, count in sorted(waiting.items())
        if len(found.get(path, [])) < count
    )
    for failure in said:
        print(failure, file=sys.stderr)
    files = f"{len(found)} file{'' if len(found) == 1 else 's'}"
    print(f"{asserts(sum(len(held) for held in found.values()))}, in {files}.")
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()
