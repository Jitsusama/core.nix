"""Print the glyphs kitty shapes each text into, as Joel's kitty draws it, for checks.nix.

It reads the kitty.conf given, with the files it includes, and shapes each text
through kitty's own font code in the style given: regular, bold, italic or
bold-italic. Each text prints as a line of its cells' glyphs, so two texts drawn
alike print the same line.

usage: kitty +launch shape.py KITTY.CONF STYLE TEXT...
"""

import sys

from kitty.config import load_config
from kitty.fast_data_types import Screen, test_shape
from kitty.fonts import render

STYLES = {
    "regular": (False, False),
    "bold": (True, False),
    "italic": (False, True),
    "bold-italic": (True, True),
}


def shaped(text: str, *, bold: bool, italic: bool) -> str:
    """Shape the text in one line of a screen and say each cell's glyphs."""
    screen = Screen(None, 1, len(text) * 2)
    screen.cursor.bold = bold
    screen.cursor.italic = italic
    screen.draw(text)
    return " ".join(str(glyphs) for *_, glyphs in test_shape(screen.line(0), None))


def main() -> None:
    """Load the configuration, then shape every text with its faces."""
    config, style, *texts = sys.argv[1:]
    bold, italic = STYLES[style]
    options = load_config(config)
    # kitty's test fonts are made from its defaults and a family; made from
    # the configuration instead, the bold and italic faces are chosen and
    # given their features as kitty itself chooses and gives them.
    render.defaults = options
    with render.setup_for_testing(options.font_family.created_from_string, options.font_size):
        for text in texts:
            print(shaped(text, bold=bold, italic=italic))


main()
