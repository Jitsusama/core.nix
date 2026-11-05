"""Hold the shell's space to the theme's rungs (space-is-set-by-rung).

Reads every QML file under the modules folder given, tests aside, and fails,
naming each, on:

- a space named by its step on the size scale (Theme.space.l), where a rung
  says the level of structure it sits between (Theme.space.rung.items), the
  beat (Theme.space.beat) or a level's pitch (Theme.level.note.pitch) belongs,
  outside the files listed as not yet moved;
- a listed file that no longer names a step, or that isn't there, so the list
  only ever shrinks.

A step says how big a space is, and a rung why it's there, so two levels of
structure can't end up sharing one step (docs/design.md, Space).

Usage: space-is-set-by-rung.py MODULES UNMOVED
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


def listed(unmoved: Path) -> set[str]:
    """Read the files the list names, by their path under the modules folder."""
    names = (line.split("#")[0].strip() for line in unmoved.read_text().splitlines())
    return {name for name in names if name}


def main(modules: Path, unmoved: Path) -> int:
    """Check every file, printing what strays, and answer 1 if any does."""
    allowed = listed(unmoved)
    stray: list[str] = []
    stale: list[str] = []
    for path in sorted(modules.rglob("*.qml")):
        relative = path.relative_to(modules)
        if "tests" in relative.parts:
            continue
        name = relative.as_posix()
        found = steps(path, name)
        if name not in allowed:
            stray.extend(found)
        elif not found:
            stale.append(
                f"{name} is listed as not yet moved but names no step; take it off {unmoved.name}."
            )
    stale.extend(
        f"{name} is listed in {unmoved.name} but isn't there."
        for name in sorted(allowed)
        if not (modules / name).is_file()
    )
    for line in stray:
        print(line)
    if stray:
        print(
            "The lines above name a space by its step; name the rung it sits between, as "
            "Theme.space.rung.items, the beat or a level's pitch (docs/design.md, Space)."
        )
    for line in stale:
        print(line)
    return 1 if stray or stale else 0


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1]), Path(sys.argv[2])))
