"""Read the turned triangle back to the ink (a-turned-mark-keeps-its-ink).

tests/turned-mark.qml draws the theme's triangle and its turned triangle
through the one mark, side by side on lines a quarter of a point apart,
keeps a picture of each at the scale the shell ran at, and says what it
drew. This finds the ink in each picture, in the device's pixels, and fails,
saying which, unless at every line:

- the turned triangle is drawn turned: wide at its top and pointed at its
  foot, where the upright one is pointed at its top;
- its ink's box stands where the upright one's does, every edge within a
  device pixel, so up and down share one middle line (VKCF3Z, 2026-11-14).
  A device pixel, since the upright triangle's own box moves by one as its
  line moves between pixels.

Usage: turned-mark.py FOLDER LOG
"""

import json
import sys
from pathlib import Path
from typing import Any

from PIL import Image

# How bright a pixel must be to count as ink, on the black the case draws on:
# past the faint edge a glyph's smoothing leaves, so only its body counts.
INK = 90

# How far an edge of the turned ink may stand from the upright's, in device
# pixels.
WITHIN = 1


def meant(log: Path) -> dict[str, Any]:
    """Read what the shell said it drew."""
    for line in log.read_text(encoding="utf-8").splitlines():
        if "MEANT " in line:
            said = json.loads(line.split("MEANT ", 1)[1])
            if isinstance(said, dict):
                return said
    silent = f"the shell never said what it drew, in {log}"
    raise SystemExit(silent)


class Ink:
    """A picture's ink: its box, and how wide it runs on its top and foot rows."""

    def __init__(self, path: Path) -> None:
        """Find the ink in the picture at path."""
        picture = Image.open(path).convert("L")
        wide, tall = picture.size
        pixels = picture.tobytes()
        rows = [[x for x in range(wide) if pixels[y * wide + x] > INK] for y in range(tall)]
        inked = [y for y, row in enumerate(rows) if row]
        if not inked:
            blank = f"no ink in {path.name}"
            raise SystemExit(blank)
        self.top = inked[0]
        self.foot = inked[-1]
        self.left = min(row[0] for row in rows if row)
        self.right = max(row[-1] for row in rows if row)
        self.top_width = len(rows[self.top])
        self.foot_width = len(rows[self.foot])

    def box(self) -> tuple[int, int, int, int]:
        """Its box's edges: left, top, right, foot."""
        return (self.left, self.top, self.right, self.foot)


def main() -> None:
    """Hold each line's turned triangle to its upright one."""
    folder, log = Path(sys.argv[1]), Path(sys.argv[2])
    said = meant(log)
    if not said["turned"] or said["upright"]:
        wrong = f"the theme turns the wrong marks: {said}"
        raise SystemExit(wrong)
    faults = []
    for by in said["offsets"]:
        upright = Ink(folder / f"upright-{by}.png")
        turned = Ink(folder / f"turned-{by}.png")
        if upright.top_width >= upright.foot_width:
            faults.append(f"at {by}, the upright triangle isn't pointed at its top")
        if turned.top_width <= turned.foot_width:
            faults.append(
                f"at {by}, the turned triangle isn't turned: its top runs "
                f"{turned.top_width} pixels and its foot {turned.foot_width}"
            )
        apart = [abs(a - b) for a, b in zip(turned.box(), upright.box(), strict=True)]
        if max(apart) > WITHIN:
            faults.append(
                f"at {by}, the turned ink's box {turned.box()} stands off the "
                f"upright's {upright.box()}"
            )
    if faults:
        raise SystemExit("\n".join(faults))


if __name__ == "__main__":
    main()
