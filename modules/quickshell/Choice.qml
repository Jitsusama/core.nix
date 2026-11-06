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
import QtQuick.Shapes
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
    readonly property real indent: Math.max(0, Math.min(2, depth)) * glyphColumn
    // A glyph's column: a mark's square cell, a line pitch wide, and the
    // mark rung before its text (VKCF3Z, 2026-11-05). The glyph stands an
    // inset in from the row's left, and what's flush right an inset in from
    // its right, as the card's frame holds all its text (docs/design.md,
    // The Ladder).
    readonly property real glyphColumn: Theme.level.name.pitch + Theme.space.rung.mark
    // How tall the name's capitals stand, which are centred in the row, and
    // its ground set round.
    readonly property real capitals: capitalsFace.tightBoundingRect.height
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
    // The key that carries an act ending something, said at its right in
    // place of its kind or chord while it's the cursor row, since no
    // Return does it: "d disconnect", or the line Return writes, ":lockscreen".
    property string carried
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
    height: 4 * Theme.space.beat
    color: "transparent"
    // Selected, it's filled, unless the list draws the selection itself so
    // it can glide from row to row (Launcher.qml's list).
    property bool fills: true

    // A TextMetrics, which follows the face as it loads, where a
    // FontMetrics' call in a binding is asked once, before it has.
    TextMetrics {
        id: capitalsFace
        text: "H"
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
    }

    // Its ground, close round its ink, as Joel liked a highlight (the
    // aesthetic skill's "Margin round a highlight, not padding inside it"):
    // a mark round the glyph and what's flush right, above the capitals and
    // below the baseline, so more room stands outside it than in.
    Rectangle {
        x: Theme.space.rung.inset - Theme.space.rung.mark
        y: (row.height - row.capitals) / 2 - Theme.space.rung.mark
        width: row.width - 2 * x
        height: row.capitals + 2 * Theme.space.rung.mark
        radius: Theme.inner
        color: row.flashed ? Theme.accent
            : row.selected && row.fills || row.tinted
                ? Qt.alpha(Theme.selection, Theme.floating.selection)
            : "transparent"
    }

    // The glyph's ink and the name's capitals as the faces draw them,
    // measured as they load at many times their size and read back down,
    // since a measure at the size drawn comes in whole pixels, a twelfth of
    // a mark's height.
    readonly property real fine: 8
    TextMetrics {
        id: markInk
        text: row.glyph
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize * row.fine
    }
    // Qt's tight measure of a glyph runs down to the baseline, so a mark
    // standing above it, as the keyboard and people do, measures taller
    // than its ink and centres high (VKCF3Z, 2026-11-05: a pixel at 1.5).
    // Its top is true; its outline gives its height. Measuring the outline
    // rather than Qt's box fixes that cause, where a size or offset tuned
    // per glyph would only hide it until the next mark.
    Shape {
        id: markOutline
        visible: false
        ShapePath {
            PathText {
                text: row.glyph
                font.family: Theme.level.figure.family
                font.pointSize: Theme.level.figure.pointSize * row.fine
            }
        }
    }
    // The ink's height, its top and its middle, at the fine size.
    readonly property real inkTall: markOutline.boundingRect.height > 0
        ? Math.min(markOutline.boundingRect.height, markInk.tightBoundingRect.height)
        : markInk.tightBoundingRect.height
    readonly property real inkMiddle: markInk.tightBoundingRect.y + inkTall / 2
    // Every mark at one optical size beside the type, whatever its shape,
    // so a wide one such as the keyboard doesn't read smaller than the
    // rest down the column (the launcher audit's finding 10): scaled so its
    // ink's area is the size's square, never taller than it, so a narrow
    // mark doesn't outgrow the type, and never wider than its cell, so it
    // keeps its column. The size is fifteen sixteenths of the em, the
    // face's size in pixels at 96 points to the inch: 15 at Figure, a step
    // lighter than the em beside the names (VKCF3Z, 2026-11-05).
    readonly property real markSize: Theme.level.figure.pointSize * 4 / 3 * 15 / 16
    readonly property real markScale: {
        const wide = markInk.tightBoundingRect.width / fine;
        const tall = inkTall / fine;
        return wide <= 0 || tall <= 0 ? 1
            : Math.min(markSize / Math.sqrt(wide * tall), markSize / tall,
                Theme.level.name.pitch / wide);
    }
    // Where the mark's ink centres, down from the row's top: on the name's
    // capitals' middle, its baseline less half their height as drawn on
    // this screen, whose pixels round them: measured at its own scale.
    readonly property real markOn: (height + capitals) / 2
        - capitalsDrawn.tightBoundingRect.height / ratio / 2
    readonly property real ratio: Screen.devicePixelRatio > 0 ? Screen.devicePixelRatio : 1
    TextMetrics {
        id: capitalsDrawn
        text: "H"
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize * row.ratio
    }

    Text {
        x: Theme.space.rung.inset + row.indent
        // Its ink's middle on the capitals'.
        y: row.markOn - row.inkMiddle / row.fine * row.markScale - baselineOffset
        // A swatch is the figure itself, so it stays.
        visible: row.sort !== "figure" || row.swatch
        text: row.glyph
        color: row.flashed && !row.swatch ? row.flashInk
            : row.sort === "holds" && !row.swatch ? Theme.edge : row.hue
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize * row.markScale
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
        x: Theme.space.rung.inset + row.glyphColumn + row.indent
        width: Math.max(0, Math.min(implicitWidth, row.ends - x - Theme.space.rung.groups
            - (row.detail === "" ? 0 : Math.min(countText.implicitWidth, row.width * 0.3)
                + Theme.space.rung.apart)))
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

    // A line of the mono, never drawn, whose baseline the name and its
    // words both sit on, so neither face's line box moves the other, set
    // so the name's capitals stand in the row's middle.
    Text {
        id: monoLine
        y: (row.height + row.capitals) / 2 - baselineOffset
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
        x: nameBox.x + nameBox.width + Theme.space.rung.apart
        // As much as the name leaves, and never less than its share, so a
        // short name's detail (what a category holds) says all it can.
        width: Math.min(implicitWidth, Math.max(row.width * 0.3,
            row.ends - x - Theme.space.rung.groups))
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
        x: right.x - width - (right.width > 0 ? Theme.space.rung.groups : 0)
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
        anchors.rightMargin: Theme.space.rung.inset
        y: monoLine.y + monoLine.baselineOffset
            - (right.chorded ? keys.baselineOffset : sideText.baselineOffset)
        readonly property bool chorded: row.landed === "" && row.carried === ""
            && row.kind === "" && row.chord.length > 0
        readonly property string said: row.landed !== "" ? row.landed
            : row.carried !== "" ? row.carried
            : row.kind !== "" ? row.kind
            : itemised ? Reading.wholeItems(row.sideItems, room,
                words => Math.ceil(sideFace.advanceWidth(words)) + 1)
            : row.side
        readonly property bool sided: !chorded && said !== ""
        readonly property bool itemised: row.landed === "" && row.carried === ""
            && row.kind === "" && row.sideItems.length > 0
        // The room the name leaves whole, its words after it too, which a
        // side of items is fitted to.
        readonly property real room: row.width - Theme.space.rung.inset - nameBox.x
            - nameBox.implicitWidth - Theme.space.rung.groups - (row.detail === "" ? 0
                : Math.min(countText.implicitWidth, row.width * 0.3) + Theme.space.rung.apart)
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
            row.width - nameBox.x + row.indent - nameBox.implicitWidth
            - Theme.space.rung.groups - row.glyphColumn - Theme.space.rung.inset
            - (row.detail === "" ? 0
                : Math.min(countText.implicitWidth, row.width * 0.3) + Theme.space.rung.apart)))
        // As wide as the widest beside a graph in the list, so the graphs
        // line up, and never more than what's said may take; for a row
        // without one, as wide as the widest said here in this step, within
        // its share.
        readonly property real column: row.graphed
            ? Math.min(row.figures * noteFace.advanceWidth, row.width * 0.45)
            : right.sided && (!right.itemised || right.squeezed)
                ? Math.min(row.sides * noteFace.advanceWidth, right.share) : 0

        // Whether a figure stands before the chord, as a collection's count
        // does, so a count is said at a row's right even where its chord
        // is (VKCF3Z, 2026-11-05: "12 \u00b7 \u2699 + 0").
        readonly property bool counted: chorded && said !== ""
        width: Math.max(column, chorded ? keys.width
                + (counted ? beforeKeys.implicitWidth + Theme.space.rung.mark : 0)
            : sided ? sideText.width : 0)
        height: chorded ? keys.height : sideText.height

        Chord {
            id: keys
            anchors.right: parent.right
            visible: right.chorded
            keys: row.chord
            colour: row.flashed ? row.flashInk : row.selected ? Theme.text : Theme.muted
        }

        // The figure before the chord, in Note, joined to it as the status
        // line joins its facts: a muted dot a mark from each, measured to
        // the dot's ink, so a dot means one thing across the card (VKCF3Z,
        // 2026-11-05).
        // The dot's ink, and where the count's ends short of its advance
        // by its last letter's bearing, measured at the fine size and read
        // back down, since a measure at the size drawn comes in whole
        // pixels, which a screen at 1.5 or 2 draws finer than.
        TextMetrics {
            id: dotInk
            text: "\u00b7"
            font.family: Theme.level.note.family
            font.italic: Theme.level.note.italic
            font.weight: Theme.level.note.weight
            font.pointSize: Theme.level.note.pointSize * row.fine
        }
        TextMetrics {
            id: countInk
            text: right.said
            font.family: Theme.level.note.family
            font.italic: Theme.level.note.italic
            font.weight: Theme.level.note.weight
            font.pointSize: Theme.level.note.pointSize * row.fine
        }
        // From the figure's width as drawn rather than its fine advance,
        // since a face hinted fully draws its advance in whole pixels.
        readonly property real countBearing: right.said === "" ? 0
            : Math.max(0, countFigure.implicitWidth - (countInk.tightBoundingRect.x
                + countInk.tightBoundingRect.width) / row.fine)
        Row {
            id: beforeKeys
            anchors.right: keys.left
            anchors.rightMargin: Theme.space.rung.mark
            anchors.baseline: keys.baseline
            baselineOffset: countFigure.baselineOffset
            spacing: Theme.space.rung.mark - right.countBearing
            visible: right.counted

            Text {
                id: countFigure
                text: right.said
                textFormat: Text.PlainText
                color: row.flashed ? row.flashInk : row.selected ? Theme.text : Theme.muted
                font.family: Theme.level.note.family
                font.italic: Theme.level.note.italic
                font.pointSize: Theme.level.note.pointSize
                font.weight: Theme.level.note.weight
                font.variableAxes: Theme.level.note.variableAxes
                font.features: Theme.level.note.features
            }
            Item {
                width: dotInk.tightBoundingRect.width / row.fine
                height: dotText.height

                Text {
                    id: dotText
                    x: -dotInk.tightBoundingRect.x / row.fine
                    text: "\u00b7"
                    color: row.flashed ? row.flashInk : Theme.muted
                    font: countFigure.font
                }
            }
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
