// A card's status line, the launcher's and held notices' alike (Joel,
// 2026-11-05: "we need for notifications and the laucnher to feel more
// similar ... i like the status line in notifications better"): the mode in
// three capitals first, where the eye starts the line, then the surface's
// own facts, then the hints at the right, joined by dots, Return last. One
// component both read, so the two lines can't drift apart (VKCF3Z's family
// cut, docs/design.md). The facts are the surface's own, the launcher's
// glance or the held list's position and what's found, so it sets them
// itself beside this, from factsAt, short of rightAt.
pragma ComponentBehavior: Bound
import QtQuick
import "Bands.js" as Bands

Item {
    id: line

    // Where the line's first ink starts, the frame's inset its owner holds,
    // so the mark moves with the card's text rather than keeping a distance
    // of its own (VKCF3Z, 2026-11-05).
    property real inset: Theme.space.rung.inset
    // The mode in three capitals, NOR FND CMD INS VIS, and whether it's
    // normal's, which is muted as his status line leaves normal unnamed;
    // every other mode is in the accent. A line with no modes, as a
    // secret's, shows none and keeps its room.
    property string mark: ""
    property bool resting: true
    property bool marked: true
    // The hints, as KeyHints takes them, and the places some keep wider
    // than what they say, so Return holds still.
    property var hints: []
    property var widths: []
    // What was just done, said in the hints' place until the next key, in
    // its own colour, as an error's alert.
    property string said: ""
    property color saidHue: Theme.text
    // How wide what was done may be drawn, its first words giving way.
    property real saidRoom: width - 2 * inset
    // The line's baseline, down from its top, which the mark, the facts
    // and the hints all sit on: centred by eye in the line's band, the
    // hairline over it to the frame under it (docs/design.md: text in a
    // band centres in it), unless its owner sets it on the rows' rhythm
    // and sizes the band by `tall`.
    property real base: Bands.baseline(height, capitals, smalls)
    // How tall the band is that centres a baseline at `base`, where the
    // frame's bottom falls (docs/design.md, A frame's bottom is set).
    readonly property real tall: Bands.around(base, capitals, smalls)
    // The Note's capitals and small letters, the line's largest level, the
    // Label's mark standing as tall: TextMetrics, since a FontMetrics'
    // call in a binding is asked once, before the face has loaded.
    readonly property real capitals: noteCaps.tightBoundingRect.height
    readonly property real smalls: noteSmall.tightBoundingRect.height

    // The hints, for a surface measuring a key by them.
    readonly property alias keys: hinted
    // Where the facts start: a group past the mark. The label's tracking
    // follows its last capital too, so it's taken off, leaving the rung
    // from ink to ink.
    readonly property real factsAt: markText.x + markText.implicitWidth
        - markText.font.letterSpacing + Theme.space.rung.groups
    // Where what's said at the right starts, the hints or what was done.
    readonly property real rightAt: width - inset
        - (said !== "" ? saidText.width : hinted.implicitWidth)

    TextMetrics {
        id: noteCaps
        text: "H"
        font: saidText.font
    }
    TextMetrics {
        id: noteSmall
        text: "x"
        font: saidText.font
    }

    Text {
        id: markText
        x: line.inset
        anchors.baseline: parent.top
        anchors.baselineOffset: line.base
        visible: line.marked
        text: line.mark
        textFormat: Text.PlainText
        color: line.resting ? Theme.muted : Theme.accent
        font.family: Theme.level.label.family
        font.pointSize: Theme.level.label.pointSize
        font.weight: Theme.level.label.weight
        font.capitalization: Theme.level.label.capitalization
        font.letterSpacing: Theme.level.label.letterSpacing
        font.features: Theme.level.label.features
    }

    KeyHints {
        id: hinted
        x: line.width - line.inset - implicitWidth
        anchors.baseline: parent.top
        anchors.baselineOffset: line.base
        visible: line.said === ""
        dotted: true
        keys: line.hints
        widths: line.widths
        // The last hint's words end its held place, on the inset, so room
        // held for a wider word stands to their left, never as a gap
        // before the frame (VKCF3Z, 2026-11-05).
        endsRight: true
    }

    Text {
        id: saidText
        anchors.right: parent.right
        anchors.rightMargin: line.inset
        anchors.baseline: parent.top
        anchors.baselineOffset: line.base
        width: Math.min(implicitWidth, line.saidRoom)
        visible: line.said !== ""
        text: line.said
        textFormat: Text.PlainText
        elide: Text.ElideLeft
        color: line.saidHue
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
    }
}
