// A prompt's foot (docs/design.md, Shape and Kinds, the prompt kind): what
// the keys do, at the right, and where they lead, at the left, when a
// prompt says so. Its capitals sit as far under its hairline as a row's sit
// under the row's top, as the field's do, and its baseline the frame's
// inset over the frame's bottom (Space, The Ladder), so the room round the
// whole prompt is the same on every side. A prompt's field is one row, so
// the foot's baseline then falls a row, four beats, under the field's.
// Polkit's, the askpass's and the greeter's are one foot.
pragma ComponentBehavior: Bound
import QtQuick

Item {
    id: foot

    // What the keys do, as KeyHints takes them.
    property var keys: []
    // Where they lead, as styled text at Note, and the letter it opens on;
    // none when it's empty.
    property string leads
    property string leadsOpens: leads.charAt(0)

    readonly property int inset: Theme.space.rung.inset
    // A row: the field's height, four beats.
    readonly property int row: 4 * Theme.space.beat
    readonly property real capitals: noteFace.tightBoundingRect("H").height
    // Its words' baseline, from its top.
    readonly property real words: (row + capitals) / 2

    // Whole device pixels tall, as every band beside a rule is
    // (Hairline.qml).
    implicitHeight: rule.whole(words + inset)

    Hairline {
        id: rule
        visible: false
    }
    baselineOffset: words

    FontMetrics {
        id: noteFace
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
    }

    Text {
        visible: foot.leads !== ""
        x: foot.inset - noteFace.tightBoundingRect(foot.leadsOpens).x
        y: foot.words - baselineOffset
        width: hints.x - Theme.space.rung.groups - x
        textFormat: Text.StyledText
        text: foot.leads
        color: Theme.level.note.colour
        elide: Text.ElideRight
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
    }

    KeyHints {
        id: hints
        x: foot.width - foot.inset - width
        y: foot.words - baselineOffset
        keys: foot.keys
    }
}
