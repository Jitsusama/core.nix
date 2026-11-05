"""Hold the shell's type to the theme's levels (type-is-set-by-level).

Reads every QML file under the modules folder given, tests aside, and fails,
naming each, on:

- a font's size, weight, axes, letter spacing or case set other than from a
  level (Theme.level.name.pointSize, or a property holding a level, such as
  row.level.pointSize), outside the files listed as not yet moved;
- a listed file that no longer sets any of them other than from a level, or
  that isn't there, so the list only ever shrinks.

A setting mixing a level with a step, a font constant, a number or an object
of its own is set by neither, and fails as well.

Usage: type-is-set-by-level.py MODULES UNMOVED
"""

import re
import sys
from pathlib import Path

# The font properties a level owns; family and features go with them, but
# naming a face or a feature is no size or weight.
OWNED = "pointSize|pixelSize|weight|variableAxes|letterSpacing|capitalization"

# A binding, `font.pointSize: ...`, or a script's assignment,
# `font.pointSize = ...`.
SETTING = re.compile(rf"\bfont\.({OWNED})\s*(?::|=(?!=))\s*(.*)")

# A level read: Theme.level.name.pointSize, or a property holding one.
LEVEL = re.compile(r"\b(?:level|\w*Level)\.[A-Za-z]")

# What sets type other than by a level: the scale's steps, the serif's axes
# and the eyebrow's tracking, Qt's own weights and cases, a number, an
# object of axes.
RAW = re.compile(r"\bTheme\.(?:mono|serif|sans|eyebrow)\b|\bFont\.\w+|(?<![\w.])\d+(?:\.\d+)?|\{")

# A line ending on one of these, or the next starting on one, carries the
# same expression on.
CARRIED_ON = ("?", ":", "+", "-", "*", "/", "&&", "||", "(", ",")


def code(line: str) -> str:
    """Give the line without a trailing comment."""
    return re.sub(r"\s//.*$", "", line).strip()


def expression(lines: list[str], start: int, rest: str) -> str:
    """Gather the whole expression a setting starts with, over its lines."""
    parts = [code(rest)]
    at = start
    while at + 1 < len(lines):
        following = code(lines[at + 1])
        if not (parts[-1].endswith(CARRIED_ON) or following.startswith(CARRIED_ON)):
            break
        at += 1
        parts.append(following)
    return " ".join(parts)


def raw_settings(modules: Path, name: str) -> list[str]:
    """Find each setting in a file not read from a level, saying where."""
    lines = (modules / name).read_text(encoding="utf-8").splitlines()
    found = []
    for number, line in enumerate(lines):
        match = SETTING.search(code(line))
        if match is None:
            continue
        setting = expression(lines, number, match.group(2))
        if LEVEL.search(setting) is None or RAW.search(setting) is not None:
            found.append(f"{name}:{number + 1}: font.{match.group(1)} is {setting}")
    return found


def listed(unmoved: Path) -> set[str]:
    """Read the files the list names, by their path under the modules folder."""
    names = (line.split("#")[0].strip() for line in unmoved.read_text().splitlines())
    return {name for name in names if name}


def main(modules: Path, unmoved: Path) -> int:
    """Check every file, printing what strays, and answer 1 if any does."""
    allowed = listed(unmoved)
    raw: list[str] = []
    stale: list[str] = []
    for path in sorted(modules.rglob("*.qml")):
        relative = path.relative_to(modules)
        if "tests" in relative.parts:
            continue
        name = relative.as_posix()
        found = raw_settings(modules, name)
        if name not in allowed:
            raw.extend(found)
        elif not found:
            stale.append(
                f"{name} is listed as not yet moved but sets its type "
                f"by level; take it off {unmoved.name}."
            )
    stale.extend(
        f"{name} is listed in {unmoved.name} but isn't there."
        for name in sorted(allowed)
        if not (modules / name).is_file()
    )
    for line in raw:
        print(line)
    if raw:
        print(
            "The lines above set type other than by a level; name one, as "
            "Theme.level.name.pointSize (docs/design.md, Type)."
        )
    for line in stale:
        print(line)
    return 1 if raw or stale else 0


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1]), Path(sys.argv[2])))
