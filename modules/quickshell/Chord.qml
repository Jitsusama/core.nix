// A chord, key by key as the desktop draws them (home.nix, keyFaces): a
// key's own glyph where one exists, its name where none does, joined by +.
// In a sequence a key pressed after the one before stands a space apart,
// with no +. At Note and muted, since a chord says how and the row says
// what.
pragma ComponentBehavior: Bound
import QtQuick

Flow {
    id: chord

    // Each key: { text, wide, after }.
    property var keys: []
    // The type's level it's set at (Theme.level).
    property var level: Theme.level.note
    property color colour: Theme.muted

    // On its keys' baseline, so a row can set it on its name's.
    baselineOffset: face.ascent

    FontMetrics {
        id: face
        font.family: chord.level.family
        font.pointSize: chord.level.pointSize
    }

    Repeater {
        model: chord.keys

        Text {
            required property var modelData
            required property int index

            // A part takes the room its glyph draws into, and each + keeps
            // the same distance from both its keys.
            leftPadding: index === 0 ? 0 : plus.width
            rightPadding: modelData.wide ? Theme.space.xs : 0
            text: modelData.text
            color: chord.colour
            font.family: chord.level.family
            font.pointSize: chord.level.pointSize
            font.weight: chord.level.weight

            Text {
                id: plus
                visible: parent.index > 0
                width: parent.modelData.after === true ? Theme.space.l
                    : implicitWidth + 2 * Theme.space.s
                horizontalAlignment: Text.AlignHCenter
                text: parent.modelData.after === true ? "" : "+"
                color: chord.colour
                font: parent.font
            }
        }
    }
}
