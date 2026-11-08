"""Hold the shell's type to the theme's levels (type-is-set-by-level).

Reads every QML file under the modules folder given, tests aside, and fails,
naming each, on:

- a font's size, weight, axes, letter spacing or case set other than from a
  level (Theme.level.name.pointSize, or a property holding a level, such as
  row.level.pointSize);
- in each Typst template given after them, as the reader's page holds its
  own in Rust, a text's size, weight, tracking, strong's delta or a weight
  or optical size axis said as a number, or a size named with one, where
  every run of text should name a level (`..level.words.text`).

A setting mixing a level with a step, a font constant, a number or an object
of its own is set by neither, and fails as well.

Usage: type-is-set-by-level.py MODULES [TEMPLATE...]
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


# A Typst argument that sets type, and a size, weight or tracking given a
# name, as a template says them; a number among what either is said as is
# one of its own.
TYPST_SETTING = re.compile(r"\b(size|weight|tracking|delta|wght|opsz)\s*:\s*([^,)]*)")
TYPST_NAMED = re.compile(r"#let\s+([\w-]*(?:size|weight|tracking|delta))\s*=\s*(.*)")
NUMBER = re.compile(r"(?<![\w.{])\d+(?:\.\d+)?")


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


def raw_in_template(template: Path) -> list[str]:
    """Find each run of text a Rust file's Typst sets by number, saying where.

    Its tests, from the test module on, set nothing the reader shows, so
    they're left out; an item only for tests before it is read as any other,
    so it can't hide what follows.
    """
    found: list[str] = []
    before = ""
    for number, line in enumerate(template.read_text(encoding="utf-8").splitlines()):
        if before == "#[cfg(test)]" and line.lstrip().startswith("mod "):
            break
        if line.strip():
            before = line.strip()
        said = code(line)
        # A comment, Rust's or Typst's, says why rather than setting anything.
        if said.startswith("//"):
            continue
        settings = [*TYPST_SETTING.finditer(said), *TYPST_NAMED.finditer(said)]
        found.extend(
            f"{template.name}:{number + 1}: {match.group(1)} is {match.group(2).strip()}"
            for match in settings
            if NUMBER.search(match.group(2))
        )
    return found


def main(modules: Path, templates: list[Path]) -> int:
    """Check every file, printing what strays, and answer 1 if any does."""
    raw: list[str] = []
    for path in sorted(modules.rglob("*.qml")):
        relative = path.relative_to(modules)
        if "tests" in relative.parts:
            continue
        raw.extend(raw_settings(modules, relative.as_posix()))
    for line in raw:
        print(line)
    if raw:
        print(
            "The lines above set type other than by a level; name one, as "
            "Theme.level.name.pointSize (docs/design.md, Type)."
        )
    typst = [line for template in templates for line in raw_in_template(template)]
    for line in typst:
        print(line)
    if typst:
        print(
            "The lines above set the reader's type by number; name a level, as "
            "..level.words.text (modules/reader/home.nix)."
        )
    return 1 if raw or typst else 0


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1]), [Path(p) for p in sys.argv[2:]]))
