// The ground behind each place a search finds in a line of words, as Joel's
// Neovim's Search draws a match under its ink (Theme.wash). Laid in the
// words' own Text, beneath its letters, so it moves as they read on. The
// ink is the words' own (Search.inked), since Qt draws a ground only in
// rich text, and rich text won't end in an ellipsis as a row cut to its
// room must. Each is made by a Loader only while a search washes, so a row
// keeps none while nothing is searched, and the Loader lies beneath the
// letters (z: -1), since z orders an item only among its siblings.
pragma ComponentBehavior: Bound
import QtQuick
import "Search.js" as Search

Item {
    id: wash

    // The words as they read, with no markup, and what's sought in them.
    property string text
    property string pattern
    // The words' face, which each place is measured in.
    property font font
    // Whether the words end in an ellipsis, cut to their room, which is
    // the width they're drawn in: a place past the cut isn't washed.
    property bool cut: false
    // Whether the words lie on the selection, which takes the wash's
    // ground a shade past it.
    property bool selected: false
    // Whether the first place is the find the selection landed on, which
    // takes the accent, as his Neovim's CurSearch draws the match under
    // the cursor.
    property bool landed: false
    // A binding doesn't see what a call reads, so each one measuring the
    // words reads the face itself, to be measured again when the words'
    // face arrives, rather than keep the default face's widths.
    readonly property real reach: {
        void measure.font;
        return cut ? width - measure.advanceWidth("\u2026") : Infinity;
    }

    readonly property var spans: Search.found(pattern, text)
    // What's washed, each place's start, width and ground, up to the
    // reach, which a test reads too.
    readonly property var laid: {
        void measure.font;
        return spans.map((span, at) => {
            const start = measure.advanceWidth(text.slice(0, span[0]));
            const end = Math.min(reach,
                start + measure.advanceWidth(text.slice(span[0], span[1])));
            const ground = at === 0 && landed ? Theme.wash.current
                : selected ? Theme.wash.selected : Theme.wash.ground;
            return [start, end - start, ground];
        }).filter(place => place[1] > 0);
    }

    FontMetrics {
        id: measure
        font: wash.font
    }

    Repeater {
        model: wash.laid

        Rectangle {
            required property var modelData
            x: modelData[0]
            width: modelData[1]
            height: wash.height
            color: modelData[2]
        }
    }
}
