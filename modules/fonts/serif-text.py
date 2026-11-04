"""Cut a variable font at fixed axes into a static font with a family of its own.

A program that can't set a variable font's axes draws its default instance,
which for Fraunces is drawn for 144 points, so the words such a program sets
for reading come from a font cut at the axes the theme gives them. The cut
gets a family name of its own, since the variable font's static siblings
share its family at every optical size and softness, and fontconfig could
answer with one of those instead.

usage: serif-text.py <variable font> <family> <out> <tag=value>...
"""

import sys

from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

# The name table's records this sets, by their IDs: the family, its style,
# a unique name, the full name and the PostScript name. The typographic
# family and style, and the variations' PostScript prefix, would name the
# variable font's family, so they go.
FAMILY, STYLE, UNIQUE, FULL, POSTSCRIPT = 1, 2, 3, 4, 6
DROPPED = (16, 17, 25)


def named(font: TTFont, family: str) -> None:
    """Give the cut the family, as the only style it has, Regular."""
    names = font["name"]
    for record in list(names.names):
        if record.nameID in DROPPED:
            names.removeNames(nameID=record.nameID)
    postscript = family.replace(" ", "") + "-Regular"
    for name_id, value in (
        (FAMILY, family),
        (STYLE, "Regular"),
        (UNIQUE, postscript),
        (FULL, family),
        (POSTSCRIPT, postscript),
    ):
        names.setName(value, name_id, 3, 1, 0x409)
        names.setName(value, name_id, 1, 0, 0)


def main() -> None:
    """Cut the font named at the axes given, and write it out."""
    source, family, out, *axes = sys.argv[1:]
    location = {}
    for axis in axes:
        tag, value = axis.split("=")
        location[tag] = float(value)
    font = TTFont(source)
    tags = {axis.axisTag for axis in font["fvar"].axes}
    if set(location) != tags:
        sys.exit(f"every axis needs a value: given {sorted(location)}, has {sorted(tags)}")
    cut = instantiateVariableFont(font, location)
    named(cut, family)
    cut.save(out)


if __name__ == "__main__":
    main()
