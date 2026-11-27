"""Check that every file read whose failure turns into nothing says why.

    every-failed-read-says-why TREE WAITING

A program that reads a file lost, torn or barred as empty text counts what
it holds as none, so a measure reads as met, a list as answered or a run as
fine while nothing was looked at. A read may still take its failure as
nothing where that's true, as a file never written yet holds nothing, and
then it says so. This reads the Rust of TREE outside its tests for each
`fs::read_to_string` or `fs::read` whose failure its expression takes as
nothing, and such a read passes when a comment on its statement's lines, or
on a line of its own just above them, says why.

A read's expression is the chain of calls on its result, followed out of a
closure or call it sits in, up to a `?`, a `;`, a `,` or a block. It takes
a failure as nothing when a link of that chain, outside the arguments of
another, defaults the error (`unwrap_or_default`, `unwrap_or`,
`unwrap_or_else` unless it panics or exits with a code other than nought,
`map_or`, `map_or_else`, `or`), drops it (`ok`, `iter`, `into_iter`) or
judges it as false (`is_ok`, `is_ok_and`, `is_err`); when its statement
opens with `let Ok(`, as `if let` and `let else` do, but for a `let else`
whose block fails loudly or returns an error; or when a `match` on it has an
arm of its own taking any error (`Err(_)`, `Err(_name)`, `Err(..)` or
`_`). A `let else` says why with a comment on its block's first line too.
A default given in a later statement isn't seen. Test code (a tests
folder, a `tests.rs` or `*_tests.rs` file, an item marked `#[cfg(test)]`,
and the file of a module declared so) is left out, since a test's own
fixture missing fails the test anyway.

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
# The links of a read's chain that take its failure as nothing.
DEFAULTED = re.compile(
    r"\.(unwrap_or_default|unwrap_or|unwrap_or_else|map_or|map_or_else|or|ok|iter|into_iter"
    r"|is_ok|is_ok_and|is_err)\s*\("
)
# An `unwrap_or_else` that ends the program rather than giving a default;
# an exit is loud only with a code other than nought, since an exit of
# nought tells its caller the run was fine.
LOUD = re.compile(
    r"\.unwrap_or_else\(\s*\|[^|]*\|\s*\{?\s*(panic!|unreachable!|todo!"
    r"|(std::)?process::exit\(\s*([1-9]|-))"
)
LET_OK = re.compile(r"\blet\s+Ok\s*\(")
LET_ELSE = re.compile(r"\belse\s*$")
# A `let else` block that fails loudly or returns an error says it couldn't read.
FAILS = re.compile(
    r"\b(return\s+Err\s*\(|Err\s*\(|panic!|unreachable!|todo!|bail!"
    r"|(std::)?process::exit\(\s*([1-9]|-))"
)
MATCH = re.compile(r"\bmatch\b")
# A match arm taking any error as one more case.
ANY_ERROR = re.compile(
    r"\bErr\(\s*(_\w*|\.\.)\s*\)\s*(if\b(?:(?!=>).)*)?=>|(^|[{},])\s*_\s*(if\b(?:(?!=>).)*)?=>",
    re.DOTALL,
)
TEST_ITEM = re.compile(r"#\[cfg\(test\)\]")
ATTRIBUTE = re.compile(r"\s*#!?\[")
TEST_MODULE = re.compile(r"\s*(pub(\([^)]*\))?\s+)?mod\s+(\w+)\s*;")
RAW_STRING = re.compile(r'b?r(#*)"')
CHARACTER = re.compile(r"'(\\.[^']*|[^\\'])'")
OPENS = "([{"
CLOSES = ")]}"


def is_test(path: Path) -> bool:
    """Whether a Rust file is test code by its path: in a tests folder, or named as tests."""
    return "tests" in path.parts[:-1] or path.name == "tests.rs" or path.stem.endswith("_tests")


def lexed(text: str) -> tuple[str, set[int]]:
    """Blank a Rust file's comments and fill its strings and characters, keeping its lines.

    What's left is the code's brackets and marks where they were, so a
    bracket, a `;` or a `//` in a string can't move a statement's bounds.
    Beside it are the numbers of the lines holding a comment.
    """
    out: list[str] = []
    commented: set[int] = set()
    index = 0
    line = 1
    while index < len(text):
        if text.startswith(("//", "/*"), index):
            closer = "\n" if text.startswith("//", index) else "*/"
            end = text.find(closer, index + 2)
            end = len(text) if end < 0 else end + (0 if closer == "\n" else 2)
            commented.update(range(line, line + text.count("\n", index, end) + 1))
            out.append(re.sub(r"[^\n]", " ", text[index:end]))
        elif raw := RAW_STRING.match(text, index):
            close = '"' + raw.group(1)
            end = text.find(close, index + len(raw.group(0)))
            end = len(text) if end < 0 else end + len(close)
            out.append(re.sub(r"[^\n]", "s", text[index:end]))
        elif text[index] == '"':
            end = index + 1
            while end < len(text) and text[end] != '"':
                end += 2 if text[end] == "\\" else 1
            end += 1
            out.append(re.sub(r"[^\n]", "s", text[index:end]))
        elif literal := CHARACTER.match(text, index):
            end = index + len(literal.group(0))
            out.append("c" * (end - index))
        else:
            end = index + 1
            out.append(text[index])
        line += text.count("\n", index, end)
        index = end
    return "".join(out), commented


def closing(code: str, index: int) -> int:
    """Give the index just past the bracket closing the one at `index`."""
    depth = 0
    for at in range(index, len(code)):
        if code[at] in OPENS:
            depth += 1
        elif code[at] in CLOSES:
            depth -= 1
            if depth == 0:
                return at + 1
    return len(code)


def test_items(code: str) -> Iterator[tuple[int, int, str | None]]:
    """Give each item marked `#[cfg(test)]`: its bounds, and the module it declares, if any."""
    for found in TEST_ITEM.finditer(code):
        at = found.end()
        while attribute := ATTRIBUTE.match(code, at):
            at = closing(code, attribute.end() - 1)
        depth = 0
        end = len(code)
        for index in range(at, len(code)):
            char = code[index]
            if char == "{" and depth == 0:
                end = closing(code, index)
                break
            # A field, a variant or an argument ends at its comma, or at
            # the bracket closing what holds it.
            if char in ";," and depth == 0:
                end = index + 1
                break
            if char in CLOSES and depth == 0:
                end = index
                break
            depth += (char in OPENS) - (char in CLOSES)
        declared = TEST_MODULE.match(code[at:end])
        yield found.start(), end, declared.group(3) if declared else None


def module_folder(path: Path) -> Path:
    """Give the folder a Rust file's `mod x;` declarations find their files in."""
    if path.name in {"mod.rs", "main.rs", "lib.rs"}:
        return path.parent
    return path.parent / path.stem


