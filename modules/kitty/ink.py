"""Print which cells of each text Joel's kitty inks, for checks.nix.

It reads the kitty.conf given, with the files it includes, and renders each
text through kitty's own font code, as a window would draw it. Each text prints
as a line of its cells, one word each: "inked" where a glyph puts ink in the
cell, "blank" where none does. A symbol that spills into the space after it
inks that space, which kitty does with a symbol wider than its cell unless
narrow_symbols holds it to one.

usage: kitty +launch ink.py KITTY.CONF TEXT...
"""

import sys

from kitty.config import load_config
from kitty.fast_data_types import Screen, sprite_idx_to_pos, test_render_line
from kitty.fonts import render

# A sprite's index without the bit kitty sets on it for a coloured glyph,
# which says nothing about where the sprite is kept, as kitty masks it.
INDEX = 0x7FFFFFFF


def inked(text: str, sprites: dict[tuple[int, int, int], bytes]) -> str:
    """Render the text in one line of a screen and say whether each cell is inked."""
    screen = Screen(None, 1, len(text))
    screen.draw(text)
    line = screen.line(0)
    test_render_line(line)
    cells = []
    for column in range(len(text)):
        sprite = line.sprite_at(column) & INDEX
        where = sprite_idx_to_pos(
            sprite, render.setup_for_testing.xnum, render.setup_for_testing.ynum
        )
        pixels = sprites.get(where, b"") if sprite else b""
        cells.append("inked" if any(pixels) else "blank")
    return " ".join(cells)


def main() -> None:
    """Load the configuration, then render every text with its faces."""
    config, *texts = sys.argv[1:]
    options = load_config(config)
    # As in shape.py: made from the configuration, the faces, and the
    # symbols held to one cell, are kitty's own for the account.
    render.defaults = options
    with render.setup_for_testing(options.font_family.created_from_string, options.font_size) as (
        sprites,
        _width,
        _height,
    ):
        for text in texts:
            print(inked(text, sprites))


main()
