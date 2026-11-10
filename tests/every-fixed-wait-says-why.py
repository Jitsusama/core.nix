"""Check that every sleep in a test says what it waits through, or polls.

    every-fixed-wait-says-why TREE WAITING

A test waits on the event it needs, since a fixed sleep is too short on a busy
machine and too long on an idle one. A fixed sleep stands only for "nothing
happens for this long", and says so. This reads the tests of TREE (Python and
shell in a tests folder or named as a test, and Rust in a tests folder) for
each sleep, and a sleep passes when it polls, inside a loop that looks again,
or says why: in a comment on its line or just above it, or by sleeping for a
named constant, such as `time.sleep(UNHEARD)`, whose name says what passes.
A stand-in's `sleep infinity` is a process that stays, so it isn't counted.

WAITING is JSON naming, for each file that held sleeps without a reason before
the rule, how many it holds while each says why or becomes a wait on its
event. A file holding more fails, naming each; a file holding fewer fails until
its count is lowered, so the list only shrinks. Every failure is printed before
the check exits 1.
"""

import ast
import json
import re
import sys
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Iterator

# A name in capitals, as a constant's: one that says what a sleep waits through.
CONSTANT = re.compile(r"^[A-Z][A-Z0-9_]+$")
COMMENT = {".py": "#", ".sh": "#", ".rs": "//"}
TEST_NAMES = ("-test.py", "_test.py", "-test.sh", "_test.sh")


def is_test(path: Path) -> bool:
    """Whether a file is a test: in a tests folder, or named as one."""
    if path.suffix == ".rs":
        return "tests" in path.parts[:-1]
    return (
        "tests" in path.parts[:-1]
        or path.name.endswith(TEST_NAMES)
        or path.name.startswith("test_")
    )


def python_sleeps(text: str) -> Iterator[tuple[int, bool]]:
    """Give each Python sleep's line, and whether it polls or sleeps for a constant."""
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return

    def walk(node: ast.AST, *, looped: bool) -> Iterator[tuple[int, bool]]:
        for child in ast.iter_child_nodes(node):
            inner = looped or isinstance(child, (ast.While, ast.For, ast.AsyncFor))
            if isinstance(child, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)):
                inner = False
            if isinstance(child, ast.Call):
                called = child.func
                name = called.attr if isinstance(called, ast.Attribute) else ""
                name = called.id if isinstance(called, ast.Name) else name
                if name == "sleep":
                    named = any(
                        CONSTANT.match(part.id if isinstance(part, ast.Name) else part.attr)
                        for argument in [*child.args, *(word.value for word in child.keywords)]
                        for part in ast.walk(argument)
                        if isinstance(part, (ast.Name, ast.Attribute))
                    )
                    yield child.lineno, looped or named
            yield from walk(child, looped=inner)

    yield from walk(tree, looped=False)


def shell_sleeps(text: str) -> Iterator[tuple[int, bool]]:
    """Give each shell sleep's line, and whether it polls or sleeps for a constant."""
    loops = 0
    for number, line in enumerate(text.splitlines(), 1):
        code = line.split("#", 1)[0]
        loops += len(re.findall(r"(^|[;&|]|\s)(while|until|for)\s", code))
        sleep = re.search(r"(^|[;&|(]|\s)sleep\s+(\S+)", code)
        # A stand-in's `sleep infinity` is a process that stays, not a wait.
        if sleep and sleep.group(2) != "infinity":
            named = CONSTANT.match(sleep.group(2).strip('${}"')) is not None
            yield number, loops > 0 or named
        loops = max(0, loops - len(re.findall(r"\bdone\b", code)))


def rust_sleeps(text: str) -> Iterator[tuple[int, bool]]:
    """Give each Rust sleep's line, and whether it polls or sleeps for a constant.

    A block counts as a loop's when `loop`, `while` or `for` came before its
    brace, so a sleep inside one, however deep, polls.
    """
    blocks: list[bool] = []
    looping = False
    for number, line in enumerate(text.splitlines(), 1):
        code = line.split("//", 1)[0]
        sleep = re.search(r"\bsleep\((.*)", code)
        if sleep:
            named = re.search(r"\b[A-Z][A-Z0-9_]+\b", sleep.group(1)) is not None
            yield number, any(blocks) or named
        for token in re.finditer(r"\b(loop|while|for)\b|[{}]", code):
            if token.group(0) == "{":
                blocks.append(looping)
                looping = False
            elif token.group(0) == "}":
                if blocks:
                    blocks.pop()
            else:
                looping = True


READERS = {".py": python_sleeps, ".sh": shell_sleeps, ".rs": rust_sleeps}


def commented(lines: list[str], number: int, mark: str) -> bool:
    """Whether a sleep's line, or the line just above it, holds a comment."""
    above = lines[number - 2].strip() if number > 1 else ""
    return mark in lines[number - 1] or (above.startswith(mark) and not above.startswith("#!"))


def bare_sleeps(tree: Path) -> Iterator[tuple[str, int, str]]:
    """Give each sleep in a test that neither polls nor says why: its file, line and code."""
    for path in sorted(tree.rglob("*")):
        relative = path.relative_to(tree)
        if ".git" in relative.parts or path.suffix not in READERS or not is_test(relative):
            continue
        if not path.is_file():
            continue
        text = path.read_text(errors="replace")
        lines = text.splitlines()
        for number, reasoned in READERS[path.suffix](text):
            if reasoned or commented(lines, number, COMMENT[path.suffix]):
                continue
            yield relative.as_posix(), number, lines[number - 1].strip()


def sleeps(count: int) -> str:
    """Say how many sleeps there are without a reason, in words that agree with the count."""
    return f"{count} sleep{'' if count == 1 else 's'} without a reason"


def main() -> None:
    """Say each file holding more bare sleeps than it waits with, and each holding fewer."""
    try:
        tree, waiting_file = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-fixed-wait-says-why TREE WAITING")
    waiting: dict[str, int] = json.loads(waiting_file.read_text())
    found: dict[str, list[str]] = {}
    for path, number, code in bare_sleeps(tree):
        found.setdefault(path, []).append(f"{path}:{number}: {code}")
    said: list[str] = []
    for path, held in sorted(found.items()):
        if len(held) > waiting.get(path, 0):
            said.append(
                f"{path} holds {sleeps(len(held))}, {waiting.get(path, 0)} of them waiting: wait "
                "on the event instead, or say what passes while it sleeps, in a comment on its "
                "line or just above it, or by sleeping for a named constant."
            )
            said.extend(f"  {sleep}" for sleep in held)
    said.extend(
        f"{path} waits with {sleeps(count)} but holds {len(found.get(path, []))}: lower its "
        "count on the waiting list, or take it off."
        for path, count in sorted(waiting.items())
        if len(found.get(path, [])) < count
    )
    for failure in said:
        print(failure, file=sys.stderr)
    files = f"{len(found)} file{'' if len(found) == 1 else 's'}"
    print(f"{sleeps(sum(len(held) for held in found.values()))}, in {files}.")
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()
