"""Read each emoji's mark back to its ink (an-emoji-stands-on-its-names-capitals).

tests/emoji-marks.qml draws the first emoji of each of Emoji's groups, three
below the emoji blocks, a heart asking for its picture, and the theme's home
mark, through the one mark beside a name's capitals, on
lines a quarter of a point apart, keeps a picture of each at the scale the
shell ran at, and says what it drew. This finds the ink in each picture, in
the device's pixels, the mark's left of the cell's middle and the capitals'
right of it, and fails, saying which, unless:

- the colour emoji face is there, and every emoji, but not the home mark,
  is taken for a picture, and none of the theme's marks is, nor any of the
  letters it asks of beside them;
- every picture's ground is clear, so ink is the mark's and the capitals';
- every mark drew ink;
- each mark's ink has its middle within two device pixels of the capitals'
  middle. A colour emoji used to stand its top edge there, half its height
  low (Joel, 2026-11-19: Emoji's groups); the theme's marks stand within
  one. Qt draws a small colour glyph under a scale factor up to a logical
  pixel off its box, which two device pixels allows at 2.

Usage: emoji-marks.py FOLDER LOG
"""

import sys
from pathlib import Path

from PIL import Image
from shell_said import meant

# How opaque a pixel must be to count as ink, on the clear ground the case
# draws on: past the faint edge smoothing leaves, so only the body counts.
INK = 128

# How far a mark's ink middle may stand from the capitals', in device pixels.
WITHIN = 2


def middle(alpha: bytes, wide: int, tall: int, columns: range) -> float | None:
    """Find the middle, down, of the ink in these columns, or None for none."""
    rows = [y for y in range(tall) if any(alpha[y * wide + x] > INK for x in columns)]
    if not rows:
        return None
    return (rows[0] + rows[-1]) / 2


def faults_in(path: Path) -> list[str]:
    """What's wrong with one picture's mark against its capitals."""
    picture = Image.open(path).convert("RGBA")
    wide, tall = picture.size
    alpha = picture.getchannel("A").tobytes()
    # The cell's corners are its ground, which the case leaves clear.
    corners = [alpha[0], alpha[wide - 1], alpha[(tall - 1) * wide], alpha[-1]]
    if any(corners):
        return [f"{path.stem}: the ground isn't clear, its corners {corners}"]
    mark = middle(alpha, wide, tall, range(wide // 2))
    capitals = middle(alpha, wide, tall, range(wide // 2, wide))
    if mark is None or capitals is None:
        return [f"{path.stem}: no ink, the mark's middle {mark}, the capitals' {capitals}"]
    if abs(mark - capitals) > WITHIN:
        off = mark - capitals
        said = (
            f"{path.stem}: the mark's ink middle stands at {mark}, {off:+} device pixels "
            f"off the capitals' {capitals}"
        )
        return [said]
    return []


def main() -> None:
    """Hold each mark to the capitals it stands beside."""
    folder, log = Path(sys.argv[1]), Path(sys.argv[2])
    said = meant(log)
    faults = []
    if not said["emojiFace"]:
        faults.append("the colour emoji face isn't there to draw in")
    emoji = [name for name in said["glyphs"] if name != "home"]
    if said["pictured"] != emoji:
        faults.append(f"taken for pictures: {said['pictured']}, not {emoji}")
    if said["themePictured"]:
        faults.append(f"the theme's marks taken for pictures: {said['themePictured']}")
    if said["lettersPictured"]:
        faults.append(f"letters taken for pictures: {said['lettersPictured']}")
    if said["wronglyKept"]:
        faults.append(f"measures kept that draw a mark as if inkless: {said['wronglyKept']}")
    if said["rightlyLost"]:
        faults.append(f"measures that should have been kept weren't: {said['rightlyLost']}")
    for by in said["offsets"]:
        for name in said["glyphs"]:
            faults += faults_in(folder / f"{name}-{by}.png")
    if faults:
        raise SystemExit("\n".join(faults))


if __name__ == "__main__":
    main()
