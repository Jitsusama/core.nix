"""Cut one of the theme's fonts into a small WOFF2 file for a website.

A site ships its own fonts, since a font from anyone else's server costs a
connection and tells them who read the page. The installed files are far too
big to send: each holds every script the face draws and every width and slant
it has. So each is cut to the characters a page in English uses, its marks
and arrows and box drawing among them, with every OpenType feature kept,
since the theme turns many on; and any axis the web never moves is pinned,
such as Monaspace's width.

usage: fonts.py <font> <out.woff2> <unicodes> [<tag=value>...]
"""

import io
import sys

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont


def pinned(axes: list[str]) -> dict[str, float]:
    """Read the axes to pin, each written as tag=value."""
    location = {}
    for axis in axes:
        tag, value = axis.split("=")
        location[tag] = float(value)
    return location


def main() -> None:
    """Pin the axes named, cut the font to the characters given, and write it."""
    source, out, unicodes, *axes = sys.argv[1:]
    options = subset.Options()
    options.layout_features = ["*"]
    options.flavor = "woff2"
    options.name_IDs = ["*"]
    # Read whole, not as each table is asked for: Fraunces' variations, read
    # lazily, lose glyphs the cut keeps.
    font: TTFont = subset.load_font(source, options, lazy=False)
    location = pinned(axes)
    if location:
        tags = {axis.axisTag for axis in font["fvar"].axes}
        if not set(location) <= tags:
            sys.exit(f"no such axis: given {sorted(location)}, has {sorted(tags)}")
        # Written out and read back, since the instancer leaves the glyphs'
        # variations keyed by names the cut can no longer find.
        buffer = io.BytesIO()
        instantiateVariableFont(font, location).save(buffer)
        buffer.seek(0)
        font = TTFont(buffer, lazy=False)
    cutter = subset.Subsetter(options)
    cutter.populate(unicodes=subset.parse_unicodes(unicodes))
    cutter.subset(font)
    subset.save_font(font, out, options)


if __name__ == "__main__":
    main()
