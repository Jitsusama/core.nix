"""Check that every bound on how long a test's step took allows for how busy the host is.

    every-timed-step-allows-for-load TREE WAITING

A bound on a step's time, "it ended within two seconds", states what an idle host
takes. On a builder running sixteen builds every process waits its turn, and the
same step takes several times as long, so a bound stated at idle fails on a change
that never touched it. This reads the tests of TREE (Rust in a tests folder, and
Python in a tests folder or named as a test) for each upper bound on a time taken:
in Rust, `x.elapsed()` held under a duration, or past one in an `if`; in Python,
`time.monotonic() - x` or `time.perf_counter() - x` held under one. A bound passes
when it goes through the host's pace: in Rust, a step started with
`test_pace::Timed::start()` and judged with its `within`, which this never sees as
a bound; in Python, a bound multiplied by `vm_helpers.pace`, on the same line.

It reads one form a line at a time, so some bounds pass it unseen: a time taken held
in a name and compared on a later line, a deadline such as `Instant::now() <
deadline`, a Python time held past one in an `if`, and Rust tests inside a crate's
src folder. A bound that only tells right behaviour from wrong by time is judged by
its author, who holds it short of the wrong behaviour's time with `short_of`.

WAITING is JSON naming, for each file that held bounds at idle before the rule, how
many it holds while each comes to allow for the host's pace. A file holding more
fails, naming each; a file holding fewer fails until its count is lowered, so the
list only shrinks. Every failure is printed before the check exits 1.
"""

import json
import re
import sys
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

# Rust: a time taken held under something, or past something where an `if` decides
# what follows, such as a deadline that fails the test once it passes.
RUST_UNDER = re.compile(r"\.elapsed\(\)\s*<")
RUST_PAST = re.compile(r"\bif\b.*\.elapsed\(\)\s*>")
# Python: a clock read less a time it was read before, held under something.
PYTHON_UNDER = re.compile(r"\b(monotonic|perf_counter)\(\)\s*-\s*[\w.]+\s*<")
PYTHON_PACED = re.compile(r"\bpace\(")
PYTHON_TEST_NAMES = ("-test.py", "_test.py")


def rust_bounds(text: str) -> Iterator[int]:
    """Give the line of each bound at idle on a time taken, in Rust."""
    for number, line in enumerate(text.splitlines(), 1):
        code = line.split("//", 1)[0]
        if RUST_UNDER.search(code) or RUST_PAST.search(code):
            yield number


def python_bounds(text: str) -> Iterator[int]:
    """Give the line of each bound at idle on a time taken, in Python."""
    for number, line in enumerate(text.splitlines(), 1):
        code = line.split("#", 1)[0]
        if PYTHON_UNDER.search(code) and not PYTHON_PACED.search(code):
            yield number


def reader_for(path: Path) -> Callable[[str], Iterator[int]] | None:
    """Give the reader for a test file this check reads, or None for any other file."""
    if ".git" in path.parts:
        return None
    in_tests = "tests" in path.parts[:-1]
    if path.suffix == ".rs":
        return rust_bounds if in_tests else None
    named = path.name.endswith(PYTHON_TEST_NAMES) or path.name.startswith("test_")
    if path.suffix == ".py" and (in_tests or named):
        return python_bounds
    return None


def bounds_at_idle(tree: Path) -> Iterator[tuple[str, int, str]]:
    """Give each bound in a test that doesn't allow for the host's pace: its file, line and code."""
    for path in sorted(tree.rglob("*")):
        relative = path.relative_to(tree)
        reader = reader_for(relative)
        if reader is None or not path.is_file():
            continue
        text = path.read_text(errors="replace")
        lines = text.splitlines()
        for number in reader(text):
            yield relative.as_posix(), number, lines[number - 1].strip()


def bounds(count: int) -> str:
    """Say how many bounds there are at idle, in words that agree with the count."""
    return f"{count} bound{'' if count == 1 else 's'} at idle"


def main() -> None:
    """Say each file holding more bounds at idle than it waits with, and each holding fewer."""
    try:
        tree, waiting_file = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-timed-step-allows-for-load TREE WAITING")
    waiting: dict[str, int] = json.loads(waiting_file.read_text())
    found: dict[str, list[str]] = {}
    for path, number, code in bounds_at_idle(tree):
        found.setdefault(path, []).append(f"{path}:{number}: {code}")
    said: list[str] = []
    for path, held in sorted(found.items()):
        if len(held) > waiting.get(path, 0):
            said.append(
                f"{path} holds {bounds(len(held))}, {waiting.get(path, 0)} of them waiting: "
                "start the step with test_pace::Timed::start() and judge it with its within, "
                "or in Python multiply the bound by vm_helpers.pace(vm_helpers.busyness()). "
                "Where the wrong behaviour takes a fixed time, hold the bound short of it "
                "(Timed::start().short_of(wrong)), or it passes the wrong one when busy."
            )
            said.extend(f"  {bound}" for bound in held)
    said.extend(
        f"{path} waits with {bounds(count)} but holds {len(found.get(path, []))}: lower its "
        "count on the waiting list, or take it off."
        for path, count in sorted(waiting.items())
        if len(found.get(path, [])) < count
    )
    for failure in said:
        print(failure, file=sys.stderr)
    files = f"{len(found)} file{'' if len(found) == 1 else 's'}"
    print(f"{bounds(sum(len(held) for held in found.values()))}, in {files}.")
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()
