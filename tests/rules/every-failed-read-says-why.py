"""Check that every file read whose failure turns into nothing says why.

    every-failed-read-says-why TREE WAITING

A program that reads a file lost, torn or barred as empty text counts what
it holds as none, so a measure reads as met, a list as answered or a run as
fine while nothing was looked at. A read may still take its failure as
nothing where that's true, as a file never written yet holds nothing, and
then it says so. This reads the Rust of TREE outside its tests for each
`fs::read_to_string` or `fs::read` whose statement turns an error into a
default (`unwrap_or_default()`, `unwrap_or_else(|_| ...)` or
`unwrap_or(String::new())`), and such a read passes when a comment says why:
on any line of its statement or just above it. Test code (a tests folder, a
`tests.rs` or `*_tests.rs` file, or what follows a file's `#[cfg(test)]`) is left out, since
a test's own fixture missing fails the test anyway.

WAITING is JSON naming, for each file that held such reads before the rule,
how many it holds while each says why or says it couldn't read. A file
holding more fails, naming each; a file holding fewer fails until its count
is lowered, so the list only shrinks. Every failure is printed before the
check exits 1.
"""

import json
import re
import sys
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Iterator

READ = re.compile(r"\bfs::(read_to_string|read)\s*\(")
# What turns a read's error into a default, and so into nothing.
DEFAULTED = re.compile(
    r"\.unwrap_or_default\(\)|\.unwrap_or_else\(\s*\|_\||\.unwrap_or\(\s*(String|Vec)::new\(\)\s*\)"
)
TESTS_FROM = re.compile(r"^\s*#\[cfg\(test\)\]")
OPENS = "([{"
CLOSES = ")]}"


def is_test(path: Path) -> bool:
    """Whether a Rust file is test code: in a tests folder, or named as tests."""
    return "tests" in path.parts[:-1] or path.name == "tests.rs" or path.stem.endswith("_tests")


def code_of(line: str) -> str:
    """Give a line's code, without its comment."""
    return line.split("//", 1)[0]


def statement_end(lines: list[str], number: int, column: int) -> int:
    """Give the line a read's statement ends on, from its call at `column` of line `number`.

    It ends at a `;` or `,` outside the read's own brackets, or where a bracket
    it sits inside closes, as a closure's body or a call's argument does.
    """
    depth = 0
    for at in range(number, len(lines) + 1):
        code = code_of(lines[at - 1])
        for char in code[column if at == number else 0 :]:
            if char in OPENS:
                depth += 1
            elif char in CLOSES:
                depth -= 1
                if depth < 0:
                    return at
            elif char in ";," and depth == 0:
                return at
    return len(lines)


def statement_start(lines: list[str], number: int) -> int:
    """Give the line the statement holding line `number` starts on.

    A line before it is the statement's while that line's code doesn't end
    one, or open or close a block.
    """
    start = number
    while start > 1:
        above = code_of(lines[start - 2]).rstrip()
        if not above or above.endswith((";", "{", "}", ",")):
            break
        start -= 1
    return start


def commented(lines: list[str], first: int, last: int) -> bool:
    """Whether any line from `first` to `last`, or the line just above, holds a comment."""
    return any("//" in lines[at - 1] for at in range(max(1, first - 1), last + 1))


def defaulted_reads(text: str) -> Iterator[tuple[int, bool]]:
    """Give each read whose error turns into a default: its line, and whether it says why."""
    lines = text.splitlines()
    for number, line in enumerate(lines, 1):
        if TESTS_FROM.match(line):
            return
        for read in READ.finditer(code_of(line)):
            # From the read's opening bracket, so its own arguments close inside it.
            last = statement_end(lines, number, read.end() - 1)
            statement = "\n".join(code_of(lines[at - 1]) for at in range(number, last + 1))
            if not DEFAULTED.search(statement[read.start() :]):
                continue
            yield number, commented(lines, statement_start(lines, number), last)


def bare_reads(tree: Path) -> Iterator[tuple[str, int, str]]:
    """Give each defaulted read that doesn't say why: its file, line and code."""
    for path in sorted(tree.rglob("*.rs")):
        relative = path.relative_to(tree)
        if ".git" in relative.parts or not path.is_file() or is_test(relative):
            continue
        text = path.read_text(errors="replace")
        lines = text.splitlines()
        for number, reasoned in defaulted_reads(text):
            if not reasoned:
                yield relative.as_posix(), number, lines[number - 1].strip()


def reads(count: int) -> str:
    """Say how many failed reads go unsaid, in words that agree with the count."""
    return f"{count} read{'' if count == 1 else 's'} whose failure is taken as nothing"


def main() -> None:
    """Say each file holding more unsaid failed reads than it waits with, and each holding fewer."""
    try:
        tree, waiting_file = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-failed-read-says-why TREE WAITING")
    waiting: dict[str, int] = json.loads(waiting_file.read_text())
    found: dict[str, list[str]] = {}
    for path, number, code in bare_reads(tree):
        found.setdefault(path, []).append(f"{path}:{number}: {code}")
    said: list[str] = []
    for path, held in sorted(found.items()):
        if len(held) > waiting.get(path, 0):
            said.append(
                f"{path} holds {reads(len(held))}, {waiting.get(path, 0)} of them waiting: say "
                "that it couldn't read, as an error or a measure left unjudged, or, where a "
                "file not there truly holds nothing, say why in a comment on its statement or "
                "just above it."
            )
            said.extend(f"  {read}" for read in held)
    said.extend(
        f"{path} waits with {reads(count)} but holds {len(found.get(path, []))}: lower its "
        "count on the waiting list, or take it off."
        for path, count in sorted(waiting.items())
        if len(found.get(path, [])) < count
    )
    for failure in said:
        print(failure, file=sys.stderr)
    files = f"{len(found)} file{'' if len(found) == 1 else 's'}"
    print(f"{reads(sum(len(held) for held in found.values()))}, in {files}.")
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()
