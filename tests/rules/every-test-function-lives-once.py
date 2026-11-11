"""Check that no function in test code is copied from one file into another.

    every-test-function-lives-once TREE WAITING

A test's function, fake or fixture copied into a second file drifts from the
first the next time one of them is fixed, and a reader can't tell which copy is
right. This reads TREE's test code, under a `tests` folder or in a file named
as a test, in Python, shell, QML, JavaScript and Rust, and gathers each
function's body of at least MINIMUM lines, with its blank lines and comments
left out and its indentation evened. A body found in more than one file is a
copy in each of them, whatever each copy is named. Python's are compared by
their syntax trees; the others by their lines, from a signature that fits on
its first line to the brace closing it at the same indentation.

WAITING is JSON naming, for each file that held copies before the rule, how
many it holds while each moves to the one place both can import. A file
holding more fails, naming each and where else it lives; a file holding fewer
fails until its count is lowered, so the list only shrinks. Every failure is
printed before the check exits 1.
"""

import ast
import hashlib
import json
import re
import sys
from pathlib import Path

# The fewest lines a body needs to count, so a one-line call written the same
# way twice isn't taken for a copy.
MINIMUM = 2
# A test file's name, for test code outside a tests folder.
TEST_NAME = re.compile(r"(^test_|[-_]test\.|-tests?\.|^tests?\.)")
SKIPPED = {".git", "node_modules", "target", "result"}
SHELL_START = re.compile(r"^\s*(?:function\s+)?([A-Za-z_][\w-]*)\s*\(\)\s*\{\s*$")
BRACED_START = {
    ".qml": re.compile(r"^\s*function\s+(\w+)\s*\([^)]*\)\s*\{\s*$"),
    ".js": re.compile(r"^\s*(?:export\s+)?function\s+(\w+)\s*\([^)]*\)\s*\{\s*$"),
    ".mjs": re.compile(r"^\s*(?:export\s+)?function\s+(\w+)\s*\([^)]*\)\s*\{\s*$"),
    ".rs": re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)\b.*\{\s*$"),
}
COMMENT = {".sh": "#", ".qml": "//", ".js": "//", ".mjs": "//", ".rs": "//"}


def normal(lines: list[str], mark: str) -> list[str]:
    """Give a body's lines without blanks or whole-line comments, each stripped."""
    return [line.strip() for line in lines if line.strip() and not line.strip().startswith(mark)]


def braced(lines: list[str], start: re.Pattern[str], mark: str) -> list[tuple[str, int, str]]:
    """Give each braced function's name, line and body, its end found by its indentation."""
    found = []
    for index, line in enumerate(lines):
        opened = start.match(line)
        if not opened:
            continue
        indent = len(line) - len(line.lstrip())
        for end in range(index + 1, len(lines)):
            closing = lines[end]
            if closing.strip().startswith("}") and len(closing) - len(closing.lstrip()) == indent:
                body = normal(lines[index + 1 : end], mark)
                found.append((opened.group(1), index + 1, "\n".join(body)))
                break
    return found


def python(text: str) -> list[tuple[str, int, str]]:
    """Give each Python function's name, line and body, as its syntax tree says it."""
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return []
    found = []
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            body = node.body
            if body and isinstance(body[0], ast.Expr) and isinstance(body[0].value, ast.Constant):
                body = body[1:]
            lines = (body[-1].end_lineno or 0) - body[0].lineno + 1 if body else 0
            if lines >= MINIMUM:
                dumped = "\n".join(ast.dump(statement) for statement in body)
                found.append((node.name, node.lineno, f"{lines}\n{dumped}"))
    return found


def functions(path: Path) -> list[tuple[str, int, str]]:
    """Give each function in a file with a body long enough to count."""
    text = path.read_text(errors="replace")
    if path.suffix == ".py":
        return python(text)
    lines = text.splitlines()
    if path.suffix == ".sh":
        found = braced(lines, SHELL_START, "#")
    else:
        found = braced(lines, BRACED_START[path.suffix], COMMENT[path.suffix])
    return [entry for entry in found if entry[2].count("\n") + 1 >= MINIMUM]


def copies(tree: Path) -> dict[str, list[str]]:
    """Give each file's functions whose bodies live in another file too."""
    where: dict[str, list[tuple[str, str, int]]] = {}
    for path in sorted(tree.rglob("*")):
        relative = path.relative_to(tree)
        if SKIPPED & set(relative.parts) or not path.is_file():
            continue
        if path.suffix not in {".py", ".sh", *BRACED_START}:
            continue
        if not ({"tests", "test"} & set(relative.parts) or TEST_NAME.search(path.name)):
            continue
        for name, line, body in functions(path):
            digest = hashlib.sha256(f"{path.suffix}\n{body}".encode()).hexdigest()
            where.setdefault(digest, []).append((relative.as_posix(), name, line))
    found: dict[str, list[str]] = {}
    for places in where.values():
        files = {file for file, _, _ in places}
        if len(files) < 2:  # noqa: PLR2004, a copy needs a second file
            continue
        for file, name, line in places:
            elsewhere = sorted(f"{other}:{at}" for other, _, at in places if other != file)
            found.setdefault(file, []).append(
                f"{file}:{line}: {name}, as at {', '.join(elsewhere)}"
            )
    return found


def copied(count: int) -> str:
    """Say how many copied functions there are, in words that agree with the count."""
    return f"{count} copied function{'' if count == 1 else 's'}"


def main() -> None:
    """Say each file holding more copied functions than it waits with, and each holding fewer."""
    try:
        tree, waiting_file = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-test-function-lives-once TREE WAITING")
    waiting: dict[str, int] = json.loads(waiting_file.read_text())
    found = copies(tree)
    said: list[str] = []
    for path, held in sorted(found.items()):
        if len(held) > waiting.get(path, 0):
            said.append(
                f"{path} holds {copied(len(held))}, {waiting.get(path, 0)} of them waiting: keep "
                "each once, in the test library every file using it imports."
            )
            said.extend(f"  {line}" for line in held)
    said.extend(
        f"{path} waits with {copied(count)} but holds {len(found.get(path, []))}: lower its "
        "count on the waiting list, or take it off."
        for path, count in sorted(waiting.items())
        if len(found.get(path, [])) < count
    )
    for failure in said:
        print(failure, file=sys.stderr)
    files = f"{len(found)} file{'' if len(found) == 1 else 's'}"
    print(f"{copied(sum(len(held) for held in found.values()))}, in {files}.")
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()
