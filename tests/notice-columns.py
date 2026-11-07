"""Read a notice's cards back to the ink (notices-stand-on-the-cards-columns).

tests/notice-columns.qml draws a notice on the one card twice, as the pop-up
does with no gutter and as the held list does with one, keeps a picture of
each at the scale the shell ran at, and says where its frame and columns
were meant to stand. This finds the ink in each picture, in the device's
pixels, and fails, saying which, unless:

- the ink stands the inset from the frame's inside edge on every side, to
  within a device pixel, the mark's included (VKCF3Z, 2026-11-06);
- the signing line's name starts on the text column, the inset and a mark's
  column in from the frame on the pop-up;
- the held card's text column is the pop-up's moved by the gutter alone;
- the mark's ink centres on the title's capitals, to within a device pixel.

Usage: notice-columns.py FOLDER LOG
"""

import json
import sys
from pathlib import Path
from typing import Any

from PIL import Image

# How far a pixel's colour must stand from the card's fill to count as ink:
# well past the shading a translucent fill takes from what's under it, so
# only what's drawn on the card counts.
INK = 90


def meant(log: Path) -> dict[str, Any]:
    """Read what the shell said the cards were meant to be."""
    for line in log.read_text(encoding="utf-8").splitlines():
        if "MEANT " in line:
            said = json.loads(line.split("MEANT ", 1)[1])
            if isinstance(said, dict):
                return said
    silent = f"the shell never said what it meant, in {log}"
    raise SystemExit(silent)


class Card:
    """A card's picture, its ink found inside its frame."""

    def __init__(self, path: Path, size: tuple[float, float], said: dict[str, Any]) -> None:
        """Find the ink in the picture at path, of a card as wide and tall as the shell says."""
        self.picture = Image.open(path).convert("RGB")
        self.pixels = self.picture.tobytes()
        self.scale = self.picture.width / size[0]
        # The frame's inside edges, in device pixels, where the shell put them:
        # a picture's own size rounds a card's height down to a whole pixel.
        border = float(said["border"]) * self.scale
        self.inside = (border, border, size[0] * self.scale - border, size[1] * self.scale - border)
        # Inside the frame and clear of its corners' arcs, in device pixels.
        self.edge = round(float(said["border"]) * self.scale)
        self.corner = round(float(said["radius"]) * self.scale)
        wide, tall = self.picture.size
        middle = self.colour(wide // 2, self.edge + 2)
        self.ink = {
            (x, y)
            for y in range(self.edge + 1, tall - self.edge - 1)
            for x in range(self.edge + 1, wide - self.edge - 1)
            if not self.in_corner(x, y)
            and sum(abs(a - b) for a, b in zip(self.colour(x, y), middle, strict=True)) > INK
        }

    def colour(self, x: int, y: int) -> tuple[int, int, int]:
        """Read a pixel's red, green and blue."""
        at = 3 * (y * self.picture.width + x)
        return self.pixels[at], self.pixels[at + 1], self.pixels[at + 2]

    def in_corner(self, x: int, y: int) -> bool:
        """Say whether a pixel lies in a corner's square, where the frame turns."""
        wide, tall = self.picture.size
        near_x = x < self.corner or x >= wide - self.corner
        near_y = y < self.corner or y >= tall - self.corner
        return near_x and near_y

    def logical(self, pixels: float) -> float:
        """Turn device pixels into the shell's own."""
        return pixels / self.scale

    def insets(self) -> dict[str, float]:
        """Say how far the ink stands from the frame's inside edge on each side."""
        xs = [x for x, _ in self.ink]
        ys = [y for _, y in self.ink]
        left, top, right, bottom = self.inside
        # A pixel's ink spans it whole, from its left or top edge to the next.
        return {
            "left": self.logical(min(xs) - left),
            "right": self.logical(right - max(xs) - 1),
            "top": self.logical(min(ys) - top),
            "bottom": self.logical(bottom - max(ys) - 1),
        }

    def lines(self) -> list[tuple[int, int]]:
        """Give the bands of rows the ink fills, top to bottom, each its first and last row."""
        rows = sorted({y for _, y in self.ink})
        bands = [[rows[0], rows[0]]]
        for y in rows[1:]:
            if y == bands[-1][1] + 1:
                bands[-1][1] = y
            else:
                bands.append([y, y])
        return [(top, foot) for top, foot in bands]

    def left_in(self, band: tuple[int, int], past: int = 0) -> int:
        """Find the leftmost ink in a band of rows, at or past a column."""
        return min(x for x, y in self.ink if band[0] <= y <= band[1] and x >= past)

    def middle_in(self, band: tuple[int, int], left: int, right: int) -> float:
        """Find the middle of the ink in a band between two columns, down from the top."""
        ys = [y for x, y in self.ink if band[0] <= y <= band[1] and left <= x < right]
        return (min(ys) + max(ys)) / 2


def main(folder: Path, log: Path) -> int:
    """Measure both cards, printing each case that fails, and answer 1 if any does."""
    said = meant(log)
    pop = Card(folder / "pop.png", (said["width"], said["heights"][0]), said)
    held = Card(folder / "held.png", (said["width"], said["heights"][1]), said)
    failed: list[str] = []
    tolerance = 1 / pop.scale
    inset = said["inset"]

    # The held card's gutter has no number in it here, so its left ink is
    # the mark's, the gutter past the inset.
    for name, card, gutter in [("pop-up", pop, 0), ("held", held, said["gutter"])]:
        for side, stands in card.insets().items():
            meant_to = inset + (gutter if side == "left" else 0)
            if abs(stands - meant_to) > tolerance:
                failed.append(
                    f"the {name} card's ink stands {stands} from its {side} edge, not {meant_to}"
                )

    # The signing line is the last band; its name's ink starts the text column.
    columns = {}
    for name, card in [("pop-up", pop), ("held", held)]:
        signing = card.lines()[-1]
        columns[name] = card.logical(card.left_in(signing) - card.inside[0])
    if abs(columns["pop-up"] - (inset + said["text"])) > tolerance:
        failed.append(
            f"the pop-up's text column stands {columns['pop-up']} in, "
            f"not the inset and a mark's column, {inset + said['text']}"
        )
    if abs(columns["held"] - columns["pop-up"] - said["gutter"]) > tolerance:
        failed.append(
            f"the held card's text column is {columns['held'] - columns['pop-up']} past the "
            f"pop-up's, not the gutter alone, {said['gutter']}"
        )

    # The title's first line is the first band; the mark is the ink left of
    # the text column in it, the capitals the ink from the column on.
    for name, card in [("pop-up", pop), ("held", held)]:
        title = card.lines()[0]
        text = round(columns[name] * card.scale + card.inside[0])
        mark = card.middle_in(title, 0, text)
        capitals = card.middle_in(title, text, card.picture.width)
        if abs(mark - capitals) > 1:
            failed.append(
                f"the {name} card's mark centres {card.logical(mark - capitals)} from its "
                "title's capitals"
            )

    for failure in failed:
        print(f"FAIL {failure}")
    print(f"at {pop.scale}: insets {pop.insets()}, columns {columns}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1]), Path(sys.argv[2])))
