// A row of a palette's list, the launcher's or the margin's panel: its glyph
// in a column of its own, its name, and on the right its chord or what it
// is, muted (docs/design.md, Shape and Kinds).
//
// A row that's a thing, a noun, says what kind of thing at its right edge
// (kind: "program", "window"), and what more it has to say muted after its
// name, so every row says what it is. A row that's an act or a set says so
// by its words ("Lock the screen", "Windows"), so it has no kind, and its
// right edge keeps its chord; a noun's chord is taught by the foot instead.
// Of four ways drawn at full size, this one read best (2026-10-23): the
// kind never comes before the thing, and the acts keep their chords.
// Not Row, which is QtQuick's own and would silently win.
pragma ComponentBehavior: Bound
import QtQuick

Rectangle {
    id: row

    property bool selected: false
    property string glyph
    property color hue
    property string name
    property var chord: []
    property string side
    // A word after the name, muted: "here" beside a workspace.
    property string count
    // What kind of thing it is, for a noun; empty for an act or a set.
    property string kind
    // The other word it was found by, when not its name: said in quotes
    // right after the name, so a row is never named by a word it doesn't
    // say (Suspend the machine "sleep").
    property string also
    // What's said after the name: the word it was found by, its count, and
    // for a noun what would have been on the right.
    readonly property string detail: [also !== "" ? `\u201c${also}\u201d` : "", count,
        kind !== "" ? side : ""].filter(part => part !== "").join(" \u00b7 ")
    // Said by the shell rather than found, so it's muted and can't be picked.
    property bool quiet: false

    signal clicked
    // The pointer is over the row, at this place in the window.
    signal pointed(point at)

    width: parent.width
    height: Theme.space.xxxl
    radius: Theme.inner
    color: selected ? Qt.alpha(Theme.selection, Theme.floating.selection) : "transparent"

    Text {
        x: Theme.space.l
        anchors.verticalCenter: parent.verticalCenter
        text: row.glyph
        color: row.hue
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }

    Text {
        id: nameText
        x: Theme.space.l + Theme.space.xxl
        width: Math.max(0, Math.min(implicitWidth, right.x - x - Theme.space.xxl
            - (row.detail === "" ? 0 : Math.min(countText.implicitWidth, row.width * 0.3)
                + Theme.space.m)))
        anchors.verticalCenter: parent.verticalCenter
        text: row.name
        textFormat: row.selected || row.quiet ? Text.PlainText : Text.StyledText
        elide: Text.ElideRight
        color: row.quiet ? Theme.muted : row.selected ? Theme.strong : Theme.text
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
        font.weight: row.selected ? Font.Medium : Font.Normal
    }

    Text {
        id: countText
        x: nameText.x + nameText.width + Theme.space.m
        width: Math.min(implicitWidth, row.width * 0.3)
        elide: Text.ElideRight
        anchors.verticalCenter: parent.verticalCenter
        visible: row.detail !== ""
        text: row.detail
        color: Theme.muted
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }

    // The chord, or what the row is.
    Item {
        id: right
        anchors.right: parent.right
        anchors.rightMargin: Theme.space.l
        anchors.verticalCenter: parent.verticalCenter
        readonly property bool chorded: row.kind === "" && row.chord.length > 0
        readonly property string said: row.kind !== "" ? row.kind : row.side
        readonly property bool sided: !chorded && said !== ""

        width: chorded ? keys.width : sided ? side.width : 0
        height: chorded ? keys.height : side.height

        Chord {
            id: keys
            visible: right.chorded
            keys: row.chord
        }

        Text {
            id: side
            visible: right.sided
            // What a row is never takes more than its share of the row, so
            // however long it runs it can't reach the name.
            width: Math.min(implicitWidth, row.width * 0.45)
            elide: Text.ElideRight
            text: right.said
            color: Theme.muted
            font.family: Theme.mono.family
            font.pointSize: Theme.mono.large
        }
    }

    MouseArea {
        anchors.fill: parent
        enabled: !row.quiet
        hoverEnabled: true
        onClicked: row.clicked()
        onPositionChanged: mouse => row.pointed(mapToItem(null, mouse.x, mouse.y))
    }
}
