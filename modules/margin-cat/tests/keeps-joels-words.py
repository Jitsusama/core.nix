"""Check that every design Joel stated for the margin cat is held by a case, or owed.

    keeps-joels-words.py GUIDE TESTS

GUIDE is modules/margin-cat/AGENTS.md, whose "Joel's Words" section has a design a `###`
heading, each with a line beginning "Joel, YYYY-MM-DD" giving his words and when he said them,
then bullets, each either

    - Held by `FILE`: "PHRASE"
    - Owed since YYYY-MM-DD: what's still to do

TESTS is the folder of the cat's cases. A held PHRASE must be in FILE there, on a line that
isn't a comment, so a case's name or its code says it rather than a note beside it. A ruling on
the cat's design that would drop what Joel asked for then fails a named case first, and a
design with no case says so, dated. Every failure is printed before the check exits 1, and the
owed designs are printed either way, so the log says what's still to do.
"""

import re
import sys
from pathlib import Path

SECTION = "## Joel's Words"
SAID = re.compile(r"^Joel, \d{4}-\d{2}-\d{2}\b")
HELD = re.compile(r'^- Held by `(?P<file>[^`/]+)`: "(?P<phrase>[^"]+)"$')
OWED = re.compile(r"^- Owed since (?P<since>\d{4}-\d{2}-\d{2}): (?P<what>\S.*)$")
COMMENT = ("//", "#")


def designs(guide: str) -> dict[str, list[str]]:
    """Give each design's heading and its lines, a wrapped bullet joined to one line."""
    start = guide.index(SECTION) + len(SECTION)
    rest = guide[start:]
    end = re.search(r"^## ", rest, re.MULTILINE)
    section = rest[: end.start()] if end else rest
    found: dict[str, list[str]] = {}
    lines: list[str] | None = None
    for line in section.splitlines():
        if line.startswith("### "):
            lines = found.setdefault(line[4:].strip(), [])
        elif lines is not None and line.startswith("  ") and lines and lines[-1].startswith("- "):
            lines[-1] += " " + line.strip()
        elif lines is not None and line.strip():
            lines.append(line)
    return found


def said_in(tests: Path, name: str) -> set[str]:
    """Give the lines of a case file that aren't comments, or nothing for a file that's gone."""
    case = tests / name
    if not case.is_file():
        return set()
    return {
        line.strip()
        for line in case.read_text().splitlines()
        if not line.strip().startswith(COMMENT)
    }


def faults(heading: str, lines: list[str], tests: Path) -> tuple[list[str], list[str]]:
    """Give what's wrong with one design, and what it still owes."""
    wrong: list[str] = []
    owed: list[str] = []
    if not any(SAID.match(line) for line in lines):
        wrong.append(f'{heading}: no line says when Joel said it ("Joel, YYYY-MM-DD ...")')
    bullets = [line for line in lines if line.startswith("- ")]
    if not bullets:
        wrong.append(f"{heading}: no case holds it, and it isn't owed")
    for bullet in bullets:
        if held := HELD.match(bullet):
            phrase, name = held["phrase"], held["file"]
            if not any(phrase in line for line in said_in(tests, name)):
                wrong.append(f'{heading}: no case in {name} says "{phrase}"')
        elif owing := OWED.match(bullet):
            owed.append(f"{heading}, since {owing['since']}: {owing['what']}")
        else:
            wrong.append(f"{heading}: a bullet neither held nor owed: {bullet}")
    return wrong, owed


def main() -> int:
    """Check the guide's designs against the cases, printing every fault."""
    guide, tests = Path(sys.argv[1]), Path(sys.argv[2])
    found = designs(guide.read_text())
    wrong: list[str] = [] if found else [f"{guide}: no designs under {SECTION}"]
    owed: list[str] = []
    for heading, lines in found.items():
        more_wrong, more_owed = faults(heading, lines, tests)
        wrong += more_wrong
        owed += more_owed
    for one in owed:
        print(f"owed: {one}")
    for one in wrong:
        print(one)
    print(f"{len(found)} of Joel's designs, {len(owed)} owed, {len(wrong)} wrong")
    return 1 if wrong else 0


if __name__ == "__main__":
    sys.exit(main())
