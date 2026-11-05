// A row of a palette's list, the launcher's or the margin's panel: its glyph
// in a column of its own, its name, and on the right its chord or what it
// is, muted (docs/design.md, Shape and Kinds).
//
// Its type ranks it (docs/design.md, Type): the name at Name, its words at
// Words, and what it is or its chord at Note, a step smaller, all on the
// name's baseline. Selection changes only colour and fill, never weight,
// so a weight means only a level.
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
    // Just copied by y, for the key timing's yank-flash: his Neovim's
    // on_yank draws a yank in IncSearch, linked to CurSearch, the accent's
    // ground under its own ink (modules/neovim/colors/jitsusama.lua), so
    // the row does too, over the selection, which it would vanish into.
    // Only fill and colour change, never weight.
    property bool flashed: false
    // The ink on the flash's ground, for every word and glyph the row says.
    readonly property color flashInk: Theme.on_accent
    // Taken by visual, or what a pending operator will take: it wears the
    // selection's ground of its own, as his Neovim's Visual does, so the
    // cursor row, the list's glide under it too, stands deeper than the
    // rest. Only the fill changes, never weight.
    property bool tinted: false
    // How many steps in it sits under the row it belongs to, as a session
    // under the one orchestrating it: 0, 1 or 2 (docs/design.md keeps no
    // layer deeper). Each step is the glyph's column, so a row's glyph
    // stands where the name above it starts, and nothing nearly lines up
    // (agents-tree-look.md). Its name's room gives the step up, never
    // what it says at its right, and the selection keeps the whole row.
    property int depth: 0
    readonly property real indent: Math.max(0, Math.min(2, depth)) * Theme.space.xxl
    // Whether the row can be seen. A closed launcher only fades its card
    // out, keeping its selection, so without this a cut row would read on
    // unseen, drawing every frame of it while nothing is shown.
    property bool shown: true
    property string glyph
    property color hue
    property string name
    // The name as it reads, when `name` carries the launcher's marks.
    property string plainName: name
    // The runs of the name's letters the find matched, [start, end] each
    // (Fuzzy.runs), which take Search's wash, as his Neovim's hlsearch
    // washes a match: the selected row's too, on a ground a shade past the
    // selection. The ink is the launcher's to give (Launcher.marked).
    property var finds: []
    // Where the name is washed, each run's start, width and ground.
    readonly property var washed: nameBox.washed
    property var chord: []
    property string side
    // What it says at its right as items, each `{ said, keep }` in the
    // order said, as a session's address, state and cost are: given, they
    // give way whole, the least needed first, while the name is cut, so
    // the name keeps its letters while more than one is said and the side
    // is never cut mid-word or slid from its left (Reading.js's
    // wholeItems). Empty, `side` is said as it is, cut to its share.
    property var sideItems: []
    // What a second Return does, said at its right in the accent in place
    // of its kind or chord, once Return from the find landed on it rather
    // than doing it ("\u21b5 turn off"), until the next key.
    property string landed
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
    // Its name is the shell speaking to Joel, in the voice's face, as an
    // empty place's words are (look 3's piece 1).
    property bool voiced: false
    // What sort of row it is: "act", "holds" or "figure".
    property string sort: "act"
    // Its glyph wears what it says, as a colour's swatch does, so it's
    // never dimmed or left off.
    property bool swatch: false
    // Its graph, as Graph.qml draws one, or null.
    property var graph: null
    // Whether what it says warns, as an update waiting does: what it says
    // at its right takes the warning colour, and its graph all of it, one
    // flag for both so they never disagree (docs/design.md, Kinds).
    property bool warns: false
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

    width: parent.width
    height: Theme.space.xxxl
    radius: Theme.inner
    // Selected, it's filled, unless the list draws the selection itself so
    // it can glide from row to row (Launcher.qml's list).
    property bool fills: true
    color: flashed ? Theme.accent
        : selected && fills || tinted ? Qt.alpha(Theme.selection, Theme.floating.selection)
        : "transparent"

    Text {
        x: Theme.space.l + row.indent
        anchors.verticalCenter: parent.verticalCenter
        // A swatch is the figure itself, so it stays.
        visible: row.sort !== "figure" || row.swatch
        text: row.glyph
        color: row.flashed && !row.swatch ? row.flashInk
            : row.sort === "holds" && !row.swatch ? Theme.edge : row.hue
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
    }

    // Selected, a name too long for its room reads on (ReadsOn.qml), and
    // so does what the row says at its right. With both cut they take
    // turns, the name first, so only one thing moves at a time and each
    // reads whole; given up by the selection, or no longer seen, both are
    // back at their start at once.
    readonly property bool reads: row.selected && row.shown
    // Where its name has its baseline, down from its top, which the
    // launcher's drawer sets its title on.
    readonly property real nameBaseline: nameBox.y + nameBox.baselineOffset
    // Where its words have theirs, which is the name's.
    readonly property real detailBaseline: countText.y + countText.baselineOffset
    // How far its words stand from its name.
    readonly property real detailGap: countText.x - nameBox.x - nameBox.width
    readonly property string detailFace: countText.font.family
    readonly property bool nameCut: nameBox.over > 0
    readonly property bool sideCut: right.sided && sideText.over > 0
    property bool sidesTurn: false
    onReadsChanged: row.sidesTurn = false

    // The name's room, which it's cut to.
    ReadsOn {
        id: nameBox
        x: Theme.space.l + Theme.space.xxl + row.indent
        width: Math.max(0, Math.min(implicitWidth, row.ends - x - Theme.space.xxl
            - (row.detail === "" ? 0 : Math.min(countText.implicitWidth, row.width * 0.3)
                + Theme.space.m)))
        // On the mono's line, so the voice sits where the mono's name
        // would, the faces' line boxes differing.
        anchors.baseline: monoLine.baseline
        level: row.voiced ? Theme.level.voice : Theme.level.name
        text: row.name
        plain: row.plainName
        // The flash takes the row's ground, so a find's wash, which would
        // lie on it in the selection's colour and vanish, gives way.
        finds: row.flashed ? [] : row.finds
        washedSelected: row.selected
        format: row.selected || row.quiet || row.flashed ? Text.PlainText : Text.StyledText
        color: row.flashed ? row.flashInk : row.quiet ? Theme.muted
            : row.selected ? Theme.strong : Theme.text
        figure: figureFace.advanceWidth
        reading: row.reads && !(row.sideCut && row.sidesTurn)
        onThrough: {
            if (row.sideCut)
                row.sidesTurn = true;
        }
    }

    // A line of the mono through the row's middle, never drawn, whose
    // baseline the name and its words both sit on, so neither face's line
    // box moves the other.
    Text {
        id: monoLine
        anchors.verticalCenter: parent.verticalCenter
        visible: false
        text: "0"
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
    }

    // What the row holds, or its note, in the serif upright at the name's
    // size, as the reader sets its text, so the eye finds the name, in the
    // mono, first (look 3's gap 1; no sans, 4 November). Muted,
    // and on the selection the text colour, since muted there reads under
    // the Lc 60 any text needs.
    Text {
        id: countText
        x: nameBox.x + nameBox.width + Theme.space.m
        // As much as the name leaves, and never less than its share, so a
        // short name's detail (what a category holds) says all it can.
        width: Math.min(implicitWidth, Math.max(row.width * 0.3, row.ends - x - Theme.space.xxl))
        elide: Text.ElideRight
        anchors.baseline: monoLine.baseline
        visible: row.detail !== ""
        text: row.detail
        color: row.flashed ? row.flashInk : row.selected ? Theme.text : Theme.muted
        font.family: Theme.level.words.family
        font.pointSize: Theme.level.words.pointSize
        font.variableAxes: Theme.level.words.variableAxes
    }

    Graph {
        id: drawing
        x: right.x - width - (right.width > 0 ? Theme.space.xl : 0)
        anchors.verticalCenter: parent.verticalCenter
        reading: row.warns && row.graphed ? Object.assign({}, row.graph, { warn: true }) : row.graph
        selected: row.selected
    }

    // The face what the row is in, at Note, to fit a side of items to its
    // room.
    FontMetrics {
        id: sideFace
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
    }

    // A figure's character, to measure the graphs' column by.
    TextMetrics {
        id: figureFace
        text: "0"
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
    }

    // A figure's character at Note, what the row says at its right, to
    // measure that column by.
    TextMetrics {
        id: noteFace
        text: "0"
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
    }

    // The chord, or what the row is, a step smaller than the name and on
    // its baseline, so the row reads as one line.
    Item {
        id: right
        anchors.right: parent.right
        anchors.rightMargin: Theme.space.l
        y: monoLine.y + monoLine.baselineOffset
            - (right.chorded ? keys.baselineOffset : sideText.baselineOffset)
        readonly property bool chorded: row.landed === "" && row.kind === ""
            && row.chord.length > 0
        readonly property string said: row.landed !== "" ? row.landed
            : row.kind !== "" ? row.kind
            : itemised ? Reading.wholeItems(row.sideItems, room,
                words => Math.ceil(sideFace.advanceWidth(words)) + 1)
            : row.side
        readonly property bool sided: !chorded && said !== ""
        readonly property bool itemised: row.landed === "" && row.kind === ""
            && row.sideItems.length > 0
        // The room the name leaves whole, its words after it too, which a
        // side of items is fitted to.
        readonly property real room: row.width - Theme.space.l - nameBox.x
            - nameBox.implicitWidth - Theme.space.xxl - (row.detail === "" ? 0
                : Math.min(countText.implicitWidth, row.width * 0.3) + Theme.space.m)
        // A side of items down to its last that still leaves the name no
        // room whole: the name is cut, so where it's cut holds still as
        // that item changes, by the column, as any side's does.
        readonly property bool squeezed: itemised
            && Math.ceil(sideFace.advanceWidth(said)) + 1 > room
        // What a row is never takes more than its share of the row, so
        // however long it runs it can't reach the name, and it gives the
        // name its room first, down to a fifth of the row: a window's
        // title is what Joel looks for, its program and place only help
        // (Joel, 2026-10-25: titles "get elipsed really quick"). It's
        // reckoned from where a name starts at no depth, so a row a step in
        // says as much at its right as the row above it.
        readonly property real share: Math.min(row.width * 0.45, Math.max(row.width * 0.2,
            row.width - nameBox.x + row.indent - nameBox.implicitWidth - 2 * Theme.space.xxl
            - Theme.space.l - (row.detail === "" ? 0
                : Math.min(countText.implicitWidth, row.width * 0.3) + Theme.space.m)))
        // As wide as the widest beside a graph in the list, so the graphs
        // line up, and never more than what's said may take; for a row
        // without one, as wide as the widest said here in this step, within
        // its share.
        readonly property real column: row.graphed
            ? Math.min(row.figures * noteFace.advanceWidth, row.width * 0.45)
            : right.sided && (!right.itemised || right.squeezed)
                ? Math.min(row.sides * noteFace.advanceWidth, right.share) : 0

        width: Math.max(column, chorded ? keys.width : sided ? sideText.width : 0)
        height: chorded ? keys.height : sideText.height

        Chord {
            id: keys
            anchors.right: parent.right
            visible: right.chorded
            keys: row.chord
            colour: row.flashed ? row.flashInk : row.selected ? Theme.text : Theme.muted
        }

        // What the row is, cut to its share, reads on as the name does,
        // after it.
        ReadsOn {
            id: sideText
            anchors.right: parent.right
            visible: right.sided
            width: right.itemised ? implicitWidth : Math.min(implicitWidth, right.share)
            text: right.said
            wholeWords: true
            endsRight: true
            level: Theme.level.note
            // Muted reads under the Lc 60 any text needs on the selection.
            color: row.flashed ? row.flashInk
                : row.landed !== "" ? Theme.accent
                : row.warns && row.kind === "" ? Theme.warning
                : row.selected ? Theme.text : Theme.muted
            figure: noteFace.advanceWidth
            reading: row.reads && right.sided && (!row.nameCut || row.sidesTurn)
            onThrough: {
                if (row.nameCut)
                    row.sidesTurn = false;
            }
        }
    }

    // A click does what Return would; the pointer resting or passing over
    // a row never selects it, since the launcher is driven by the keys
    // (Joel, 2026-10-29: "the launcher window is not meant to be mouse
    // navigated").
    MouseArea {
        anchors.fill: parent
        enabled: !row.quiet
        onClicked: row.clicked()
    }
}
