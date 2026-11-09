"""Check that no VM test grows past the pyramid's twenty subtests.

    every-vm-test-keeps-to-the-pyramid TESTS OVER

TESTS is a folder holding VM tests' files, each named *-vm-test.py at any
depth, whose top-level test_ functions are the subtests pytest runs. OVER
lists the tests already past the limit, a line each: the file's path within
TESTS, a space, and how many subtests it holds, with # starting a comment.
docs/testing.md says why: a VM test past about twenty subtests splits by area,
and its many cases go in the tiers below.

A file not listed with more than twenty subtests fails, and so does a listed
one holding more than its count, so none grows. One listed holding fewer
fails too, saying its new count, or that it's off the list once it's within
the limit, and so does one listed that's gone: the list only shrinks. Every
failure is printed before the check exits 1.
"""

import ast
import sys
from pathlib import Path

LIMIT = 20


def subtests(test: Path) -> int:
    """How many test functions a VM test's file holds at its top level."""
    tree = ast.parse(test.read_text(), filename=str(test))
    return sum(
        1
        for node in tree.body
        if isinstance(node, ast.FunctionDef | ast.AsyncFunctionDef)
        and node.name.startswith("test_")
    )


def listed(over: Path) -> dict[str, int]:
    """Give the tests past the limit today, by path, with how many subtests each holds."""
    counts: dict[str, int] = {}
    for line in over.read_text().splitlines():
        words = line.split("#", 1)[0].split()
        if not words:
            continue
        if len(words) != 2 or not words[1].isdigit():  # noqa: PLR2004
            sys.exit(f"{over}: '{line}' isn't a path and a count.")
        counts[words[0]] = int(words[1])
    return counts


def faults(counts: dict[str, int], over: dict[str, int]) -> list[str]:
    """Say what breaks the limit or the list, a sentence each."""
    said = []
    for path, count in sorted(counts.items()):
        allowed = over.get(path)
        if allowed is None:
            if count > LIMIT:
                said.append(
                    f"{path} holds {count} subtests, past {LIMIT}: split it by area, "
                    "and move its many cases to the tiers below (docs/testing.md)."
                )
        elif count > allowed:
            said.append(
                f"{path} holds {count} subtests, more than the {allowed} it's listed with: "
                "a case added here goes in a tier below, or the test splits."
            )
        elif count <= LIMIT:
            said.append(f"{path} holds {count} subtests, within {LIMIT}: take it off the list.")
        elif count < allowed:
            said.append(f"{path} holds {count} subtests now: list it with {count}.")
    said.extend(
        f"{path} is listed but isn't there: take it off the list."
        for path in sorted(over.keys() - counts.keys())
    )
    return said


def main() -> None:
    """Count every VM test's subtests and say each fault."""
    tests, over = Path(sys.argv[1]), Path(sys.argv[2])
    counts = {str(test.relative_to(tests)): subtests(test) for test in tests.rglob("*-vm-test.py")}
    if not counts:
        sys.exit(f"{tests} holds no VM tests.")
    said = faults(counts, listed(over))
    for fault in said:
        print(fault)
    if said:
        sys.exit(1)


if __name__ == "__main__":
    main()
