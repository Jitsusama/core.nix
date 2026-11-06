// What the keys do here, as every prompt's footer says it: each key in the
// text colour, then what it does, muted, in the monospace face (docs/design.md,
// Chords). The launcher, the picker, the lock screen and polkit all say it
// so, so it reads the same wherever it's asked. Each gap is its rung of the
// ladder (docs/design.md, Space): a chord's keys tie, a key stands a mark
// from its word, and one hint stands an item from the next, so "Space ? keys"
// reads as one chord and its word rather than "Space" and "? keys".
pragma ComponentBehavior: Bound
import QtQuick

Row {
    id: hints

    // Each a key and what it does: [["↵", "unlock"], ["Esc", "clear"]],
    // and the key's colour third where it isn't the text's, as a done
    // mark wears success, and its word's fourth. A key of several keys
    // pressed one after another names them with a space between,
    // "Space ?", and they're set a tie apart.
    property var keys: []
    // How wide each one's place is, when it keeps one wider than what it
    // says now, so the keys hold still as what they say changes; a key of
    // "" leaves its place empty.
    property var widths: []
    // Whether the last key's words keep to its place's right end, for keys
    // set against the right edge: what it says ends there however wide the
    // place has grown.
    property bool endsRight: false
    // The type's level (Theme.level): Note, a step under what they're
    // keys for, wherever they're said.
    property var level: Theme.level.note
    // Whether a muted dot stands between two hints, a mark from each, as a
    // card's status line joins them (StatusLine.qml), so its hints read as
    // one line of the same kind as the facts before them. The dot is in
    // the gap, so a line summing the hints by `spacing` sums it too.
    property bool dotted: false

    // How wide a key and its word are drawn, for a line holding a place
    // for them before they show: the keys a tie apart, then the word a
    // mark on.
    function advance(key: string, word: string): real {
        const parts = key.split(" ").filter(part => part !== "");
        return parts.reduce((sum, part) => sum + face.advanceWidth(part), 0)
            + Math.max(0, parts.length - 1) * Theme.space.rung.tie
            + ((word ?? "") === "" ? 0 : Theme.space.rung.mark + face.advanceWidth(word));
    }

    // Read again once the theme's face arrives, since a binding follows
    // what it reads and never what a call measured before.
    spacing: {
        void face.font;
        return dotted ? 2 * Theme.space.rung.mark + face.advanceWidth(hints.dot)
            : Theme.space.rung.items;
    }
    readonly property string dot: "\u00b7"
    // On its words' baseline, so a line can set it on its own.
    baselineOffset: face.ascent

    FontMetrics {
        id: face
        font.family: hints.level.family
        font.pointSize: hints.level.pointSize
        font.weight: hints.level.weight
    }

    Repeater {
        model: hints.keys

        Item {
            id: hint

            required property var modelData
            required property int index

            readonly property string key: modelData[0] ?? ""
            readonly property string word: modelData[1] ?? ""

            width: Math.max(said.implicitWidth, hints.widths[index] ?? 0)
            height: said.implicitHeight

            // The dot before every hint but the first, in the gap before
            // it, a mark from each side.
            Text {
                x: -Theme.space.rung.mark - implicitWidth
                visible: hints.dotted && hint.index > 0
                text: hints.dot
                textFormat: Text.PlainText
                color: Theme.muted
                font.family: hints.level.family
                font.pointSize: hints.level.pointSize
                font.weight: hints.level.weight
            }

            Row {
                id: said

                // Kept to the place's right end, or its left.
                x: hints.endsRight && hint.index === hints.keys.length - 1
                    ? hint.width - implicitWidth : 0
                spacing: Theme.space.rung.mark
                // A key of "" leaves its place empty.
                visible: hint.key !== ""

                Row {
                    spacing: Theme.space.rung.tie

                    Repeater {
                        model: hint.key.split(" ").filter(part => part !== "")

                        Text {
                            required property string modelData

                            text: modelData
                            textFormat: Text.PlainText
                            color: hint.modelData[2] ?? Theme.text
                            font.family: hints.level.family
                            font.pointSize: hints.level.pointSize
                            font.weight: hints.level.weight
                        }
                    }
                }

                // A key Joel knows by its mark alone, as the arrows, says
                // no word. Its word may have a hue of its own, as the act a
                // second Return does after a landing is said in the accent.
                Text {
                    visible: hint.word !== ""
                    text: hint.word
                    textFormat: Text.PlainText
                    color: hint.modelData[3] ?? Theme.muted
                    font.family: hints.level.family
                    font.pointSize: hints.level.pointSize
                    font.weight: hints.level.weight
                }
            }
        }
    }
}
