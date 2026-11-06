// A prompt's foot (docs/design.md, Shape and Kinds, the prompt kind): what
// the keys do, at the right, and where they lead, at the left, when a
// prompt says so. Its capitals sit as far under its hairline as a row's sit
// under the row's top, as the field's do, so its baseline falls a row, four
// beats, under the field's, a prompt's field being one row; and the frame's
// bottom falls where its words read centred between the hairline and it
// (docs/design.md, A frame's bottom is set).
// Polkit's, the askpass's and the greeter's are one foot.
pragma ComponentBehavior: Bound
import QtQuick
import "Bands.js" as Bands

Item {
    id: foot

    // What the keys do, as KeyHints takes them, joined by dots where the
    // owner speaks the status line's way (StatusLine.qml), as the region
    // picker does.
    property var keys: []
    property bool dotted: false
    // Where they lead, as styled text at Note, and the letter it opens on;
    // none when it's empty.
    property string leads
    property string leadsOpens: leads.charAt(0)

    readonly property int inset: Theme.space.rung.inset
    // A row: the field's height, four beats.
    readonly property int row: 4 * Theme.space.beat
    readonly property real capitals: noteCaps.tightBoundingRect.height
    // Its words' baseline, from its top.
    readonly property real words: (row + capitals) / 2

    // Whole device pixels tall, as every band beside a rule is
    // (Hairline.qml).
    implicitHeight: rule.whole(Bands.around(words, capitals,
        noteSmall.tightBoundingRect.height))
    // How wide it wants to be: its keys with the inset each side.
    readonly property real wanted: hints.implicitWidth + 2 * inset

    Hairline {
        id: rule
        visible: false
    }
    baselineOffset: words

    // The Note's capitals and small letters: TextMetrics rather than a
    // FontMetrics' call, which a binding asks once, before the face has
    // loaded, and reads small ever after.
    TextMetrics {
        id: noteCaps
        text: "H"
        font: noteFace.font
    }
    TextMetrics {
        id: noteSmall
        text: "x"
        font: noteFace.font
    }

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
        dotted: foot.dotted
    }
}
