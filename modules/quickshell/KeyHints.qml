// What the keys do here, as every prompt's footer says it: each key in the
// text colour, then what it does, muted, in the monospace face, a large step
// apart (docs/design.md, Chords). The launcher, the picker, the lock screen
// and polkit all say it so, so it reads the same wherever it's asked.
import QtQuick

Row {
    id: hints

    // Each a key and what it does: [["↵", "unlock"], ["Esc", "clear"]].
    property var keys: []

    spacing: Theme.space.xl

    Repeater {
        model: hints.keys

        Text {
            required property var modelData

            text: `<font color="${Theme.text}">${modelData[0]}</font> ${modelData[1]}`
            textFormat: Text.StyledText
            color: Theme.muted
            font.family: Theme.mono.family
            font.pointSize: Theme.mono.large
        }
    }
}
