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
    // "Space ?", and a chord joins its keys by +, "Super+O"; they're set
    // a tie apart, and so is each + (partsOf).
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
    // Each is measured once until the face changes: the launcher's foot
    // weighs every listed row's keys as the rows change, the same few
    // keys and words each time, and measuring them in the font again
    // was a tenth of a step into Desktop under load (2026-11-10).
    function advance(key: string, word: string): real {
        const tie = Theme.space.rung.tie;
        const mark = Theme.space.rung.mark;
        const asked = `${key}\u241f${word ?? ""}\u241f${tie}\u241f${mark}`;
        const known = measured[asked];
        if (known !== undefined)
            return known;
        const parts = partsOf(key);
        const wide = parts.reduce((sum, part) => sum + face.advanceWidth(part), 0)
            + Math.max(0, parts.length - 1) * tie
            + ((word ?? "") === "" ? 0 : mark + face.advanceWidth(word));
        measured[asked] = wide;
        return wide;
    }
    // What advance has measured in this face, by key and word: a plain
    // object, so remembering one notifies nothing.
    property var measured: ({})

    // What a key is drawn as, each a tie from the next: the keys pressed
    // one after another, and in a chord its keys with each + on its own
    // between them, as a row's chord sets it (Chord.qml), so the foot's
    // "Super + \u21b5" reads as the rows' does (VKCF3Z, 2026-11-07). A +
    // that is the key itself, alone or as Ctrl's +, stays a key.
    function partsOf(key: string): var {
        const parts = [];
        for (const pressed of key.split(" ").filter(part => part !== "")) {
            let held = "";
            for (let at = 0; at < pressed.length; at++) {
                if (pressed[at] === "+" && held !== "" && at < pressed.length - 1) {
                    parts.push(held, "+");
                    held = "";
                } else {
                    held += pressed[at];
                }
            }
            parts.push(held);
        }
        return parts;
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
        // Another face measures every key afresh.
        onFontChanged: hints.measured = {}
        font.family: hints.level.family
        font.pointSize: hints.level.pointSize
        font.weight: hints.level.weight
    }

    // One place a key, each hint reading its key by its place: the foot's
    // keys are a new list at every key the selected row changes, and how
    // many fit changes with the row, and a Repeater given a new list, or a
    // new count, makes every hint again, which was 10 ms of each key's
    // frame under load (2026-11-11). So the places grow and shrink only by
    // the difference, and those kept say their new words.
    ListModel {
        id: places
    }
    function placed() {
        const wanted = hints.keys.length;
        if (places.count > wanted)
            places.remove(wanted, places.count - wanted);
        while (places.count < wanted)
            places.append({ at: places.count });
    }
    onKeysChanged: placed()
    Component.onCompleted: placed()

    Repeater {
        model: places

        Item {
            id: hint

            required property int index

            // This one's key, its word and their colours.
            readonly property var told: hints.keys[index] ?? []
            readonly property string key: told[0] ?? ""
            readonly property string word: told[1] ?? ""
            readonly property var parts: hints.partsOf(key)

            width: Math.max(said.implicitWidth, hints.widths[index] ?? 0)
            height: said.implicitHeight

            // The dot before every hint but the first, at its place's left
            // edge, a mark from the hint before it, so the dots run evenly:
            // a place held wider than what it says, as Return's is set to
            // the right end, keeps its room between the dot and its key,
            // and nothing left of it moves as its words change (VKCF3Z,
            // 2026-11-08).
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

                    // By how many, so a key of as many parts says its new
                    // ones in the texts already drawn.
                    Repeater {
                        model: hint.parts.length

                        Text {
                            required property int index

                            text: hint.parts[index] ?? ""
                            textFormat: Text.PlainText
                            color: hint.told[2] ?? Theme.text
                            font.family: hints.level.family
                            font.pointSize: hints.level.pointSize
                            font.weight: hints.level.weight
                            // The mono's slashed zero, so a key's 0 and O
                            // read apart, as Chord draws them.
                            font.features: ({ cv01: 1 })
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
                    color: hint.told[3] ?? Theme.muted
                    font.family: hints.level.family
                    font.pointSize: hints.level.pointSize
                    font.weight: hints.level.weight
                }
            }
        }
    }
}
