"""Hold the shell's cards to the one card (every-card-is-the-one-card).

The launcher, the held notices and a notice are one card (Card.qml): its
frame, and everything inside it standing the inset from its edge
(docs/design.md, Shape and Kinds). Each surface drew its own frame from
Floating and placed its own ink from the inset, so a change to one never
reached the others and they drifted apart (VKCF3Z, 2026-11-06).

Reads each card surface named, by its path under the folder given, and fails,
naming each line, on:

- a frame of its own: a border, the whole thing's radius or a shadow, which
  only Floating.qml draws, beneath Card.qml;
- a frame taken straight from Floating rather than through Card;
- a read of the inset rung, which the card stands its content at. The
  card's own `inset`, which a ground reaching past the content to the frame
  reads, is the one read meant to pass;
- a mark, name or number placed across by hand, an x or a side's margin set
  from a rung or a pitch, since every column comes from Columns.js;
- a row's ground drawn of its own, read from the selection's tint, since
  every row's ground is Ground.qml's, which fills its band edge to edge
  (Joel, 2026-11-06: rows tile like Vim's cursorline).

Usage: every-card-is-the-one-card.py FOLDER SURFACE...
"""

import re
import sys
from pathlib import Path

# What draws a frame: a border, a whole thing's corners, a shadow.
FRAME = re.compile(r"\bborder\.(?:width|color)\b|\bradius:\s*Theme\.radius\b|\bRectangularShadow\b")
# A frame made straight from the floating surface, never through the card.
FLOATING = re.compile(r"\bFloating\s*\{")
# The inset, which only the card stands its content at.
INSET = re.compile(r"\bTheme\.space\.rung\.inset\b")
# Ink placed across by hand, from a rung or a pitch, rather than a column.
HAND_SET = re.compile(
    r"^\s*(?:x|anchors\.(?:left|right|horizontalCenter)Margin|anchors\.margins"
    r"|leftPadding|rightPadding)\s*:.*\bTheme\.(?:space\.rung\.\w+|level\.\w+\.pitch)\b"
)
# A row's ground, tinted as only the shared row tints one.
GROUND = re.compile(r"\bTheme\.floating\.selection\b")


def code(line: str) -> str:
    """Give the line without a trailing comment, which says why rather than sets."""
    return re.sub(r"(^|\s)//.*$", "", line)


def found(path: Path, name: str, pattern: re.Pattern[str]) -> list[str]:
    """Find each line in a file the pattern matches, saying where."""
    lines = path.read_text(encoding="utf-8").splitlines()
    return [
        f"{name}:{number + 1}: {line.strip()}"
        for number, line in enumerate(lines)
        if pattern.search(code(line))
    ]


def main(folder: Path, surfaces: list[str]) -> int:
    """Check every surface, printing what strays, and answer 1 if any does."""
    framed: list[str] = []
    floated: list[str] = []
    inset: list[str] = []
    placed: list[str] = []
    grounded: list[str] = []
    for name in surfaces:
        path = folder / name
        framed.extend(found(path, name, FRAME))
        floated.extend(found(path, name, FLOATING))
        inset.extend(found(path, name, INSET))
        placed.extend(found(path, name, HAND_SET))
        grounded.extend(found(path, name, GROUND))
    said = [
        (framed, "The lines above draw a frame of their own; a card's frame is Card.qml's."),
        (floated, "The lines above take Floating for a card; take Card, whose frame it is."),
        (inset, "The lines above read the inset; stand them on the card's content, at the inset."),
        (placed, "The lines above place ink across by hand; stand it on a column (Columns.js)."),
        (grounded, "The lines above draw a row's ground; draw it through Ground.qml."),
    ]
    for lines, why in said:
        for line in lines:
            print(line)
        if lines:
            print(why)
    return 1 if framed or floated or inset or placed or grounded else 0


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1]), sys.argv[2:]))
