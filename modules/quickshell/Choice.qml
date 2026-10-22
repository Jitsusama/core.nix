// A row of a palette's list, the launcher's or the margin's panel: its glyph
// in a column of its own, its name, and on the right its chord or what it
// is, muted (docs/design.md, Shape and Kinds).
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
            - (row.count === "" ? 0 : countText.implicitWidth + Theme.space.m)))
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
        anchors.verticalCenter: parent.verticalCenter
        visible: row.count !== ""
        text: row.count
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
        readonly property bool chorded: row.chord.length > 0
        readonly property bool sided: !chorded && row.side !== ""

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
            text: row.side
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
