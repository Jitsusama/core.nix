"""Hold the shell's space to the theme's rungs (space-is-set-by-rung).

Reads every QML file under the modules folder given, tests aside, and fails,
naming each, on a space named by its step on the size scale (Theme.space.l),
where a rung says the level of structure it sits between
(Theme.space.rung.items), the beat (Theme.space.beat) or a level's pitch
(Theme.level.note.pitch) belongs.

A step says how big a space is, and a rung why it's there, so two levels of
structure can't end up sharing one step (docs/design.md, Space).

Usage: space-is-set-by-rung.py MODULES
"""

import re
import sys
from pathlib import Path

# A step of the size scale, read straight from the theme.
STEP = re.compile(r"\bTheme\.space\.(?:xs|s|m|l|xl|xxl|xxxl|huge)\b")


def code(line: str) -> str:
    """Give the line without a trailing comment, which says why rather than sets."""
    return re.sub(r"(^|\s)//.*$", "", line)


def steps(path: Path, name: str) -> list[str]:
    """Find each line in a file that names a step, saying where."""
    lines = path.read_text(encoding="utf-8").splitlines()
    return [
        f"{name}:{number + 1}: {line.strip()}"
        for number, line in enumerate(lines)
        if STEP.search(code(line))
    ]


def main(modules: Path) -> int:
    """Check every file, printing what strays, and answer 1 if any does."""
    stray: list[str] = []
    for path in sorted(modules.rglob("*.qml")):
        relative = path.relative_to(modules)
        if "tests" in relative.parts:
            continue
        stray.extend(steps(path, relative.as_posix()))
    for line in stray:
        print(line)
    if stray:
        print(
            "The lines above name a space by its step; name the rung it sits between, as "
            "Theme.space.rung.items, the beat or a level's pitch (docs/design.md, Space)."
        )
    return 1 if stray else 0


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1])))
