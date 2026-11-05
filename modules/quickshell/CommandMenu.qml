// The command line's menu (CommandLine.qml), as his completion menu shows
// over his command line: it rises over the bottom of the rows the moment :
// is typed, so the cursor row stays in view above it, and holds ten lines
// at most, as his pumheight does (modules/neovim/init.lua), scrolling to
// keep the chosen one in view. A hairline splits it from the rows, as it
// splits the field from them, rather than a box of its own.
//
// Each line reads `disc[onnect]   Disconnect It   d`: the command in Vim's
// help notation on the names' column, the act's title on a column of its
// own past the widest command, and its key flush right, all at Note in
// muted, so the menu teaches all three ways. The chosen line takes the
// selection's ground, its words in text, since muted isn't drawn there
// (docs/design.md, Colour). Only the lines in view are made, as a ListView
// makes them.
pragma ComponentBehavior: Bound
import QtQuick
import "Commands.js" as Commands

Item {
    id: menu

    // Every line the menu could show, which sets the columns, so they hold
    // still as typing narrows it.
    property var all: []
    // The lines it shows now, and the one chosen, -1 for none.
    property var lines: []
    property int chosen: -1
    // How many lines show at most, as his pumheight.
    property int most: 10
    // Whether rows lie behind it for its hairline to split it from; over a
    // viewer none do, and the field's own hairline already sits above it.
    property bool split: true
    // A line's pitch: a Note line and the room the space scale gives it.
    readonly property real pitch: Theme.space.xl
    readonly property int showing: Math.min(lines.length, most)

    implicitHeight: showing === 0 ? 0 : showing * pitch + Theme.border + 2 * Theme.space.xs
    visible: showing > 0
    clip: true

    FontMetrics {
        id: note
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
    }

    // The commands' column, as wide as the widest command the menu holds.
    readonly property real commandsWide: menu.all.reduce((widest, entry) =>
        Math.max(widest, note.advanceWidth(Commands.notation(entry.command, entry.short))), 0)
    readonly property real commandsAt: Theme.space.l + Theme.space.xxl
    readonly property real titlesAt: commandsAt + Math.ceil(commandsWide) + Theme.space.xl

    Rectangle {
        visible: menu.split
        width: parent.width
        height: Theme.border
        color: Theme.edge
    }

    ListView {
        id: list
        y: Theme.border + Theme.space.xs
        width: parent.width
        height: menu.showing * menu.pitch
        model: menu.lines.length
        interactive: false
        boundsBehavior: Flickable.StopAtBounds
        currentIndex: menu.chosen
        highlightFollowsCurrentItem: false
        onCurrentIndexChanged: {
            if (currentIndex >= 0)
                positionViewAtIndex(currentIndex, ListView.Contain);
            else
                positionViewAtBeginning();
        }

        delegate: Item {
            id: entry

            required property int index
            readonly property var line: menu.lines[index] ?? ({})
            readonly property bool picked: index === menu.chosen
            readonly property color ink: picked ? Theme.text : Theme.muted

            width: list.width
            height: menu.pitch

            Rectangle {
                anchors.fill: parent
                visible: entry.picked
                radius: Theme.inner
                color: Qt.alpha(Theme.selection, Theme.floating.selection)
            }

            Text {
                id: command
                x: menu.commandsAt
                anchors.verticalCenter: parent.verticalCenter
                text: Commands.notation(entry.line.command ?? "", entry.line.short ?? "")
                textFormat: Text.PlainText
                color: entry.ink
                font.family: Theme.level.note.family
                font.pointSize: Theme.level.note.pointSize
                font.weight: Theme.level.note.weight
            }

            Text {
                x: menu.titlesAt
                width: key.x - x - Theme.space.xl
                anchors.baseline: command.baseline
                text: entry.line.title ?? ""
                textFormat: Text.PlainText
                elide: Text.ElideRight
                color: entry.ink
                font.family: Theme.level.note.family
                font.pointSize: Theme.level.note.pointSize
                font.weight: Theme.level.note.weight
            }

            Text {
                id: key
                x: parent.width - Theme.space.l - implicitWidth
                anchors.baseline: command.baseline
                text: entry.line.key ?? ""
                textFormat: Text.PlainText
                color: entry.ink
                font.family: Theme.level.note.family
                font.pointSize: Theme.level.note.pointSize
                font.weight: Theme.level.note.weight
            }
        }
    }
}