def chain_end(code: str, index: int) -> int:
    """Give the index a read's expression ends at, from just past the read's call.

    It takes the calls on the result, and leaves a closure or call the read
    sits in to take that one's calls too, ending before a `?`, a `;`, a `,`
    or a block's bracket, so a `?` ends it propagating the error.
    """
    depth = 0
    for at in range(index, len(code)):
        char = code[at]
        if char in "?;," and depth == 0:
            return at
        if char == "{" and depth == 0:
            return at
        if char in OPENS:
            depth += 1
        elif char in CLOSES:
            if char == "}" and depth == 0:
                return at
            depth = max(depth - 1, 0)
    return len(code)


def outermost(code: str, opens: str = OPENS) -> str:
    """Blank what sits inside the brackets of `opens` a piece of code opens, keeping them.

    So a link of a read's chain is seen only where it is the chain's own,
    not in a closure or argument of another, and a match's arm only where
    it is the match's own, not one of a match in an arm's body. A bracket
    closing one opened before the piece is kept as it is.
    """
    closes = "".join(CLOSES[OPENS.index(bracket)] for bracket in opens)
    out: list[str] = []
    depth = 0
    for char in code:
        if char in closes and depth > 0:
            depth -= 1
        if depth > 0 and char != "\n":
            out.append(char if char in opens + closes else " ")
        else:
            out.append(char)
        if char in opens:
            depth += 1
    return "".join(out)


