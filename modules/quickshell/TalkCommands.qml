// Super+S's : menu, his completion menu under the line while : is open, as the launcher's and
// the held notices' command lines list theirs (CommandLine.qml draws none of its own): under a
// hairline, each command a launcher row (Choice.qml), its name in Vim's help notation on the
// text column under what's typed, what it does in its words, and the line Tab, the arrows,
// Ctrl+N and Ctrl+P chose on the selection's ground, from the frame's inside edge to its other.
// Without it, : typed blind: Tab wrote a command in with nothing saying what it could be
// (Joel, 2026-11-14 16:25: "the void lacks any Ex command completion like other windows do").
// It stands in the talks' place while : is open, as the held notices' menu stands in their
// lines', and as many lines show as the frame has room for, the chosen one kept in view.
pragma ComponentBehavior: Bound
import QtQuick
import "Commands.js" as Commands

Item {
    id: commands

    // The menu as typing narrows it (CommandLine.shown), and the line chosen in it, or -1.
    property var shown: []
    property int chosen: -1
    // How far left of it the frame's inside edge is, which the ground reaches out to.
    property real reach: 0
    // How tall it may stand, its hairline included.
    property real room: 0
    // Whether the line is seen, so a cut name never reads on unseen (Choice.qml).
    property bool up: false

    readonly property real rowTall: 4 * Theme.space.beat
    // What a row leaves under its capitals, which the hairline stands as far over, so the
    // room over it and under it is the same to the ink.
    readonly property real leaves: (rowTall - capitals.tightBoundingRect.height) / 2
    readonly property real rowsFrom: Math.round(Theme.space.rung.tie + leaves) + Theme.border
    readonly property int fits: Math.max(1, Math.floor((room - rowsFrom) / rowTall))
    // How many rows stand wholly in view, which the line's showing says.
    readonly property int drawn: Math.min(shown.length, fits)

    height: shown.length === 0 ? 0 : rowsFrom + drawn * rowTall

    // A line's name as the menu writes it: its command in Vim's help notation, then what
    // an argument's line writes after it.
    function named(entry) {
        return Commands.called(entry) + (entry.after ?? "");
    }

    TextMetrics {
        id: capitals
        text: "H"
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
        font.weight: Theme.level.name.weight
    }

    Rectangle {
        y: Math.round(Theme.space.rung.tie + commands.leaves)
        width: parent.width
        height: Theme.border
        color: Theme.edge
    }

    ListView {
        id: rows
        x: -commands.reach
        y: commands.rowsFrom
        width: commands.width + 2 * commands.reach
        height: commands.drawn * commands.rowTall
        clip: true
        interactive: false
        boundsBehavior: Flickable.StopAtBounds
        model: commands.shown
        currentIndex: commands.chosen
        onCurrentIndexChanged: {
            if (currentIndex >= 0)
                positionViewAtIndex(currentIndex, ListView.Contain);
            else
                positionViewAtBeginning();
        }

        delegate: Choice {
            id: command

            required property int index
            required property var modelData
            readonly property string named: commands.named(modelData)

            width: rows.width
            height: commands.rowTall
            selected: command.index === rows.currentIndex
            shown: commands.up
            name: command.named
            plainName: command.named
            count: command.modelData.title ?? ""
            side: command.modelData.key ?? ""
        }
    }
}
