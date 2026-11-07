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
- a frame taken straight from Floating rather than through Card, outside the
  files listed as not yet on the card;
- a read of the inset rung, which the card stands its content at, outside
  the files listed as not yet on the card. The card's own `inset`, which a
  ground reaching past the content to the frame reads, is the one read
  meant to pass;
- a mark, name or number placed across by hand, an x or a side's margin set
  from a rung or a pitch, outside the files listed as not yet on the card,
  since every column comes from Columns.js;
- a listed file that neither takes Floating, reads the inset nor places
  anything by hand any more, or that isn't a card surface, so the list only
  ever shrinks.

Usage: every-card-is-the-one-card.py FOLDER UNMOVED SURFACE...
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


def listed(unmoved: Path) -> set[str]:
    """Read the files the list names, by their path under the folder."""
    names = (line.split("#")[0].strip() for line in unmoved.read_text().splitlines())
    return {name for name in names if name}


def main(folder: Path, unmoved: Path, surfaces: list[str]) -> int:
    """Check every surface, printing what strays, and answer 1 if any does."""
    allowed = listed(unmoved)
    framed: list[str] = []
    floated: list[str] = []
    inset: list[str] = []
    placed: list[str] = []
    stale: list[str] = []
    for name in surfaces:
        path = folder / name
        framed.extend(found(path, name, FRAME))
        floats = found(path, name, FLOATING)
        insets = found(path, name, INSET)
        hands = found(path, name, HAND_SET)
        if name not in allowed:
            floated.extend(floats)
            inset.extend(insets)
            placed.extend(hands)
        elif not floats and not insets and not hands:
            stale.append(
                f"{name} is listed as not yet on the card but is on it; take it off {unmoved.name}."
            )
    stale.extend(
        f"{name} is listed in {unmoved.name} but isn't a card surface."
        for name in sorted(allowed - set(surfaces))
    )
    said = [
        (framed, "The lines above draw a frame of their own; a card's frame is Card.qml's."),
        (floated, "The lines above take Floating for a card; take Card, whose frame it is."),
        (inset, "The lines above read the inset; stand them on the card's content, at the inset."),
        (placed, "The lines above place ink across by hand; stand it on a column (Columns.js)."),
    ]
    for lines, why in said:
        for line in lines:
            print(line)
        if lines:
            print(why)
    for line in stale:
        print(line)
    return 1 if framed or floated or inset or placed or stale else 0


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3:]))
