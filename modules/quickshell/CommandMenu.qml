// The command line's menu (CommandLine.qml), as his completion menu shows
// over his command line: it rises over the bottom of the rows the moment :
// is typed, so the cursor row stays in view above it, and holds ten lines
// at most, as his pumheight does (modules/neovim/init.lua), scrolling to
// keep the chosen one in view. A hairline splits it from the rows, as it
// splits the field from them, rather than a box of its own.
//
// Each line reads `disc[onnect]   Disconnect It   d`: the command in Vim's
// help notation on the names' column, with what its line names after it
// where it takes an argument (`after`, as e[dit] Notices or g[lobal]/pat/cmd), the act's title on a column of its
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
    // A line's pitch, two beats, so the menu sits on the card's grid as the
    // drawer's lines do (VKCF3Z, 2026-11-05).
    readonly property real pitch: 2 * Theme.space.beat
    readonly property int showing: Math.min(lines.length, most)
    // Where its commands stand from its left: on the names' column, a
    // glyph's column past the inset, unless its surface's names stand
    // elsewhere, as the launcher's do past its gutter.
    property real commandsAt: Theme.space.rung.inset + Theme.level.name.pitch
        + Theme.space.rung.mark
    // The room above its first line and below its last: a beat, so its last
    // line's middle lies a row's middle above its bottom, and its baselines
    // fall on the rows' grid, and the chosen line's ground stands clear of
    // the hairline.
    readonly property real room: Theme.space.beat

    implicitHeight: showing === 0 ? 0 : showing * pitch + Theme.border + 2 * room
    visible: showing > 0
    clip: true

    FontMetrics {
        id: note
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
    }

    // A TextMetrics, which follows the face as it loads, where a
    // FontMetrics' call in a binding is asked once, before it has.
    TextMetrics {
        id: noteCaps
        text: "H"
        font: note.font
    }

    // The commands' column, as wide as the widest command the menu holds.
    // An argument listed after a space may be wider than every command.
    readonly property real commandsWide: menu.all.concat(menu.lines).reduce((widest, entry) =>
        Math.max(widest, note.advanceWidth(menu.written(entry))), 0)

    // A line's command as written: as its words call it (Commands.called),
    // and what follows it.
    function written(entry) {
        return Commands.called(entry) + (entry.after ?? "");
    }
    readonly property real titlesAt: commandsAt + Math.ceil(commandsWide)
        + Theme.space.rung.groups
    // How tall a Note line's capitals stand, which the chosen line's ground
    // is set round.
    readonly property real capitals: noteCaps.tightBoundingRect.height

    // Inset to inset, spanning the ink, as every rule of a card does.
    Rectangle {
        visible: menu.split
        x: Theme.space.rung.inset
        width: parent.width - 2 * x
        height: Theme.border
        color: Theme.edge
    }

    ListView {
        id: list
        y: Theme.border + menu.room
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

            // Close round its ink, a mark round the command and the key,
            // above the capitals and below the baseline, as the rows'
            // ground is (Choice.qml).
            Rectangle {
                x: menu.commandsAt - Theme.space.rung.mark
                y: command.y + command.baselineOffset - menu.capitals - Theme.space.rung.mark
                width: key.x + key.width + Theme.space.rung.mark - x
                height: menu.capitals + 2 * Theme.space.rung.mark
                visible: entry.picked
                radius: Theme.inner
                color: Qt.alpha(Theme.selection, Theme.floating.selection)
            }

            Text {
                id: command
                x: menu.commandsAt
                anchors.verticalCenter: parent.verticalCenter
                text: menu.written(entry.line)
                textFormat: Text.PlainText
                color: entry.ink
                font.family: Theme.level.note.family
                font.pointSize: Theme.level.note.pointSize
                font.weight: Theme.level.note.weight
            }

            Text {
                x: menu.titlesAt
                width: key.x - x - Theme.space.rung.groups
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
                x: parent.width - Theme.space.rung.inset - implicitWidth
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
