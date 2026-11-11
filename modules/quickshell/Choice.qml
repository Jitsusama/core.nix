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
// muted, and a figure, a value an answer's part only says, wears none, its
// name keeping the column the glyphs leave; but a warning's or an alert's
// hue is kept, and a figure marked wears its mark, so a row says how it
// stands by its mark as a budget's do. A row with a
// graph draws it between its words and what it is, in one column down the
// list, its name giving way to it first.
// Not Row, which is QtQuick's own and would silently win.
pragma ComponentBehavior: Bound
import QtQuick
import "Measures.js" as Measures
import "Reading.js" as Reading
import "Columns.js" as Columns

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
    readonly property real glyphColumn: Columns.across(0, 0, Columns.ofMark(Theme.level.name.pitch),
        Theme.space.rung).text
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
    // of its side, kind or chord, once Return from the find landed on it
    // rather than doing it ("\u21b5 turn off"), until the next key. The key
    // that carries an act ending something is the foot's alone, never the
    // row's, so the cursor row reads as its neighbours do (VKCF3Z,
    // 2026-11-09: a key is taught once, in the foot).
    property string landed
    // What the cursor row says of how it stands, as a battery's "full in
    // 40 min", at the far end in Note, before what it says at its right,
    // so that keeps the inset and only this comes and goes; with nothing
    // there, it takes the inset itself. It's a fact about the row, so it's
    // said on the row rather than in the head (VKCF3Z, 2026-11-06). The
    // name never gives way to it: it has what the name and its words
    // leave, and a long one is cut from its end.
    property string say
    // The cursor row's note, as Wi-Fi's "a password": after its name and
    // what it holds, in their face and muted, never at the right, which
    // says how it stands and never gives way to it. It has what the name,
    // its words and the say leave, giving way first by whole items from
    // its end (VKCF3Z, 2026-11-09 [6918348b]).
    property string note
    // A word after the name, muted: "here" beside a workspace.
    property string count
    // Whether its side is its need, as a player's "paused \u00b7 0:00 of
    // 1:00" is: said whole, never cut, the name and the note giving way
    // to it (VKCF3Z, 2026-11-08).
    property bool sideWhole: false
    // The note as items, each `{ said, keep }`, beside a whole side: it
    // gives way to the side, the least kept item first, one naming a
    // `file` giving way from its middle once it's the last (Reading.js's
    // keptItems), so the note never ends in an ellipsis either.
    property var countItems: []
    // Whether its note gives way by whole words from its end, never with an
    // ellipsis, its name taking its letters first: a notice's line reads
    // "line 7 of the", never "line 7 of the de\u2026" (VKCF3Z, 2026-11-08).
    property bool countWords: false
    // The note as it fits beside the whole side, or by whole words, the
    // name taking its letters first.
    readonly property string noted: {
        if (!countWords && (!sideWhole || countItems.length === 0)) {
            return count;
        }
        void wordsFace.font;
        const width = Measures.measurer(wordsFace, true);
        const around = [row.also !== "" ? `\u201c${row.also}\u201d \u00b7 ` : "",
            row.kind !== "" && row.side !== "" ? ` \u00b7 ${row.side}` : ""]
            .reduce((sum, words) => sum + (words === "" ? 0 : width(words)), 0);
        const room = row.ends - Theme.space.rung.groups - nameBox.x - nameBox.implicitWidth
            - Theme.space.rung.apart - around;
        return countWords ? Reading.wordsFromEnd(count, room, width)
            : Reading.keptItems(countItems, room, width).join(" \u00b7 ");
    }
    // Whether its count is a run of names, as a home row's is, which gives
    // way by whole names rather than by items.
    property bool names: false
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
        row.noted, row.kind !== "" ? row.side : ""].filter(part => part !== "").join(" \u00b7 ")
    // The detail as it would be uncut, its note whole, and whether there's
    // one at all. The side's room and share reckon from these rather than
    // from the detail, since a note that gives way reads the side's place
    // (row.ends), and reading the detail back would loop.
    readonly property string detailUncut: [row.also !== "" ? `\u201c${row.also}\u201d` : "",
        row.sideWhole && row.countItems.length > 0
            ? row.countItems.map(item => item.said).join(" \u00b7 ") : row.count,
        row.kind !== "" ? row.side : ""].filter(part => part !== "").join(" \u00b7 ")
    readonly property bool detailed: detailUncut !== ""
    // Read through the face's font, as noted is, so a scale or a theme
    // changed measures it again.
    readonly property real detailWide: {
        void wordsFace.font;
        return detailed ? Measures.measurer(wordsFace, true)(detailUncut) : 0;
    }
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
    // Whether a figure wears its mark anyway, as a budget's window does,
    // whose mark's hue says how it stands (VKCF3Z, 2026-11-07).
    property bool marked: false
    // Whether its hue says how it stands, a warning's or an alert's, which
    // its mark keeps however quiet a row of its sort is.
    readonly property bool standing: Qt.colorEqual(hue, Theme.warning)
        || Qt.colorEqual(hue, Theme.alert)
    // Its graph, as Graph.qml draws one, or null.
    property var graph: null
    // Whether what it says warns, as an update waiting does: what it says
    // at its right takes the warning colour, and its graph all of it, one
    // flag for both so they never disagree (docs/design.md, Kinds).
    property bool warns: false
    // Whether what it says needs him now, as a session's folder gone: what
    // it says at its right takes the alert colour (VKCF3Z, 2026-11-07).
    property bool alerts: false
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
    readonly property real ends: graphed ? drawing.x : atRight.x

    signal clicked

    width: parent.width
    height: 4 * Theme.space.beat
    color: "transparent"
    // Selected, it's filled, unless the list draws the selection itself so
    // it can glide from row to row (Launcher.qml's list).
    property bool fills: true
    // How far left of the row its frame's inside edge stands, the gutter's
    // room between, which its ground reaches out to.
    property real groundFrom: 0

    // A TextMetrics, which follows the face as it loads, where a
    // FontMetrics' call in a binding is asked once, before it has.
    TextMetrics {
        id: capitalsFace
        text: "H"
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
    }

    // Its ground, its whole band (Ground.qml).
    Ground {
        from: row.groundFrom
        width: row.width + row.groundFrom
        height: row.height
        ink: row.flashed ? "flash" : row.selected && row.fills || row.tinted ? "selection" : ""
    }

    // What the row says at its right is measured as it loads at many times
    // its size and read back down, since a measure at the size drawn comes
    // in whole pixels.
    readonly property real fine: 8

    // Its glyph, on the name's capitals (Mark.qml), whose baseline is the
    // row's middle less half the capitals.
    Mark {
        x: Theme.space.rung.inset + row.indent
        line: (row.height + row.capitals) / 2
        // A swatch is the figure itself, so it stays, as does a figure's
        // mark saying how it stands.
        visible: row.sort !== "figure" || row.swatch || row.marked
        text: row.glyph
        // A row holding more is quieter than one that acts, muted as a
        // place's mark is in the head: never the frame's edge, which the
        // cursor's ground all but hides (VKCF3Z, 2026-11-06); but for a
        // warning or an alert, which it keeps.
        color: row.flashed && !row.swatch ? row.flashInk
            : row.sort === "holds" && !row.swatch && !row.standing ? Theme.muted : row.hue
    }

    // Selected, a name too long for its room reads on (ReadsOn.qml), and
    // so does what the row says at its right. With both cut they take
    // turns, the name first, so only one thing moves at a time and each
    // reads whole; given up by the selection, or no longer seen, both are
    // back at their start at once. They read on twice, then rest at their
    // start, cut, until the selection moves again (VKCF3Z, 2026-11-09:
    // motion that never ends ranks nothing and costs frames while Joel
    // reads; the start names the row, and the drawer holds it whole).
    readonly property bool reads: row.selected && row.shown
    readonly property int readings: 2
    // How many turns its cut words have read on through since it was
    // selected: a turn is the name's and then the side's, where both are
    // cut.
    property int readsDone: 0
    readonly property bool readingOn: row.reads && row.readsDone < row.readings
    // Where its name has its baseline, down from its top, which the
    // launcher's drawer sets its title on.
    readonly property real nameBaseline: nameBox.y + nameBox.baselineOffset
    // Where its words have theirs, which is the name's.
    readonly property real detailBaseline: countText.y + countText.baselineOffset
    // How far its words stand from its name.
    readonly property real detailGap: countText.x - nameBox.x - nameBox.width
    readonly property string detailFace: countText.font.family
    readonly property bool nameCut: nameBox.over > 0
    readonly property bool sideCut: atRight.sided && sideText.over > 0
    property bool sidesTurn: false
    onReadsChanged: row.letGo()
    // Forget what its words did: whose turn it was and how many times they
    // read on. A row the launcher lets go is given to another, which reads
    // from its own start (Launcher.qml's reuseItems).
    function letGo() {
        row.sidesTurn = false;
        row.readsDone = 0;
    }

    // The name's room, which it's cut to.
    ReadsOn {
        id: nameBox
        x: Theme.space.rung.inset + row.glyphColumn + row.indent
        width: Math.max(0, Math.min(implicitWidth, row.ends - x - Theme.space.rung.groups
            - (!row.detailed ? 0 : Math.min(row.detailWide, row.width * 0.3)
                + Theme.space.rung.apart)))
        // On the mono's line, so the voice sits where the mono's name
        // would, the faces' line boxes differing.
        anchors.baseline: monoLine.baseline
        level: row.voiced ? Theme.level.voice : Theme.level.name
        text: row.name
        plain: row.plainName
        // Cut only at a whole word (docs/design.md, Set, not styled).
        wholeWords: true
        // The flash takes the row's ground, so a find's wash, which would
        // lie on it in the selection's colour and vanish, gives way.
        finds: row.flashed ? [] : row.finds
        washedSelected: row.selected
        format: row.selected || row.quiet || row.flashed ? Text.PlainText : Text.StyledText
        color: row.flashed ? row.flashInk : row.quiet ? Theme.muted
            : row.selected ? Theme.strong : Theme.text
        figure: figureFace.advanceWidth
        reading: row.readingOn && !(row.sideCut && row.sidesTurn)
        onThrough: {
            if (row.sideCut)
                row.sidesTurn = true;
            else
                row.readsDone += 1;
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
        readonly property real room: Math.max(row.width * 0.3,
            row.ends - x - Theme.space.rung.groups)
        // A row naming what it holds (`names`, as a home network's rows
        // do) gives way by whole names from its end, ending on the last
        // that fits, never an ellipsis (VKCF3Z, 2026-11-08); a place's row
        // names nothing after its name (dae7fb67); any other row's words
        // give way by whole items, so a side such as "370 Mb/s down, 20 up"
        // is never cut at its comma.
        // The elision below is only a last guard. Its words are measured
        // as detailWide measures them, so the room kept for it and the fit
        // agree to the pixel.
        readonly property string whole: row.names
            ? Reading.wholeNamesFromEnd(row.detail, room, Measures.measurer(wordsFace, true))
            : Reading.wholeItemsFromEnd(row.detail, room, Measures.measurer(wordsFace, true))
        width: Math.min(implicitWidth, room)
        elide: Text.ElideRight
        anchors.baseline: monoLine.baseline
        visible: whole !== ""
        text: whole
        color: row.flashed ? row.flashInk : row.selected ? Theme.text : Theme.muted
        font.family: Theme.level.words.family
        font.pointSize: Theme.level.words.pointSize
        font.variableAxes: Theme.level.words.variableAxes
    }

    // The note after the detail, across a dot as the detail's items are,
    // or after the name where there's none, as the detail would stand. It
    // ends a groups' rung short of the say, or of a graph, or of the ink
    // said at the right, not of its column, which the list's widest side
    // sets; or on the inset where nothing's said there.
    readonly property real noteFrom: countText.visible ? countText.x + countText.width
        : nameBox.x + nameBox.width + Theme.space.rung.apart
    readonly property string noteApart: countText.visible ? " \u00b7 " : ""
    readonly property real noteEnd: sayText.visible ? sayText.x - Theme.space.rung.groups
        : graphed ? drawing.x - Theme.space.rung.groups
        : atRight.inked === 0 ? atRight.x + atRight.width
        : atRight.x + atRight.width - atRight.inked - Theme.space.rung.groups
    // None where the name, or what the row holds, gave way: those are cut
    // at the column, and the note gives way first, so it never takes the
    // room past it that they were refused (Y2YBTS, 2026-11-09).
    readonly property bool detailCut: row.detailed
        && (countText.whole !== row.detailUncut || countText.truncated)
    readonly property string noteWhole: {
        if (row.note === "" || row.nameCut || row.detailCut)
            return "";
        void wordsFace.font;
        const width = Measures.measurer(wordsFace, true);
        const room = row.noteEnd - row.noteFrom
            - (row.noteApart === "" ? 0 : Measures.measurer(wordsFace)(row.noteApart));
        return Reading.noteFitted(row.note, room, width);
    }

    Text {
        id: noteText
        x: row.noteFrom
        anchors.baseline: monoLine.baseline
        visible: row.noteWhole !== ""
        textFormat: Text.PlainText
        text: row.noteApart + row.noteWhole
        color: countText.color
        font: countText.font
    }

    // The note's face, to fit a note of items, or of names, to its room.
    FontMetrics {
        id: wordsFace
        font: countText.font
    }

    Graph {
        id: drawing
        x: atRight.x - width - (atRight.width > 0 ? Theme.space.rung.groups : 0)
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
        id: atRight
        anchors.right: parent.right
        anchors.rightMargin: Theme.space.rung.inset
        y: monoLine.y + monoLine.baselineOffset
            - (atRight.chorded ? keys.baselineOffset : sideText.baselineOffset)
        readonly property bool chorded: row.landed === "" && row.kind === ""
            && row.chord.length > 0
        // A landing while it shows, else a noun's kind, else how it stands;
        // never the key an ending act is carried by (Reading.rightSays).
        readonly property string stands: itemised ? kept.join(" \u00b7 ") : row.side
        readonly property string said: Reading.rightSays(row.landed, row.kind, stands)
        // The items a side of them keeps in its room, the least needed
        // given way first (Reading.js's keptItems).
        readonly property var kept: itemised ? Reading.keptItems(row.sideItems, room,
            Measures.measurer(sideFace, true)) : []
        // The colour each kept item is drawn in where it names a role of
        // its own (`hue`), as a count of what failed wears the alert's,
        // or "" for the side's.
        readonly property var inks: kept.map(words => {
            const item = row.sideItems.find(one => one.said === words);
            return item?.hue ? String(Theme[item.hue]) : "";
        })
        // Whether its dots are drawn apart from its words: where the words
        // aren't muted already, so its dots are, as the status line's are,
        // or where an item wears a colour of its own.
        readonly property bool dotted: itemised && kept.length > 1 && !row.flashed
            && (row.selected || row.warns || inks.some(ink => ink !== ""))
        readonly property bool sided: !chorded && said !== ""
        readonly property bool itemised: row.landed === "" && row.kind === ""
            && row.sideItems.length > 0
        // The room the name leaves whole, its words after it too, which a
        // side of items is fitted to.
        readonly property real room: row.width - Theme.space.rung.inset - nameBox.x
            - nameBox.implicitWidth - Theme.space.rung.groups - (!row.detailed ? 0
                : Math.min(row.detailWide, row.width * 0.3) + Theme.space.rung.apart)
        // A side of items down to its last that still leaves the name no
        // room whole: the name is cut, so where it's cut holds still as
        // that item changes, by the column, as any side's does.
        readonly property bool squeezed: itemised
            && Measures.measurer(sideFace, true)(said) > room
        // How a row stands never gives way: the name is cut first, at a
        // whole word, and "256 \u00d7 256 px" stays whole (docs/design.md,
        // A row says what it is; VKCF3Z, 6918348b, and of Inkscape's
        // copied picture, 2026-11-09). A window's title keeps its room
        // since its program and place are words after its name, which give
        // way first (Joel, 2026-10-25: titles "get elipsed really quick").
        // Only words too long for 45% of the row, a sentence, are cut, and
        // to what the name leaves, down to a fifth of the row, so however
        // long they run they can't reach the name. Past its own side a row
        // takes only that too, so a long side on one row doesn't cut every
        // name in the step through the column they share. It's
        // reckoned from where a name starts at no depth, so a row a step in
        // says as much at its right as the row above it. That start already
        // holds the glyph's column, so only the right's inset is taken here.
        readonly property real share: Math.min(row.width * 0.45, Math.max(row.width * 0.2,
            atRight.sided && sideText.implicitWidth <= row.width * 0.45
                ? Math.ceil(sideText.implicitWidth) : 0,
            row.width - nameBox.x + row.indent - nameBox.implicitWidth
            - Theme.space.rung.groups - Theme.space.rung.inset
            - (!row.detailed ? 0
                : Math.min(row.detailWide, row.width * 0.3) + Theme.space.rung.apart)))
        // As wide as the widest beside a graph in the list, so the graphs
        // line up, and never more than what's said may take; for a row
        // without one, as wide as the widest said here in this step, within
        // its share.
        readonly property real column: row.graphed
            ? Math.min(row.figures * noteFace.advanceWidth, row.width * 0.45)
            : atRight.sided && row.sideWhole ? row.sides * noteFace.advanceWidth
            : atRight.sided && (!atRight.itemised || atRight.squeezed)
                ? Math.min(row.sides * noteFace.advanceWidth, atRight.share) : 0

        // Whether a figure stands before the chord, as a collection's count
        // does, so a count is said at a row's right even where its chord
        // is (VKCF3Z, 2026-11-05: "12 \u00b7 \u2699 + 0").
        readonly property bool counted: chorded && said !== ""
        // The figure as the count draws it, or nothing where there's no
        // count: a row reused for another lays out and measures its words
        // anew at eight times their size, so a side said by sideText alone
        // is never laid out a second and third time unseen.
        readonly property string counts: counted ? said : ""
        // How wide what it says is drawn, from its right, which the say
        // stands before.
        readonly property real inked: chorded ? keys.width
                + (counted ? beforeKeys.implicitWidth + Theme.space.rung.mark : 0)
            : sided ? sideText.width : 0
        width: Math.max(column, inked)
        height: chorded ? keys.height : sideText.height

        Chord {
            id: keys
            anchors.right: parent.right
            visible: atRight.chorded
            // None where it isn't drawn, since each key is a Text made
            // afresh whenever the row is given other keys.
            keys: atRight.chorded ? row.chord : []
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
            text: atRight.counts
            font.family: Theme.level.note.family
            font.italic: Theme.level.note.italic
            font.weight: Theme.level.note.weight
            font.pointSize: Theme.level.note.pointSize * row.fine
        }
        // From the figure's width as drawn rather than its fine advance,
        // since a face hinted fully draws its advance in whole pixels.
        readonly property real countBearing: atRight.counts === "" ? 0
            : Math.max(0, countFigure.implicitWidth - (countInk.tightBoundingRect.x
                + countInk.tightBoundingRect.width) / row.fine)
        Row {
            id: beforeKeys
            anchors.right: keys.left
            anchors.rightMargin: Theme.space.rung.mark
            anchors.baseline: keys.baseline
            baselineOffset: countFigure.baselineOffset
            spacing: Theme.space.rung.mark - atRight.countBearing
            visible: atRight.counted

            Text {
                id: countFigure
                text: atRight.counts
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
            visible: atRight.sided
            width: atRight.itemised || row.sideWhole ? implicitWidth
                : Math.min(implicitWidth, atRight.share)
            text: atRight.dotted ? Reading.styledItems(atRight.kept, Theme.muted, atRight.inks)
                : atRight.said
            format: atRight.dotted ? Text.StyledText : Text.PlainText
            plain: atRight.said
            wholeWords: true
            endsRight: true
            level: Theme.level.note
            // Muted reads under the Lc 60 any text needs on the selection.
            color: row.flashed ? row.flashInk
                : row.landed !== "" ? Theme.accent
                : row.alerts && row.kind === "" ? Theme.alert
                : row.warns && row.kind === "" ? Theme.warning
                : row.selected ? Theme.text : Theme.muted
            figure: noteFace.advanceWidth
            reading: row.readingOn && atRight.sided && (!row.nameCut || row.sidesTurn)
            onThrough: {
                if (row.nameCut)
                    row.sidesTurn = false;
                row.readsDone += 1;
            }
        }
    }

    // Where the say ends: a mark, the dot and a mark short of the ink
    // said at the right, as the status line joins its facts, or the inset
    // where nothing's said there, or clear of a graph.
    readonly property real sayEnd: graphed ? drawing.x - Theme.space.rung.groups
        : atRight.inked === 0 ? atRight.x + atRight.width
        : atRight.x + atRight.width - atRight.inked - 2 * Theme.space.rung.mark - sayDot.width
    // Where it may start: past the name, and its words where there are any.
    readonly property real sayFrom: (row.detail !== "" ? countText.x + countText.width
        : nameBox.x + nameBox.width) + Theme.space.rung.groups
    // The say as much as fits its room: a note of items between dots given
    // way by whole items from its end, and not said at all where its first
    // doesn't fit whole (VKCF3Z, 2026-11-07: never "11.9 Mb/s\u2026";
    // 2026-11-08: never "found by\u2026"); a sentence given way at a whole
    // word, and not said where only its first word and an ellipsis would
    // fit (2026-11-07: never "A b\u2026 84% full"). Qt elides with a pixel
    // to spare, so each is measured with one.
    readonly property string sayWhole: {
        void sayFace.font;
        const cut = Reading.wholeItemsFromEnd(row.say, row.sayEnd - row.sayFrom,
            Measures.measurer(sayFace, true));
        return !cut.endsWith("\u2026") || cut.includes(" ") ? cut : "";
    }

    FontMetrics {
        id: sayFace
        font: sayText.font
    }

    Text {
        id: sayText
        width: Math.max(0, Math.min(implicitWidth, row.sayEnd - row.sayFrom))
        x: row.sayEnd - width
        anchors.baseline: monoLine.baseline
        visible: row.sayWhole !== "" && width > 0
        elide: Text.ElideRight
        textFormat: Text.PlainText
        text: row.sayWhole
        color: row.flashed ? row.flashInk : row.selected ? Theme.text : Theme.muted
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
        font.features: Theme.level.note.features
    }

    // The dot between the say and what's said at the right, measured to
    // its ink, as the figure before a chord's is.
    Item {
        id: sayDot
        x: row.sayEnd + Theme.space.rung.mark
        anchors.baseline: monoLine.baseline
        baselineOffset: sayDotText.baselineOffset
        width: dotInk.tightBoundingRect.width / row.fine
        height: sayDotText.height
        visible: sayText.visible && !row.graphed && atRight.inked > 0

        Text {
            id: sayDotText
            x: -dotInk.tightBoundingRect.x / row.fine
            text: "\u00b7"
            color: row.flashed ? row.flashInk : Theme.muted
            font: countFigure.font
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
