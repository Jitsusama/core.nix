// The ground behind the letters a find matched in a line of words, as Joel's
// Neovim's Search draws a match under its ink (Theme.wash). Laid in the
// words' own Text, beneath its letters, so it moves as they read on. The
// ink is the words' own (Launcher.qml's marked), since Qt draws a ground
// only in rich text, and rich text won't end in an ellipsis as a row cut to
// its room must. Each is made by a Loader once there's something to wash,
// so a row nothing was found in keeps none, and kept while its words stand
// (ReadsOn.qml), and the Loader lies beneath the letters (z: -1), since z
// orders an item only among its siblings.
pragma ComponentBehavior: Bound
import QtQuick

Item {
    id: wash

    // The words as they read, with no markup, and the runs of their
    // letters to wash, [start, end] each (Fuzzy.runs).
    property string text
    property var runs: []
    // The words' face, which each run is measured in.
    property font font
    // Whether the words end in an ellipsis, cut to their room, which is
    // the width they're drawn in: a run past the cut isn't washed.
    property bool cut: false
    // Whether the words lie on the selection, which takes the wash's
    // ground a shade past it.
    property bool selected: false
    // A binding doesn't see what a call reads, so each one measuring the
    // words reads the face itself, to be measured again when the words'
    // face arrives, rather than keep the default face's widths.
    readonly property real reach: {
        void measure.font;
        return cut ? width - measure.advanceWidth("\u2026") : Infinity;
    }

    // What's washed, each run's start, width and ground, up to the reach,
    // which a test reads too.
    readonly property var laid: {
        void measure.font;
        const ground = selected ? Theme.wash.selected : Theme.wash.ground;
        return runs.map(run => {
            const start = measure.advanceWidth(text.slice(0, run[0]));
            const end = Math.min(reach,
                start + measure.advanceWidth(text.slice(run[0], run[1])));
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
