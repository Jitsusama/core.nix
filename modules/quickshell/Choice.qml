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
//
// Its glyph says what sort of row it is, so a thing to do never reads like
// a figure (docs/design.md, Shape and Kinds): an act wears its glyph in its
// hue, a row that holds more, an answer's parts or a collection, wears it
// in the edge colour, and a figure, a value an answer's part only says,
// wears none, its name keeping the column the glyphs leave. A row with a
// graph draws it between its words and what it is, in one column down the
// list, its name giving way to it first.
// Not Row, which is QtQuick's own and would silently win.
pragma ComponentBehavior: Bound
import QtQuick
import "Reading.js" as Reading

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
    // for a noun what would have been on the right. Each is named through
    // the row, since a bare name here would find an id before a property.
    readonly property string detail: [row.also !== "" ? `\u201c${row.also}\u201d` : "",
        row.count, row.kind !== "" ? row.side : ""].filter(part => part !== "").join(" \u00b7 ")
    // Said by the shell rather than found, so it's muted and can't be picked.
    property bool quiet: false
    // What sort of row it is: "act", "holds" or "figure".
    property string sort: "act"
    // Its glyph wears what it says, as a colour's swatch does, so it's
    // never dimmed or left off.
    property bool swatch: false
    // Its graph, as Graph.qml draws one, or null.
    property var graph: null
    // How many characters the widest of what the list's graphed rows say
    // at their right runs to, so every graph in the list stands in one
    // column clear of them all.
    property int figures: 0
    // The same for every row in the list, graphed or not, so what a row is
    // takes the room the widest has needed in this step, and where its
    // name is cut holds still as what it says there changes.
    property int sides: 0
    readonly property bool graphed: graph !== null && graph !== undefined
    // Where the name and its detail have to stop: the graph, or else what
    // the row is.
    readonly property real ends: graphed ? drawing.x : right.x

    signal clicked
    // The pointer is over the row, at this place in the window.
    signal pointed(point at)

    width: parent.width
    height: Theme.space.xxxl
    radius: Theme.inner
    // Selected, it's filled, unless the list draws the selection itself so
    // it can glide from row to row (Launcher.qml's list).
    property bool fills: true
    color: selected && fills ? Qt.alpha(Theme.selection, Theme.floating.selection)
        : "transparent"

    Text {
        x: Theme.space.l
        anchors.verticalCenter: parent.verticalCenter
        // A swatch is the figure itself, so it stays.
        visible: row.sort !== "figure" || row.swatch
        text: row.glyph
        color: row.sort === "holds" && !row.swatch ? Theme.edge : row.hue
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }

    // The name's room, which it's cut to. Selected, a name too long for
    // it reads on: after a rest to read how it starts, it slides at a
    // steady reading pace until its end shows, rests there, then glides
    // back to its start and rests again, for as long as it's selected.
    // Only the name moves, inside its own room, so nothing beside it does,
    // and the row isn't any wider for it (Joel, 2026-10-25: titles "get
    // elipsed really quick").
    Item {
        id: nameBox
        x: Theme.space.l + Theme.space.xxl
        width: Math.max(0, Math.min(nameText.implicitWidth, row.ends - x - Theme.space.xxl
            - (row.detail === "" ? 0 : Math.min(countText.implicitWidth, row.width * 0.3)
                + Theme.space.m)))
        height: nameText.height
        anchors.verticalCenter: parent.verticalCenter
        clip: scrolls

        // How far past its room the name runs, when it does.
        readonly property real over: Math.max(0, nameText.implicitWidth - width)
        readonly property bool scrolls: row.selected && over > 0
        // A reading pace of twelve characters a second, held between soft
        // ends (Reading.js). A short rest to see how it starts, and a
        // longer one at its end to read that (Joel, 2026-10-29: the slide
        // stuttered, was too slow, and started and stopped abruptly).
        readonly property real pace: 12 * figureFace.advanceWidth
        readonly property var slide: Reading.slide(over, pace)
        readonly property int restFirst: Math.round(500 * Theme.motion.slowdown)
        readonly property int restLast: Math.round(1000 * Theme.motion.slowdown)

        Text {
            id: nameText
            width: nameBox.scrolls ? implicitWidth : nameBox.width
            text: row.name
            textFormat: row.selected || row.quiet ? Text.PlainText : Text.StyledText
            elide: nameBox.scrolls ? Text.ElideNone : Text.ElideRight
            color: row.quiet ? Theme.muted : row.selected ? Theme.strong : Theme.text
            font.family: Theme.mono.family
            font.pointSize: Theme.mono.large
            font.weight: row.selected ? Font.Medium : Font.Normal
        }

        // Animators, which move the name on the render thread, so the
        // slide keeps every frame while the shell is busy making rows
        // again, as it is each second in Machine (Joel, 2026-10-29:
        // "scrolling long lines feels a bit janky"). An animator says the
        // name's x back only when it stops, so each move runs on its own,
        // the rests between them timed here, rather than looping in one
        // group that would never say where the name rests.
        onScrollsChanged: nameBox.scrolls ? nameBox.read() : nameBox.unread()
        Component.onCompleted: {
            if (nameBox.scrolls)
                nameBox.read();
        }
        // Which move the rest ends in: out to the name's end, or back.
        property bool outward: true
        function read() {
            nameBox.outward = true;
            resting.interval = nameBox.restFirst;
            resting.restart();
        }
        function unread() {
            resting.stop();
            slideOut.stop();
            slideBack.stop();
            nameText.x = 0;
        }

        Timer {
            id: resting
            onTriggered: (nameBox.outward ? slideOut : slideBack).start()
        }
        XAnimator {
            id: slideOut
            target: nameText
            from: 0
            to: -nameBox.over
            duration: Math.round(1000 * Theme.motion.slowdown * nameBox.slide.seconds)
            easing.type: Easing.BezierSpline
            easing.bezierCurve: nameBox.slide.curve
            onFinished: {
                if (!nameBox.scrolls)
                    return;
                nameBox.outward = false;
                resting.interval = nameBox.restLast;
                resting.restart();
            }
        }
        XAnimator {
            id: slideBack
            target: nameText
            to: 0
            duration: Theme.motion.medium
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Theme.motion.curve
            onFinished: {
                if (nameBox.scrolls)
                    nameBox.read();
            }
        }
    }

    Text {
        id: countText
        x: nameBox.x + nameBox.width + Theme.space.m
        // As much as the name leaves, and never less than its share, so a
        // short name's detail (what a category holds) says all it can.
        width: Math.min(implicitWidth, Math.max(row.width * 0.3, row.ends - x - Theme.space.xxl))
        elide: Text.ElideRight
        anchors.verticalCenter: parent.verticalCenter
        visible: row.detail !== ""
        text: row.detail
        color: Theme.muted
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }

    Graph {
        id: drawing
        x: right.x - width - (right.width > 0 ? Theme.space.xl : 0)
        anchors.verticalCenter: parent.verticalCenter
        reading: row.graph
        selected: row.selected
    }

    // A figure's character, to measure the graphs' column by.
    TextMetrics {
        id: figureFace
        text: "0"
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
        // What a row is never takes more than its share of the row, so
        // however long it runs it can't reach the name, and it gives the
        // name its room first, down to a fifth of the row: a window's
        // title is what Joel looks for, its program and place only help
        // (Joel, 2026-10-25: titles "get elipsed really quick").
        readonly property real share: Math.min(row.width * 0.45, Math.max(row.width * 0.2,
            row.width - nameBox.x - nameText.implicitWidth - 2 * Theme.space.xxl
            - Theme.space.l - (row.detail === "" ? 0
                : Math.min(countText.implicitWidth, row.width * 0.3) + Theme.space.m)))
        // As wide as the widest beside a graph in the list, so the graphs
        // line up, and never more than what's said may take; for a row
        // without one, as wide as the widest said here in this step, within
        // its share.
        readonly property real column: row.graphed
            ? Math.min(row.figures * figureFace.advanceWidth, row.width * 0.45)
            : right.sided ? Math.min(row.sides * figureFace.advanceWidth, right.share) : 0

        width: Math.max(column, chorded ? keys.width : sided ? sideText.width : 0)
        height: chorded ? keys.height : sideText.height

        Chord {
            id: keys
            anchors.right: parent.right
            visible: right.chorded
            keys: row.chord
        }

        Text {
            id: sideText
            anchors.right: parent.right
            visible: right.sided
            width: Math.min(implicitWidth, right.share)
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
