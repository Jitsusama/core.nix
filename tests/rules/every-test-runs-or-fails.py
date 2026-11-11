"""Check that a test that can't run fails, rather than passing having proved nothing.

    every-test-runs-or-fails TREE WAITING

A test that can't run, for want of a filesystem, a device or a tool, fails the
gate instead of returning early, since a skip nobody sees passes a gate that
proved nothing. This reads the tests of TREE for each way one bows out: in
Rust, an `#[ignore]` that gives no reason, a line printing that it skipped,
and, in a tests folder's file holding a test, a `let ... else { return }`;
in Python, pytest's and
unittest's skips and expected failures; in QML, a TestCase's `skip(`. An
`#[ignore = "..."]` passes, since its reason names what runs it instead.

WAITING is JSON naming, for each file that held such skips before the rule,
how many it holds while each becomes a test that runs where it can and fails
where it can't. A file holding more fails, naming each; a file holding fewer
fails until its count is lowered, so the list only shrinks. Every failure is
printed before the check exits 1.
"""

import json
import re
import sys
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Iterator

# What bows a test out, by the kind of file it's in. A Rust test folder's early
# return from a `let ... else` is read only in a file holding a test, since code
# beside it, a stand-in's included, uses the same shape for what it's meant to
# do.
SKIPS = {
    ".rs": re.compile(r'#\[ignore\]|\be?println!\(\s*"(?i:skip)'),
    ".py": re.compile(
        r"\bpytest\.(skip|xfail|importorskip)\(|\bpytest\.mark\.(skip|skipif|xfail)\b"
        r"|\bunittest\.(skip|skipIf|skipUnless|expectedFailure)\b"
    ),
    ".qml": re.compile(r"\bskip\("),
}
RETURNS = re.compile(r"\belse\s*\{\s*return;?\s*\}")
# A Rust test, by `#[test]` or a runtime's own, such as `#[tokio::test]`.
RUST_TEST = re.compile(r"#\[(\w+::)*test\b")


def is_test(path: Path) -> bool:
    """Whether a file holds tests: in a tests folder, or a Rust file with a test in it."""
    return "tests" in path.parts[:-1] or path.suffix == ".rs"


def skips(path: Path, text: str) -> Iterator[int]:
    """Give the line of each way a test bows out, in a file of tests."""
    in_tests = "tests" in path.parts[:-1]
    tested = path.suffix == ".rs" and RUST_TEST.search(text) is not None
    if path.suffix == ".rs" and not in_tests and not tested:
        return
    for number, line in enumerate(text.splitlines(), 1):
        code = line.split("//" if path.suffix in {".rs", ".qml"} else "#", 1)[0]
        returns = tested and in_tests and RETURNS.search(code)
        if SKIPS[path.suffix].search(code) or returns:
            yield number


def quiet_skips(tree: Path) -> Iterator[tuple[str, int, str]]:
    """Give each way a test bows out without failing: its file, line and code."""
    for path in sorted(tree.rglob("*")):
        relative = path.relative_to(tree)
        if ".git" in relative.parts or path.suffix not in SKIPS or not is_test(relative):
            continue
        if not path.is_file():
            continue
        text = path.read_text(errors="replace")
        lines = text.splitlines()
        for number in skips(relative, text):
            yield relative.as_posix(), number, lines[number - 1].strip()


def skips_said(count: int) -> str:
    """Say how many skips there are, in words that agree with the count."""
    return f"{count} skip{'' if count == 1 else 's'} passing a test that didn't run"


def main() -> None:
    """Say each file holding more skips than it waits with, and each holding fewer."""
    try:
        tree, waiting_file = (Path(arg) for arg in sys.argv[1:])
    except ValueError:
        sys.exit("usage: every-test-runs-or-fails TREE WAITING")
    waiting: dict[str, int] = json.loads(waiting_file.read_text())
    found: dict[str, list[str]] = {}
    for path, number, code in quiet_skips(tree):
        found.setdefault(path, []).append(f"{path}:{number}: {code}")
    said: list[str] = []
    for path, held in sorted(found.items()):
        if len(held) > waiting.get(path, 0):
            said.append(
                f"{path} holds {skips_said(len(held))}, {waiting.get(path, 0)} of them waiting: "
                "fail where the test can't run, run it where it can, such as in a VM, or give "
                'an #[ignore = "..."] naming what runs it instead.'
            )
            said.extend(f"  {line}" for line in held)
    said.extend(
        f"{path} waits with {skips_said(count)} but holds {len(found.get(path, []))}: lower its "
        "count on the waiting list, or take it off."
        for path, count in sorted(waiting.items())
        if len(found.get(path, [])) < count
    )
    for failure in said:
        print(failure, file=sys.stderr)
    files = f"{len(found)} file{'' if len(found) == 1 else 's'}"
    print(f"{skips_said(sum(len(held) for held in found.values()))}, in {files}.")
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()
