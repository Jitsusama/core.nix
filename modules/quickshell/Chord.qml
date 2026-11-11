// A chord, key by key as the desktop draws them (home.nix, keyFaces): a
// key's own glyph where one exists, its name where none does, joined by +.
// In a sequence a key pressed after the one before stands apart from it
// with no +. A chord is one token, so its parts are tied (docs/design.md,
// Space, The Ladder): a + a tie from each key, and a key pressed after
// another a tie from it, so "Space ?" reads as one chord and never as two
// keys. At Note and muted, since a chord says how and the row says what.
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

    // By how many keys, not the keys themselves: a Repeater given a new
    // list makes each of its Texts again, even one alike, as a row reused
    // for another, or refreshed, is; a count that stays the same keeps
    // them, each reading its key anew.
    Repeater {
        model: chord.keys.length

        Text {
            required property int index
            readonly property var modelData: chord.keys[index] ?? ({})

            // An icon's advance and its ink rarely agree, one drawing past
            // the other by a few pixels either way, so a wide part ends where
            // its ink does, and each + keeps the same distance from both its
            // keys. Any other key's width is left to its text: one bound to
            // implicitWidth reads it before a kept key's new padding is laid
            // out, and that read lays it out and changes the width being
            // worked out, a binding loop.
            readonly property rect ink: face.tightBoundingRect(text)
            leftPadding: index === 0 ? 0
                : modelData.after === true ? Theme.space.rung.tie : plus.width
            width: modelData.wide ? leftPadding + ink.x + ink.width : undefined
            text: modelData.text
            color: chord.colour
            font.family: chord.level.family
            font.pointSize: chord.level.pointSize
            font.weight: chord.level.weight
            // The mono's slashed zero, so a key's 0 and O read apart, which
            // at Note's size they didn't (VKCF3Z, 2026-11-06: Super+O read as
            // "+ 0"). Only in keys: a figure anywhere else keeps its zero.
            font.features: ({ cv01: 1 })

            // Always a +, hidden after a key pressed before it: a kept key
            // that changed its + to nothing changed the width read from it
            // as that width was worked out, a binding loop.
            Text {
                id: plus
                visible: parent.index > 0 && parent.modelData.after !== true
                width: implicitWidth + 2 * Theme.space.rung.tie
                horizontalAlignment: Text.AlignHCenter
                text: "+"
                color: chord.colour
                font: parent.font
            }
        }
    }
}
