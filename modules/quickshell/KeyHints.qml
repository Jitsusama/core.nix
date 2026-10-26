// What the keys do here, as every prompt's footer says it: each key in the
// text colour, then what it does, muted, in the monospace face, a large step
// apart (docs/design.md, Chords). The launcher, the picker, the lock screen
// and polkit all say it so, so it reads the same wherever it's asked.
pragma ComponentBehavior: Bound
import QtQuick

Row {
    id: hints

    // Each a key and what it does: [["↵", "unlock"], ["Esc", "clear"]].
    property var keys: []
    // How wide each one's place is, when it keeps one wider than what it
    // says now, so the keys hold still as what they say changes; a key of
    // "" leaves its place empty.
    property var widths: []
    // Whether the last key's words keep to its place's right end, for keys
    // set against the right edge: what it says ends there however wide the
    // place has grown.
    property bool endsRight: false
    // The type's step, large in a prompt and smaller in a notice, whose
    // footer is smaller; the space between them follows it.
    property real size: Theme.mono.large

    spacing: size < Theme.mono.large ? Theme.space.l : Theme.space.xl

    Repeater {
        model: hints.keys

        Text {
            required property var modelData
            required property int index

            width: Math.max(implicitWidth, hints.widths[index] ?? 0)
            horizontalAlignment: hints.endsRight && index === hints.keys.length - 1
                ? Text.AlignRight : Text.AlignLeft
            text: modelData[0] === "" ? ""
                : `<font color="${Theme.text}">${modelData[0]}</font> ${modelData[1]}`
            textFormat: Text.StyledText
            color: Theme.muted
            font.family: Theme.mono.family
            font.pointSize: hints.size
        }
    }
}
