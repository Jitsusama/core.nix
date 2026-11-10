"""Check that every lint suppression says why, where it's made.

    every-suppression-says-why TREE WAITING

A suppression is a judgement that the linter is wrong here, and a judgement
with no reason can't be checked when the code or the linter changes. This
reads TREE for each suppression the linters themselves don't hold to a reason:
Python's `# noqa` and `# type: ignore`, shell's `# shellcheck disable=` and
QML's `// qmllint disable`. One passes when its line says more after the codes
it names, or when a comment stands on the line just above it. Rust's
`#[allow]` and `#[expect]` aren't read, since clippy refuses either without a
`reason =`, and neither is Biome's `biome-ignore`, which Biome refuses without
an explanation.

WAITING is JSON naming, for each file that held suppressions without a reason
before the rule, how many it holds while each gains one. A file holding more
fails, naming each; a file holding fewer fails until its count is lowered, so
the list only shrinks. Every failure is printed before the check exits 1.
"""

import json
import re
import sys
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Iterator

# Each suppression, by the kind of file it's in, with what it names after it:
# the codes, which aren't a reason, then whatever follows them, which is.
CODES = r"(?:[:=]?\s*[A-Za-z]+[0-9]*(?:\s*,\s*[A-Za-z]+[0-9]*)*)?"
SUPPRESSIONS = {
    ".py": re.compile(rf"#\s*(?:noqa|type:\s*ignore(?:\[[^\]]*\])?){CODES}(?P<after>.*)$"),
    ".sh": re.compile(r"#\s*shellcheck\s+disable=[A-Z0-9,]+(?P<after>.*)$"),
    ".qml": re.compile(rf"//\s*qmllint\s+disable{CODES}(?P<after>.*)$"),
}
COMMENT = {".py": "#", ".sh": "#", ".qml": "//"}


def reasoned(lines: list[str], number: int, after: str, mark: str) -> bool:
    """Whether a suppression says why: after its codes, or in a comment just above it."""
    if after.strip(" ,;:#-").strip():
        return True
    above = lines[number - 2].strip() if number > 1 else ""
    # Another directive above isn't a reason, but a sentence naming the linter is.
    directive = re.match(
        r"(#|//)\s*(shellcheck\s+\w+=|qmllint\s+(disable|enable)|noqa\b|type:\s*ignore)", above
    )
    return above.startswith(mark) and not above.startswith("#!") and not directive


def bare_suppressions(tree: Path) -> Iterator[tuple[str, int, str]]:
    """Give each suppression that doesn't say why: its file, line and code."""
    for path in sorted(tree.rglob("*")):
        relative = path.relative_to(tree)
        if ".git" in relative.parts or path.suffix not in SUPPRESSIONS or not path.is_file():
            continue
        lines = path.read_text(errors="replace").splitlines()
        for number, line in enumerate(lines, 1):
            found = SUPPRESSIONS[path.suffix].search(line)
            mark = COMMENT[path.suffix]
            if found and not reasoned(lines, number, found.group("after"), mark):
                yield relative.as_posix(), number, line.strip()


def suppressions(count: int) -> str:
    """Say how many suppressions don't say why, in words that agree with the count."""
    return f"{count} suppression{'' if count == 1 else 's'} without a reason"


def main() -> None:
    """Say each file holding more bare suppressions than it waits with, and each holding fewer."""
    try:
        tree, waiting_file = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-suppression-says-why TREE WAITING")
    waiting: dict[str, int] = json.loads(waiting_file.read_text())
    found: dict[str, list[str]] = {}
    for path, number, code in bare_suppressions(tree):
        found.setdefault(path, []).append(f"{path}:{number}: {code}")
    said: list[str] = []
    for path, held in sorted(found.items()):
        if len(held) > waiting.get(path, 0):
            said.append(
                f"{path} holds {suppressions(len(held))}, {waiting.get(path, 0)} of them "
                "waiting: say why after the codes it names, or in a comment on the line above."
            )
            said.extend(f"  {line}" for line in held)
    said.extend(
        f"{path} waits with {suppressions(count)} but holds {len(found.get(path, []))}: lower "
        "its count on the waiting list, or take it off."
        for path, count in sorted(waiting.items())
        if len(found.get(path, [])) < count
    )
    for failure in said:
        print(failure, file=sys.stderr)
    files = f"{len(found)} file{'' if len(found) == 1 else 's'}"
    print(f"{suppressions(sum(len(held) for held in found.values()))}, in {files}.")
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()