def statement_start(code: str, index: int) -> int:
    """Give the index the statement holding `index` starts at: past the last `;`, `{` or `}`."""
    starts = [code.rfind(mark, 0, index) for mark in ";{}"]
    return max(starts) + 1


def defaulted(code: str, read: re.Match[str]) -> tuple[int, int] | None:
    """Give the bounds of a read's statement when it takes the read's failure as nothing."""
    after = closing(code, read.end() - 1)
    end = chain_end(code, after)
    start = statement_start(code, read.start())
    head = code[start : read.start()]
    chain = outermost(LOUD.sub("", code[after:end]))
    if DEFAULTED.search(chain):
        return start, end
    if LET_OK.search(head) and code[end : end + 1] != "?":
        if code[end : end + 1] == "{" and LET_ELSE.search(code[after:end]):
            if FAILS.search(code[end : closing(code, end)]):
                return None
            # The block's first line is the statement's too, where a
            # `let else` says why.
            first = code.find("\n", code.find("\n", end) + 1)
            return start, len(code) if first < 0 else first
        return start, end
    in_a_match = MATCH.search(head) and code[end : end + 1] == "{"
    arms = outermost(code[end + 1 : closing(code, end) - 1], "{")
    if in_a_match and ANY_ERROR.search(arms):
        return start, end
    return None


def says_why(code: str, commented: set[int], start: int, end: int) -> bool:
    """Whether a comment sits on a statement's lines or on a line of its own just above it."""
    start = end - len(code[start:end].lstrip())
    first = code.count("\n", 0, start) + 1
    last = code.count("\n", 0, end) + 1
    above_ends = code.rfind("\n", 0, start)
    above = code[code.rfind("\n", 0, max(above_ends, 0)) + 1 : max(above_ends, 0)]
    alone = above_ends >= 0 and first - 1 in commented and not above.strip()
    return alone or any(line in commented for line in range(first, last + 1))


def defaulted_reads(text: str) -> tuple[list[tuple[int, bool]], list[str]]:
    """Give each read whose failure turns into nothing, with whether it says why.

    Beside them are the modules the file declares as test code.
    """
    code, commented = lexed(text)
    modules: list[str] = []
    for start, end, module in list(test_items(code)):
        code = code[:start] + re.sub(r"[^\n]", " ", code[start:end]) + code[end:]
        if module:
            modules.append(module)
    found: list[tuple[int, bool]] = []
    for read in READ.finditer(code):
        if bounds := defaulted(code, read):
            number = code.count("\n", 0, read.start()) + 1
            found.append((number, says_why(code, commented, *bounds)))
    return found, modules


def bare_reads(tree: Path) -> Iterator[tuple[str, int, str]]:
    """Give each defaulted read that doesn't say why: its file, line and code."""
    judged: dict[Path, tuple[str, list[tuple[int, bool]]]] = {}
    tested: list[Path] = []
    for path in sorted(tree.rglob("*.rs")):
        relative = path.relative_to(tree)
        if ".git" in relative.parts or not path.is_file() or is_test(relative):
            continue
        text = path.read_text(errors="replace")
        found, modules = defaulted_reads(text)
        judged[relative] = (text, found)
        tested.extend(module_folder(relative) / module for module in modules)
    for relative, (text, found) in judged.items():
        if any(relative.with_suffix("").is_relative_to(module) for module in tested):
            continue
        lines = text.splitlines()
        for number, reasoned in found:
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
                "on a line of its own just above it."
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
