// The launcher's drawer: what the selected row has to show beyond its one
// line, out of the card's right side rather than below its rows (Joel,
// 2026-10-28: "having it come out of the bottom feels wrong, i think it
// should come out of the right side like a drawer, making sure it never
// passes the bound of the screen"). It's the card continued, a floating
// surface like it, as tall as the card and top aligned with it, so the
// card never changes size for what's selected.
//
// It's set as a plate, a small page about the selected thing (look 3's
// piece 3), ranked by its type (docs/design.md, Type): an eyebrow saying
// when or where at Label, the thing's name at Title, the one strong thing
// while it's shown, its facts as a label at Note and a value at Figure,
// its words, then its picture. Its head is one unit and the rest lies on
// the card's beats, so the two read as one page: the eyebrow on the
// field's baseline, the name a Title pitch under it, and the rest from the
// first of the card's beats two Title pitches under the name, on the
// drawer's line pitch, one empty line between each group. A row gives it as
// `drawer: { eyebrow, title, preview, facts: [{ label, value }], body,
// styled, picture, across, down }`, the body plain unless it says it's
// styled, a fact whose value isn't known yet null, shown as a fixed
// placeholder until it is. A preview, as a file's plate has (FileFacts.js),
// comes right under the name: `{ picture, across, down }` or `{ lines,
// quiet }`. What doesn't fit is reached by going into it (below).
//
// Home's Now gives a plate of the moment instead (look 3's piece 4,
// launcher/Now.qml): `{ now: true, eyebrow, seconds, say, groups:
// [[{ label, value, qualifier }]] }`, set by the rule every plate is
// (VKCF3Z, 2026-11-05): its head the eyebrow, the time large a Display
// pitch under it with its seconds counting after, and what needs Joel a
// Voice pitch under that; its groups of facts that hold any as a body
// under its head, each the drawer's groups apart.
//
// It's a window the keys go into, as his Neovim's splits are: Space and
// the right arrow from the rows, and back with Space and the left arrow
// (QueryKeys.js, the page mode). While it holds them its frame takes the
// accent and a cursor stands on one of its lines, each line of words it
// shows: the eyebrow, the title's, a preview's, each fact's, the words',
// and the moment's. A picture is no line. The cursor's line is marked by
// an accent rule in the left margin, as the reader marks its own and his
// Neovim lights only the cursor line's number (VKCF3Z's spec v2, The
// Reader): the cursor keeps a letter along it, which h and l move, for
// visual to start from. Its words are laid out by Qt as read-only
// TextEdits (PageText, below), which can say which letters each line
// holds and where, so the cursor stands on the letters drawn;
// DrawerPage.js says where each key takes it and the view.
pragma ComponentBehavior: Bound
import Quickshell
import QtQuick
import QtQuick.Effects
import "launcher/Moment.js" as Moment
import "DrawerFigure.js" as Figure
import "DrawerPage.js" as Page
import "Identifier.js" as Identifier
import "Pictures.js" as Pictures
import "Search.js" as Search
import "Reading.js" as Reading

Floating {
    id: drawer

    // Words set on the plate, laid out by Qt as a read-only TextEdit so the
    // keys can ask which letters each line holds and where, at a fixed
    // line height as a Text's FixedHeight sets it, which a TextEdit takes
    // from its rich text's style: it breaks at the same letters a Text
    // would. A line that elides is cut first, by Qt's elision, and what's
    // left laid out.
    component PageText: Text {
        id: words

        // What it says, plain unless `styled` says it's the row's own
        // markup, as a StyledText reads it.
        property string plain
        property bool styled: false
        // What the drawer's search seeks, each place it's found in plain
        // words washed as his Neovim's Search draws a match, set by each
        // use, since an inline component can't see the drawer; a styled
        // text keeps its own markup.
        property string sought
        // How tall each of its lines is, the plate's pitch or a step of it,
        // or 0 for the face's own, as a line set alone by its baseline is.
        required property real leading
        // Whether it's one line, cut with an ellipsis where it's longer
        // than its width; or one line as long as it is, with no width.
        property bool elides: false
        property bool wraps: true
        // Whether its layout is read, which the drawer says while it holds
        // the keys, and what reads it then: a TextEdit laid on its letters
        // in no ink, since a Text can't say which letters each of its lines
        // holds. It's made only then, as a TextEdit for every text drawn
        // would give each plate a QTextDocument a text as the rows are
        // moved through, twice a Text's time to make one.
        property bool reading: false
        // Whether its reader is made to count what it holds past a line,
        // as the drawer's words cut at its foot are, by paragraph.
        property bool counting: false
        readonly property TextEdit reader: readerLoader.item
        // Its letters' colour. While the reader is made it draws every
        // letter and the Text none, so each letter is drawn once: a chosen,
        // washed or flashed letter drawn over the Text's own would add
        // their antialiased edges together and look heavier than its
        // neighbours, where a selection changes only fill and colour.
        property color ink: Theme.text

        // Search's wash, strong ink on raised, as the held notices wash what
        // they find (Notifications.qml), drawn by the reader on the Text's
        // letters, which a search, only ever run in the drawer, has.
        readonly property var wash: ({ ink: `${Theme.strong}`, ground: `${Theme.raised}` })
        // Whether its share of what the drawer's visual chose is flashing,
        // which its reader draws (below). Set by the drawer's paint(), since
        // an inline component can't see it.
        property bool flashed: false
        // Whether what's chosen is whole lines, which the drawer grounds as
        // bands across the measure, so its reader grounds none of its own
        // under them; set by paint() as flashed is.
        property bool banded: false
        // What a copy as markdown puts before it: a title's #, a heading's ##.
        property string marked: ""

        // A styled text's own colours, which no ink hides, go while the
        // reader draws its letters; colours move no letter.
        text: reader && styled ? plain.replace(/ color="[^"]*"/g, "") : plain
        textFormat: styled ? Text.StyledText : Text.PlainText
        color: reader ? "transparent" : ink
        // A link is underlined in the words' own colour, never Qt's blue,
        // since colour goes on glyphs and never on text.
        linkColor: color
        // Broken only at spaces, as a page is: a drawer too narrow for a
        // word lets it run past the edge rather than split it (VKCF3Z).
        wrapMode: elides || !wraps ? Text.NoWrap : Text.WordWrap
        elide: elides ? Text.ElideRight : Text.ElideNone
        lineHeightMode: leading > 0 ? Text.FixedHeight : Text.ProportionalHeight
        lineHeight: leading > 0 ? leading : 1

        Loader {
            id: readerLoader
            active: words.reading || words.counting
            sourceComponent: TextEdit {
                // How far below its top Qt sets its first line's letters: a
                // fixed line keeps its spare room above its letters, which a
                // TextEdit's baselineOffset leaves out, so it's set by both.
                readonly property real drop: {
                    void contentHeight;
                    void width;
                    return positionToRectangle(0).y;
                }
                readonly property string said: words.elides
                    ? cut.elidedText(words.plain, Qt.ElideRight, words.width) : words.plain
                // How tall its capitals stand over its baseline, which a
                // band and the cursor's rule are set by: a TextMetrics's
                // bounds follow the face as it loads, where a FontMetrics
                // asked in a binding answers once, before it has (HVD6O3,
                // 2026-11-05: about 15% small in the shots).
                readonly property real caps: capitals.tightBoundingRect.height

                // Its letters where the Text's were, in their ink and the
                // row's markup's colours, with what a search washes on its
                // own ground.
                y: words.baselineOffset - drop - baselineOffset
                width: words.width
                readOnly: true
                selectByMouse: false
                activeFocusOnPress: false
                color: words.ink
                // Its share of what visual chose, drawn as Qt draws a
                // selection: on the selection's ground in its own ink as his
                // Neovim's Visual is, or the accent's under the accent's ink
                // while y flashes it, as his on_yank does. Only fill and
                // colour change, never weight.
                persistentSelection: true
                selectionColor: words.banded ? "transparent"
                    : words.flashed ? Theme.accent
                    : Qt.alpha(Theme.selection, Theme.floating.selection)
                selectedTextColor: words.flashed ? Theme.on_accent : words.ink
                font: words.font
                // Set as its Text is, so a right-to-left script's letters
                // are drawn where they were laid, not at the far edge.
                horizontalAlignment: words.horizontalAlignment
                textFormat: TextEdit.RichText
                wrapMode: words.wrapMode === Text.WordWrap ? TextEdit.WordWrap : TextEdit.NoWrap
                text: `<div style="white-space:pre-wrap`
                    + (words.leading > 0
                        ? `; line-height:${words.leading}px; -qt-line-height-type:fixed` : "")
                    // A match in a styled text lands the cursor but isn't
                    // washed, since washing inside markup isn't done. Its
                    // links take its ink, as the Text's do, never rich
                    // text's blue.
                    + `">${words.styled ? said.replace(/<a /g, `<a style="color:${words.ink}" `)
                        : words.sought !== "" ? Search.washed(said, words.sought, words.wash)
                            : Search.escaped(said)}</div>`

                FontMetrics {
                    id: cut
                    font: words.font
                }
                TextMetrics {
                    id: capitals
                    font: words.font
                    text: "H"
                }
            }
        }
    }

    // What it's given to show, or null once it's to go in, when it keeps
    // what it last showed while it slides away.
    property var given: null
    property var shown: null
    onGivenChanged: {
        if (given)
            shown = given;
    }
    // Whether it's out, which it fades only for, never for what it shows.
    property bool out: false
    // Out again, it shows what it was last given, which a row coming back
    // hands it unchanged: what it showed was let go as it faded, and the
    // same drawer given again changes nothing to show it by.
    onOutChanged: {
        if (out && given)
            shown = given;
    }
    // Whether it's kept from showing only until the keys go in, where it
    // takes the card's place: what it shows is kept, so it has lines for
    // the keys to go into.
    property bool waits: false
    onWaitsChanged: {
        if (waits && given)
            shown = given;
    }
    // How tall it is, the card's height.
    property real tall: 0
    // The card's lines it's set on, down from the card's top as from its
    // own, since the two share a top: the field's baseline, which the
    // eyebrow shares, and the middle of the first row, whose baseline the
    // plate's beats are counted from.
    property real fieldBase: 0
    property real rowLine: 0
    // How far below a row's line its name's baseline stands, as the card
    // sets its rows (their capitals centred on the line, Choice.qml).
    property real rowDrop: 0

    readonly property string eyebrow: shown ? shown.eyebrow ?? "" : ""
    // The level its eyebrow is set at: Label, in capitals, for the
    // shell's own words, and Note, as written, for one that carries a name
    // from the world, an address, a repo or a folder, which is never
    // re-cased, as a title never re-cases one (VKCF3Z, 2026-11-06). A row
    // says so with `eyebrowNames: true`. Both are one level, so the
    // drawer's count of them stays.
    readonly property var eyebrowLevel: shown?.eyebrowNames ? Theme.level.note
        : Theme.level.label
    // The eyebrow as it's drawn, given way by whole items and never cut
    // inside a word (Reading.eyebrowOf), so it's one line with no ellipsis
    // of Qt's.
    readonly property string eyebrowSaid: Reading.eyebrowOf(shown, eyebrowText.width,
        words => Math.ceil(eyebrowFace.advanceWidth(words)) + 1)
    FontMetrics {
        id: eyebrowFace
        font.family: drawer.eyebrowLevel.family
        font.pointSize: drawer.eyebrowLevel.pointSize
        font.weight: drawer.eyebrowLevel.weight
        font.italic: drawer.eyebrowLevel.italic
        font.variableAxes: drawer.eyebrowLevel.variableAxes
        font.capitalization: drawer.eyebrowLevel.capitalization
        font.letterSpacing: drawer.eyebrowLevel.letterSpacing
        font.features: drawer.eyebrowLevel.features
    }
    readonly property string title: shown ? shown.title ?? "" : ""
    // Its facts, less any that only says the title again, as a server's
    // name would right under it (VKCF3Z, 2026-11-03).
    readonly property var facts: shown ? (shown.facts ?? [])
        .filter(fact => fact && String(fact.value ?? "") !== title) : []
    // A fact's row is kept once made, each reading the fact at its place
    // and hidden past the facts shown, so only a thing with more facts
    // than any before makes rows: they were made afresh for every fact
    // each time the facts changed, and the first program's three took
    // 5 ms of a 19 ms step into Programs at rest, made again on every
    // step since home's drawer holds none (nested niri, 2026-11-10). So
    // the rows kept grow to the most facts any drawer has shown, a few
    // rows, and never shrink.
    onFactsChanged: drawer.keptTo(factsKept, facts.length)
    // How many fact rows it has made, for the check that moving between
    // things and back into a place makes none.
    property int factsMade: 0
    function keptTo(kept, count) {
        while (kept.count < count)
            kept.append({});
    }
    // What a hidden row reads, the same each time, so its bindings stay.
    readonly property var noFact: ({})
    readonly property string body: shown ? shown.body ?? "" : ""
    // Whether it lists keys (`keys: true`), as Space ? does: each chord is
    // the name and what it does the value, both at Note, the chord in the
    // text's ink and its words muted, as the reader's key list sets them, so
    // a line fits on one. At Figure, in the mono, most wrapped (VKCF3Z,
    // 2026-11-04).
    readonly property bool listsKeys: shown ? shown.keys === true : false
    // The script its title and values are written in (`script: "hebrew"`),
    // as a calendar's date in its own is, set in the script face's cuts
    // beside the serif and the mono (Theme.script), since Qt falls back by
    // script, not language, and so draws Chinese in its own forms only where
    // its cut is named. Its names stay in the theme's own.
    readonly property string script: shown ? shown.script ?? "" : ""
    readonly property string picture: shown ? shown.picture ?? "" : ""
    readonly property var preview: shown ? shown.preview ?? null : null
    // Whether it's the moment's plate, home's Now's.
    readonly property bool moment: shown?.now === true
    // Whether its words are prose to read, set as the reader sets its text,
    // rather than lines in the mono.
    readonly property bool prose: shown?.prose === true
    // Whether its words are a run of items a dot apart (`runs: true`), as a
    // place's members are, laid as a fact's run is (itemsKept): where it
    // wraps, the break falls between two and the dot goes with it, so no
    // line ends or starts on one (VKCF3Z, 2026-11-09, 8e0712aa).
    readonly property bool runs: shown?.runs === true
    // Its words as they're given to be drawn, a run measured in their own
    // face, for the launcher to report.
    readonly property string bodySaid: runs
        ? itemsKept(body, bodyFace, wordsBox.width) : body
    // How many lines Qt draws its words in, for the launcher to report: a
    // run laid as one is as many as bodySaid's, Qt wrapping none further.
    readonly property int bodyLines: bodyText.lineCount
    // Its groups of facts that hold any: a group with nothing to show
    // gives up its place, so a day with no weather or meeting leaves no
    // empty lines in the middle of the plate (docs/design.md, Space
    // Available).
    readonly property var groups: moment ? (shown.groups ?? [])
        .filter(facts => facts.length > 0).map(facts => ({ facts: facts })) : []
    // The time and its seconds as it says them, for the launcher to report.
    readonly property string momentTime: moment ? timeText.text : ""
    readonly property string momentSeconds: moment ? secondsText.text : ""
    // Whether it holds more than fits, so the foot says how to scroll it.
    readonly property bool more: view.natural > drawer.viewTop + drawer.viewRoom + 1
    // How far it's scrolled from its rest, which its keys set and the
    // launcher reports.
    property real scrolled: 0
    // The content's top the view shows from, as its keys set it. What
    // reads it takes it from here rather than the view's contentY: a
    // Flickable whose height changes rounds a contentY off a whole pixel
    // and sets it again, which since the field's baseline stands on a half
    // pixel came back through what sets that height (shownAndHidden) as a
    // binding loop at each change of scale.
    readonly property real shownFrom: viewTop + scrolled
    function scrollTo(top) {
        scrolled = top - viewTop;
    }

    // The drawer's line pitch: Figure's, two beats, for every line of the
    // plate, Note's runs too, so the plate keeps one grid and its lines
    // fall on the card's beats (VKCF3Z, 2026-11-05).
    readonly property real pitch: Theme.level.figure.pitch
    // How far below a line's centre a row's name has its baseline, as a row
    // centred on that line does. Every line here is set on such a
    // baseline, whatever its own face, since the faces' line boxes differ.
    readonly property real dropped: rowDrop
    // How far apart a Title's lines sit: its own pitch, tighter than the
    // plate's two, so a name that wraps reads as one heading rather than
    // two (VKCF3Z, 2026-11-04).
    readonly property real titleLeading: Theme.level.title.pitch
    // The plate's head as one unit (docs/design.md, The Ladder): the eyebrow
    // on the field's baseline, as everything on that line is, and the title
    // a Title pitch under it, the tightest gap on the plate. What follows
    // starts on the first of the card's beats at least two Title pitches
    // under the title's last line, the widest, so the head reads apart
    // from the body. The beats are counted from the first row's baseline,
    // so every line after the head lies a whole number of them from every
    // row of the card, home's gap or not.
    readonly property real rowBaseline: rowLine + dropped
    readonly property real titleBase: fieldBase + Theme.level.title.pitch
    readonly property real titleLastBase: titleBase
        + (titleText.lineCount - 1) * titleLeading
    readonly property real bodyBase: onBeat(titleLastBase + 2 * Theme.level.title.pitch)
    // The first of the card's beats at or below a line, down from the top.
    // A line already on a beat, to within the sums' rounding, keeps it
    // rather than going a beat on: the rounding is a share of a beat, not
    // a space.
    readonly property real rounding: 0.001
    function onBeat(line) {
        return rowBaseline + Math.ceil((line - rowBaseline) / Theme.space.beat - rounding)
            * Theme.space.beat;
    }

    // Where its lines' baselines are, down from its top, for the launcher
    // to report: the eyebrow's and the middle of its line, the title's first
    // and last lines',
    // each fact's first line's, the words' first line's, and the top of a
    // preview, its end and its lines' baselines. Worked out
    // only when asked, since nothing drawn depends on them.
    function baselines() {
        const contentTop = Theme.border + drawer.viewTop - view.contentY;
        const shown = item => item.visible ? contentTop + item.y + item.baselineOffset : null;
        const flowed = item => item.visible ? contentTop + flow.y + item.y + item.baselineOffset
            : null;
        return {
            eyebrow: shown(eyebrowText),
            eyebrowMiddle: eyebrowText.visible ? contentTop + eyebrowText.y
                + eyebrowText.height / 2 : null,
            words: wordsBox.visible ? contentTop + flow.y + wordsBox.y + bodyText.y
                + bodyText.baselineOffset : null,
            title: flowed(titleText),
            // Its last line's, a Title's leading below its first.
            titleLast: titleText.visible ? flowed(titleText)
                + (titleText.lineCount - 1) * drawer.titleLeading : null,
            preview: previewBox.visible ? contentTop + flow.y + previewBox.y : null,
            previewEnd: previewBox.visible ? contentTop + flow.y + previewBox.y
                + previewBox.height : null,
            previewLines: previewLines.visible ? Array.from({ length: previewRows.count },
                (_, at) => {
                    const line = previewRows.itemAt(at);
                    return line ? contentTop + flow.y + previewBox.y + line.y
                        + line.baselineOffset : null;
                }) : [],
            facts: factsShown.visible ? Array.from({ length: factRows.shown }, (_, at) => {
                const row = factRows.itemAt(at);
                return row ? contentTop + flow.y + factsShown.y + row.y + row.base : null;
            }) : [],
            // The moment's: the time's and its seconds', and each group's
            // facts'.
            time: drawer.moment ? shown(timeText) : null,
            timeMiddle: drawer.moment ? shown(timeText)
                - timeFace.tightBoundingRect("H").height / 2 : null,
            seconds: drawer.moment ? shown(secondsText) : null,
            groups: drawer.moment ? Array.from({ length: groupsShown.count }, (_, at) => {
                const group = groupsShown.itemAt(at);
                return group ? group.bases.map(base => contentTop + group.y + base) : [];
            }) : []
        };
    }

    // A figure, then what qualifies it, quieter after a dot, each number
    // kept to its unit.
    function qualified(fact) {
        const plain = words => unbroken(words ?? "").replace(/&/g, "&amp;")
            .replace(/</g, "&lt;");
        return (fact.qualifier ?? "") === "" ? plain(fact.value)
            : `${plain(fact.value)} <font color="${Theme.muted}">\u00b7 `
                + `${plain(fact.qualifier)}</font>`;
    }


    // Whether it has words for a cursor to sit on, so the keys can go in.
    readonly property bool lined: shown !== null && (moment || eyebrow !== "" || title !== ""
        || facts.length > 0 || body !== "" || (preview?.lines ?? []).length > 0)
    // The lines kept about the cursor as the view follows it, as his
    // Neovim's scrolloff (modules/neovim/init.lua).
    readonly property int scrolloff: 4
    // Its lines as Qt laid them out, worked out as the keys come in and
    // again once what it shows is laid out anew, never for each key; and
    // the cursor: its line, its letter along it, and how far along the
    // line it wants to be, which j and k go back to.
    property var laid: []
    // The tops its view may scroll to, none cutting a line, worked out once
    // its lines are laid or its end moves rather than for each key, which
    // on a long page would spend half a key's budget (DrawerPage.js).
    readonly property var wholeTops: Page.tops({ lines: laid, pitch: drawer.pitch,
        start: drawer.viewTop, phase: drawer.stops, end: drawer.viewEnd })
    property var cursor: ({ line: 0, letter: 0, want: 0 })
    // The search being typed in it, as the launcher's line holds it, and
    // the last one run, { pattern, forward }, which n and N go on with, as
    // Vim keeps its last search; and whether the last run went round an
    // end or found nothing, for the launcher to report. What's washed is
    // what's being typed, as his incsearch shows it, else the last search
    // while the keys are in it, as his hlsearch keeps it.
    property string seeking: ""
    property bool seekingForward: true
    property var lastSought: null
    property string landing: ""
    readonly property string washing: seeking !== "" ? seeking
        : keys && lastSought ? lastSought.pattern : ""
    onKeysChanged: {
        if (keys) {
            enter();
        } else {
            for (const text of texts())
                text.reading = false;
            // What the last search found goes with it, so a drawer gone
            // back into never says it went round before anything's sought.
            lastSought = null;
            landing = "";
        }
    }
    // Visual, as on a page: what it chooses by, "letters" or "lines", or
    // "" while it chooses nothing, which the launcher sets from its mode;
    // where it began, the cursor being its other end; and the last chosen,
    // { began, at, by }, for gv, let go as the keys go in again. What's
    // taken is each line's letters (DrawerPage.chosen), or while y flashes
    // what it copied, those it took, for the key timing's yank-flash.
    property string choosing: ""
    property string choseBy: ""
    property var began: null
    property var lastChosen: null
    property int flashFor: 200
    property var flashing: []
    property string flashedBy: ""
    property string copied: ""
    readonly property var taken: choosing !== "" && began !== null && began.line < laid.length
        && cursor.line < laid.length ? Page.chosen(began, cursor, choosing, laid) : flashing
    onChoosingChanged: {
        if (choosing !== "" && began === null)
            began = { line: cursor.line, letter: cursor.letter };
        else if (choosing === "" && began !== null) {
            lastChosen = { began, at: { line: cursor.line, letter: cursor.letter }, by: choseBy };
            began = null;
        }
        if (choosing !== "")
            choseBy = choosing;
        // y's flash is drawn once it's let go, though what it took stays.
        paint();
    }
    onTakenChanged: paint()
    // Whole lines chosen, or flashing once copied, are grounded as V's are
    // in his Neovim: a band for each, one width, the measure and a mark
    // past it at each side, so the gap between a key and its words, or
    // between two texts, is grounded too, where v grounds only the letters
    // it takes. The first stands a mark over its capitals, the last a mark
    // under its baseline, and each meets the next where the next's begins
    // (VKCF3Z, 2026-11-05). Each is { top, height } in the content.
    // How wide its letters' column is, which a band spans.
    readonly property real measure: column.width
    // Each band as drawn, { x, width, top, height } in the column, rounded
    // so one ending where the next begins says so, for the launcher to
    // report.
    function bandsDrawn() {
        const drawn = [];
        for (let at = 0; at < bandsShown.count; at++) {
            const band = bandsShown.itemAt(at);
            if (!band)
                continue;
            const top = Math.round(band.y);
            drawn.push({ x: Math.round(band.x), width: Math.round(band.width), top,
                height: Math.round(band.y + band.height) - top });
        }
        return drawn;
    }
    readonly property bool banded: choosing === "lines"
        || (choosing === "" && flashing.length > 0 && flashedBy === "lines")
    readonly property var bands: {
        if (!banded)
            return [];
        const lines = taken.filter(one => one.line < laid.length).map(one => laid[one.line]);
        const mark = Theme.space.rung.mark;
        const top = line => line.base - line.caps - mark;
        return lines.map((line, at) => ({
            top: top(line),
            height: (at + 1 < lines.length ? top(lines[at + 1]) : line.base + mark) - top(line)
        }));
    }
    Timer {
        id: flash
        interval: drawer.flashFor
        onTriggered: drawer.flashing = []
    }
    onShownChanged: relay()
    onWidthChanged: relay()
    Connections {
        target: view
        function onContentHeightChanged() {
            drawer.relay();
        }
    }

    // Every PageText it has, shown or not.
    function texts() {
        return [eyebrowText, timeText, secondsText, titleText]
            .concat(groupsShown.texts(), previewRows.texts(), factRows.texts(), [bodyText])
            .filter(text => text);
    }

    // Every line of words it shows, top to bottom down its content: each
    // { top, height, length, parts }, a part being the letters `from` on
    // of one PageText (`text`) that lie on the line, as its reader holds
    // them (`words`), where it starts across, and how many there are. Two
    // texts sharing a baseline, as a fact's name and its value, are one
    // line, read left to right. Each text shown is read from here on, its
    // reader made at once, as a Loader makes it.
    function lines() {
        const pieces = [];
        for (const text of texts()) {
            if (!text.visible)
                continue;
            text.reading = true;
            const words = text.reader;
            if (!words || words.length === 0)
                continue;
            for (let at = 0; at < words.lineCount; at++) {
                const step = text.leading > 0 ? text.leading
                    : words.contentHeight / words.lineCount;
                const y = words.drop + at * step + 1;
                const from = words.positionAt(0, y);
                const next = at + 1 < words.lineCount ? words.positionAt(0, y + step)
                    : words.length;
                const box = words.positionToRectangle(from);
                const place = words.mapToItem(column, box.x, box.y);
                pieces.push({ text, words, from, y,
                    length: words.getText(from, next).replace(/\s+$/, "").length,
                    x: place.x, top: place.y, height: box.height,
                    base: place.y + words.baselineOffset, caps: words.caps });
            }
        }
        pieces.sort((one, other) => one.base - other.base || one.x - other.x);
        const found = [];
        for (const piece of pieces) {
            const line = found[found.length - 1];
            const together = line && Math.abs(line.base - piece.base) < 2;
            // A blank piece on another's baseline is no part of its line,
            // which would take a letter the eye can't see: a blank line
            // is a line only where it stands alone.
            if (together && piece.length === 0)
                continue;
            if (together && line.length === 0) {
                found[found.length - 1] = { base: piece.base, top: piece.top,
                    height: piece.height, caps: piece.caps, length: piece.length,
                    parts: [piece] };
            } else if (together) {
                const bottom = Math.max(line.top + line.height, piece.top + piece.height);
                line.top = Math.min(line.top, piece.top);
                line.height = bottom - line.top;
                line.caps = Math.max(line.caps, piece.caps);
                line.length += piece.length;
                line.parts.push(piece);
            } else {
                found.push({ base: piece.base, top: piece.top, height: piece.height,
                    caps: piece.caps, length: piece.length, parts: [piece] });
            }
        }
        return found;
    }

    // The part of a line a letter along it lies in, and the letter's place
    // in that part's text.
    function partOf(line, letter) {
        let left = letter;
        for (const part of line.parts) {
            if (left < part.length || part === line.parts[line.parts.length - 1])
                return { part, at: part.from + Math.max(0, Math.min(left, part.length - 1)) };
            left -= part.length;
        }
        return null;
    }
    // How far across a letter is, and the letter nearest a place across.
    function xOf(line, letter) {
        const found = partOf(line, letter);
        if (!found)
            return 0;
        const box = found.part.words.positionToRectangle(found.at);
        return found.part.words.mapToItem(column, box.x, 0).x;
    }
    function letterAt(line, x) {
        let before = 0;
        for (const part of line.parts) {
            const last = part === line.parts[line.parts.length - 1];
            const ends = last ? Infinity : line.parts[line.parts.indexOf(part) + 1].x;
            if (x < ends) {
                const across = part.words.mapFromItem(column, x, 0).x;
                const at = part.words.positionAt(across, part.y) - part.from;
                return before + Math.max(0, Math.min(at, part.length - 1));
            }
            before += part.length;
        }
        return Math.max(0, line.length - 1);
    }

    // Into it: the cursor on the title's first letter where it's in view,
    // else the first line in view, so nothing moves as the keys come in.
    function enter() {
        lastChosen = null;
        began = choosing !== "" ? began : null;
        laid = lines();
        const titled = laid.findIndex(line => line.parts.some(part => part.text === titleText));
        const top = drawer.shownFrom;
        const held = laid.map((line, at) => at).filter(at => laid[at].top >= top
            && laid[at].top + laid[at].height <= top + drawer.viewRoom);
        const line = held.includes(titled) ? titled : held.length > 0 ? held[0] : 0;
        cursor = { line, letter: 0, want: laid.length > 0 ? xOf(laid[line], 0) : 0 };
    }
    // Another page shown while it holds the keys, as its list of keys is:
    // read from its top once Qt has laid it out, as a window opened on it.
    function anew() {
        if (keys)
            Qt.callLater(() => {
                scrolled = 0;
                enter();
            });
    }
    // Laid out anew while it holds the keys, as a fact's value comes or the
    // row refreshes: the lines read again once Qt has laid them out, the
    // cursor kept on its line and letter where they still are. A page not
    // laid out yet gives no lines, as the drawer comes in over the card,
    // and that says nothing of where the cursor was: clamped into it, the
    // cursor would fall from the title to the eyebrow and stay.
    function relay() {
        if (keys)
            Qt.callLater(() => {
                laid = lines();
                if (laid.length === 0)
                    return;
                const line = Math.max(0, Math.min(cursor.line, laid.length - 1));
                const most = laid.length > 0 ? Math.max(0, laid[line].length - 1) : 0;
                cursor = { line, letter: Math.min(cursor.letter, most), want: cursor.want };
            });
    }

    // A key's act while it holds the keys: where DrawerPage.js says it
    // takes the cursor and the view.
    function go(act, count) {
        const page = laid;
        const moved = Page.moved(act, count, {
            lines: page,
            line: cursor.line,
            letter: cursor.letter,
            want: cursor.want,
            top: drawer.shownFrom,
            height: drawer.viewRoom,
            start: drawer.viewTop,
            end: drawer.viewEnd,
            pitch: drawer.pitch,
            off: drawer.scrolloff,
            tops: wholeTops,
            xOf: (line, letter) => xOf(page[line], letter),
            letterAt: (line, x) => letterAt(page[line], x)
        });
        cursor = { line: moved.line, letter: moved.letter, want: moved.want };
        scrollTo(moved.top);
    }
    // A search run in it, from the cursor, the way it goes and a count on,
    // nothing sought seeking the last again, as Vim's / Return does; and n
    // and N going on with the last, its way and the other. The cursor goes
    // to the match's first letter and the view follows it; `landing` says
    // "wrapped" where it went round an end, as Vim says it hit the bottom,
    // and "missing" where it's found nowhere, the cursor staying. Each
    // gives how it went for the launcher to say, as the rows' n and N do
    // (Finding.matchTo): `none` Vim's E35 with nothing sought before, or
    // E486 where it's found nowhere, else "", and `wrapped` the end it went
    // round, with what was sought.
    function seek(pattern, forward, count) {
        const sought = pattern !== "" ? { pattern, forward }
            : lastSought ? { pattern: lastSought.pattern, forward } : null;
        if (!sought)
            return { none: "E35", wrapped: "", pattern };
        lastSought = sought;
        return land(sought.pattern, forward, count);
    }
    function seekOn(act, count) {
        if (!lastSought)
            return { none: "E35", wrapped: "", pattern: "" };
        return land(lastSought.pattern, act === "seekNext" ? lastSought.forward
            : !lastSought.forward, count);
    }
    function land(pattern, forward, count) {
        const landed = Page.sought(paged(), pattern, forward, count);
        landing = landed === null ? "missing" : landed.wrapped !== "" ? "wrapped" : "";
        if (landed === null)
            return { none: "E486", wrapped: "", pattern };
        cursor = { line: landed.line, letter: landed.letter, want: landed.want };
        scrollTo(landed.top);
        return { none: "", wrapped: landed.wrapped, pattern };
    }

    // Its lines as a search reads them, each with its letters as shown.
    readonly property var lettered: laid.map(line => Object.assign({}, line, {
        text: line.parts.map(part => part.words.getText(part.from, part.from + part.length))
            .join("") }))
    // The page as DrawerPage moves on it, from the cursor and the view.
    function paged() {
        return {
            lines: lettered,
            line: cursor.line,
            letter: cursor.letter,
            want: cursor.want,
            top: drawer.shownFrom,
            height: drawer.viewRoom,
            start: drawer.viewTop,
            end: drawer.viewEnd,
            pitch: drawer.pitch,
            off: drawer.scrolloff,
            tops: wholeTops,
            xOf: (line, letter) => xOf(lettered[line], letter),
            letterAt: (line, x) => letterAt(lettered[line], x)
        };
    }
    // Which match of how many, { at, of }, for the foot to count as his
    // Neovim's search count does: while one's typed the one it would land
    // on, and after the one the cursor stands on, or null off every match
    // (VKCF3Z, 2026-11-05: "/down \u00b7 1 of 3").
    readonly property var found: {
        if (!keys || washing === "" || laid.length === 0)
            return null;
        if (seeking === "")
            return Page.found(lettered, washing, cursor.line, cursor.letter);
        const landed = Page.sought(paged(), seeking, seekingForward, 1);
        return landed === null ? null : { at: landed.at, of: landed.of };
    }

    // Whether the view's top cuts one of its lines partway, for the
    // launcher to report; only lines laid while it holds the keys count.
    function topCut() {
        return keys && Page.cut({ lines: laid, pitch: drawer.pitch }, drawer.shownFrom);
    }

    // The cursor put at a place, wanting its letter's place across, and the
    // view following it as a move's does.
    function standOn(at) {
        const line = Math.max(0, Math.min(at.line, laid.length - 1));
        const letter = Math.max(0, Math.min(at.letter, laid[line].length - 1));
        cursor = { line, letter, want: xOf(laid[line], letter) };
        scrollTo(Page.shown(laid[line], drawer.shownFrom, {
            lines: laid,
            height: drawer.viewRoom,
            start: drawer.viewTop,
            end: drawer.viewEnd,
            pitch: drawer.pitch,
            off: drawer.scrolloff,
            tops: wholeTops
        }));
    }
    // o: the cursor to where visual began, which becomes where it is, as
    // Vim's o goes to the selection's other end.
    function otherEnd() {
        if (began === null || laid.length === 0)
            return;
        const was = { line: cursor.line, letter: cursor.letter };
        standOn(began);
        began = was;
    }
    // gv: the last chosen again, from where it began to where it ended,
    // the launcher choosing by its kind once this has stood the cursor.
    function reselect() {
        if (lastChosen === null || laid.length === 0)
            return;
        began = lastChosen.began;
        standOn(lastChosen.at);
    }
    // y: what's chosen as markdown, each text's letters in turn, a title
    // taken from its start after its # and a heading after its ##, those
    // sharing a line a space apart and the rest a line each; flashed, and
    // given back for the launcher to put on the clipboard. The cursor goes
    // to its start, as Vim's y leaves it, once gv has what it chose.
    function copy(lines = false) {
        const took = lines && began !== null ? Page.chosen(began, cursor, "lines", laid) : taken;
        const spans = spansOf(took);
        copied = spans.map((span, at) => (at === 0 ? ""
            : spans[at - 1].last === span.line ? " " : "\n")
            + (span.start === 0 ? span.text.marked : "")
            + span.words.getText(span.start, span.end).replace(/[\u2028\u2029]/g, "\n")
                .replace(/\u200b/g, "")).join("");
        flashedBy = lines || choosing === "lines" ? "lines" : "letters";
        flashing = took;
        flash.restart();
        if (began !== null) {
            const at = { line: cursor.line, letter: cursor.letter };
            lastChosen = { began, at, by: choosing };
            const first = took.length > 0 ? { line: took[0].line, letter: took[0].from } : at;
            began = null;
            standOn(first);
        }
        return copied;
    }
    // Each text's share of what's taken, in order down the drawer: where it
    // starts and ends in that text, and the first and last lines it's on.
    function spansOf(chosen) {
        const spans = [];
        for (const one of chosen) {
            let left = 0;
            for (const part of laid[one.line]?.parts ?? []) {
                const from = Math.max(one.from, left);
                const to = Math.min(one.to, left + part.length);
                left += part.length;
                if (to <= from)
                    continue;
                const start = part.from + from - (left - part.length);
                const end = start + to - from;
                const span = spans.find(found => found.words === part.words);
                if (span) {
                    span.start = Math.min(span.start, start);
                    span.end = Math.max(span.end, end);
                    span.last = one.line;
                } else {
                    spans.push({ text: part.text, words: part.words, start, end,
                        line: one.line, last: one.line });
                }
            }
        }
        return spans;
    }
    // What's taken drawn on its texts, each selecting its share, and every
    // other text selecting nothing.
    function paint() {
        const spans = spansOf(taken);
        const flashed = choosing === "" && flashing.length > 0;
        for (const line of laid)
            for (const part of line.parts) {
                const span = spans.find(found => found.words === part.words);
                part.text.flashed = flashed;
                part.text.banded = banded;
                if (span)
                    part.words.select(span.start, span.end);
                else
                    part.words.deselect();
            }
    }
    // How many visual has chosen, for the mode word to count: lines where
    // it takes more than one or takes them whole, else letters, as Vim's
    // showcmd counts a selection.
    readonly property int chosenCount: choosing === "" || taken.length === 0 ? 0
        : choosing === "lines" || taken.length > 1 ? taken.length
        : taken[0].to - taken[0].from

    // Where the cursor stands, in the content's terms: the letter as Qt
    // laid it, as wide as it is, or a space's width on an empty line or
    // past a line's end, with its baseline and face. The rule is drawn
    // beside its line, and visual's block on the letter.
    readonly property var block: {
        if (!keys || laid.length === 0)
            return null;
        const line = laid[Math.min(cursor.line, laid.length - 1)];
        const found = partOf(line, cursor.letter);
        if (!found)
            return null;
        const words = found.part.words;
        const box = words.positionToRectangle(found.at);
        const after = words.positionToRectangle(found.at + 1);
        const place = words.mapToItem(column, box.x, box.y);
        const letter = found.part.length === 0 ? "" : words.getText(found.at, found.at + 1);
        return {
            x: place.x,
            y: place.y,
            width: after.y === box.y && after.x > box.x ? after.x - box.x : box.height / 2,
            height: box.height,
            letter,
            base: found.part.base,
            // Where the rule stands: the drawer's pitch tall on whatever
            // level the line is set, centred on its capitals (roles.md).
            ruleTop: line.base - line.caps / 2 - drawer.pitch / 2,
            font: words.font,
            line: cursor.line,
            said: line.parts.map(part => part.words.getText(part.from, part.from
                + part.length)).join(" ")
        };
    }

    // Where the cursor line's rule is drawn, from the drawer's top left, as
    // the launcher reports it: its left edge, how wide, and its top and
    // height in the view, or null while the keys are on the rows.
    function rule() {
        if (drawer.block === null)
            return null;
        // From the block the rule is bound to, not the rule's own place,
        // since a binding on the block may run before the rule's has.
        const at = column.mapToItem(drawer, ruleShown.x, drawer.block.ruleTop);
        return {
            x: Math.round(at.x),
            width: Math.round(ruleShown.width),
            top: Math.round(at.y),
            height: Math.round(drawer.pitch)
        };
    }

    height: tall
    opacity: out ? 1 : 0
    // Kept visible while it waits, though drawn in no ink, since a text
    // that isn't visible gives no lines for the keys to go into.
    visible: opacity > 0 || waits
    // Gone once it has faded away, so what it showed is let go: a picture
    // kept while hidden would be read again as the screen's scale changes,
    // after its file may be gone.
    onVisibleChanged: {
        if (!visible && !out && !waits)
            shown = null;
    }
    // It fades in where it rests and out again, on the one curve, never
    // sliding, so nothing beside the card moves as the selection does
    // (VKCF3Z, 2026-10-29).
    Behavior on opacity {
        NumberAnimation {
            duration: drawer.out ? Theme.motion.medium : Theme.motion.short
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Theme.motion.curve
        }
    }
    // Each new thing it shows starts at its top; the same thing read anew,
    // as a row refreshes, stays where it was scrolled.
    onTitleChanged: scrolled = 0

    // The widest of its facts' names, which their values start beyond, so
    // the values share one edge: a name is never cut short, and a value
    // wraps instead (VKCF3Z, 2026-11-03). Only one so long it would leave
    // its figure less than eight figures' room breaks onto a second line.
    FontMetrics {
        id: factFace
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
    }
    // The title's face, as it's drawn, script and all, to tell which of its
    // words are too wide for a line alone (Identifier.titleBreaks).
    FontMetrics {
        id: titleFace
        font: titleText.font
    }
    // A value's face, measured and drawn: a key list's at Note, a script's
    // in its cut beside the mono, and the rest at Figure.
    FontMetrics {
        id: valueFace
        font.family: drawer.listsKeys ? Theme.level.note.family
            : drawer.script !== "" ? Theme.script.mono[drawer.script] : Theme.level.figure.family
        font.pointSize: drawer.listsKeys ? Theme.level.note.pointSize
            : Theme.level.figure.pointSize * drawer.scriptGrown
        font.features: drawer.listsKeys ? ({}) : Theme.level.figure.features
    }
    // The words' own face, a run among them measured by it.
    FontMetrics {
        id: bodyFace
        font: bodyText.font
    }
    // A fact's figure (the fact rows', below): Graph's ten braille cells at
    // Figure, which Meter's stroke is as wide as so the two read as kin, and
    // a capital at Figure, whose middle the stroke is centred on, as Meter's.
    TextMetrics {
        id: figureInk
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
        text: String.fromCharCode(0x28ff).repeat(10)
    }
    TextMetrics {
        id: figureCapital
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
        text: "H"
    }
    // The digits of a script that has its own, which a value may be
    // written in beside the figures' (VKCF3Z, 2026-11-05).
    readonly property var scriptDigits: ({
        arabic: "\u0660\u0661\u0662\u0663\u0664\u0665\u0666\u0667\u0668\u0669"
    })
    // A script with digits of its own, set at a point size, can stand
    // smaller than the figures beside it: Naskh's stand 0.596 of its em and
    // the mono's 0.742, so a Hijri year read smaller than "30 days" in one
    // column. Such a script's values and title are grown until its digits
    // stand as tall as the figures' do, measured from the faces themselves
    // so it holds whichever the theme names, and never past half again, so
    // a face that measures wrongly can't swamp a plate. Hebrew's letters
    // are drawn to stand with Latin as the mono's do, and CJK's fill their
    // em, so they keep their size. Both are measured at ten times Figure's
    // size, since a face's bounds come in whole pixels, and at Figure's a
    // pixel's rounding put the ratio at 1.17 rather than 1.25. Neither face
    // draws anything, so the size they measure at isn't type shown.
    readonly property int measuredLarger: 10
    FontMetrics {
        id: scriptDigitFace
        font.family: drawer.script !== "" ? Theme.script.mono[drawer.script]
            : Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize * drawer.measuredLarger
    }
    FontMetrics {
        id: figureDigitFace
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize * drawer.measuredLarger
        font.features: Theme.level.figure.features
    }
    // A binding doesn't see what a call reads, so it reads both faces.
    // A script's cut takes no axes of the serif's.
    readonly property var noAxes: ({})
    readonly property real scriptGrown: {
        void scriptDigitFace.font;
        void figureDigitFace.font;
        const digits = scriptDigits[script] ?? "";
        if (digits === "")
            return 1;
        const stands = scriptDigitFace.tightBoundingRect(digits).height;
        const figures = figureDigitFace.tightBoundingRect("0123456789").height;
        return stands > 0 ? Math.min(Math.max(figures / stands, 1), 1.5) : 1;
    }
    // A value in Latin letters on a script's plate (`latin: true`), as a
    // count or a day is, at Figure in the theme's own face, since the
    // script's cut draws Latin in another's.
    FontMetrics {
        id: latinFace
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
        font.features: Theme.level.figure.features
    }
    // A value that's words with no figure in them, as "no sunset today",
    // in the face words are read in, as a notice's are.
    FontMetrics {
        id: wordsFace
        font.family: Theme.level.words.family
        font.pointSize: Theme.level.words.pointSize
        font.variableAxes: Theme.level.words.variableAxes
    }
    // An identifier (`identifies: true`), a node, a serial or a bus name, in
    // the Name face, as a row's name is, so it reads as something to copy
    // rather than a figure.
    FontMetrics {
        id: nameFace
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
        font.weight: Theme.level.name.weight
    }
    // The face a fact's value is measured and drawn in (DrawerPage.js's
    // faceOf).
    function faceOf(fact) {
        return ({ name: nameFace, words: wordsFace, latin: latinFace, value: valueFace })[
            Page.faceOf(fact, drawer.script)];
    }
    // Every fact it shows, the moment's groups' among them.
    readonly property var allFacts: facts.concat(...groups.map(group => group.facts))
    // One drawer lays out one way (VKCF3Z, 2026-11-07): it reads as a table
    // when every fact keeps its value beside its name, and as a list, each
    // value on the line under its name, when one can't; a mix reads as
    // neither. A value that's prose (`words: true`) always stands under its
    // name as a paragraph and counts for neither, so a sentence never turns
    // a table into a list. A key list's sayings stay beside every key, and
    // the moment's beside every name, so its column is never narrower than
    // the widest of those. A binding doesn't see what a call reads, so it
    // reads every face, to be measured again as a key list's face comes and
    // goes.
    readonly property var laying: {
        void valueFace.font;
        void factFace.font;
        void wordsFace.font;
        void latinFace.font;
        void nameFace.font;
        void figureInk.advanceWidth;
        const room = column.width - labelsGap;
        const most = room - valueFace.advanceWidth("00000000");
        // Its value as it would stand in `beside`, a list with a tail as the
        // line it's drawn on there (DrawerPage.besideText).
        const valueWidth = (fact, beside) => {
            if (fact.value === null || fact.value === undefined)
                return 0;
            const face = faceOf(fact);
            const said = Page.besideText(fact, beside, text => face.advanceWidth(text));
            return face.advanceWidth(unbroken(said)) + figureLeast(fact);
        };
        const named = facts.filter(fact => fact && fact.heading === undefined && !proseFact(fact));
        const moment = [].concat(...groups.map(group => group.facts));
        const widest = Math.max(0, ...named.concat(moment)
            .map(fact => Math.ceil(nameWidth(fact))));
        const wide = Math.min(most, widest);
        return {
            wide,
            beside: listsKeys || widest <= most
                && named.every(fact => valueWidth(fact, room - wide) <= room - wide)
        };
    }
    // The names' column: as wide as the widest name, short of a value's
    // eight figures.
    readonly property real labelsWide: laying.wide
    // Whether its facts stand beside their names, a table, else under them.
    readonly property bool besideAll: laying.beside
    // Whether a fact's value is prose, which stands under its name always.
    // (The drawer's own prose, above, is whether its body reads as prose.)
    function proseFact(fact) {
        return fact?.words === true;
    }
    // The least room a fact's figure takes after its value, two runs of one
    // thing apart as Meter's figure and stroke are: a spark Graph's ten
    // cells, a stroke three of them, short of which a share no longer reads.
    function figureLeast(fact) {
        const figure = fact?.figure ?? null;
        return figure === null ? 0 : Theme.space.rung.apart + figureInk.advanceWidth
            * (figure.shape === "stroke" ? 0.3 : 1);
    }
    // A key's name as runs, { said, x }: each key of a chord a run of its
    // own, the tie rung from the next, so Space Tab reads as one key, and
    // chord from chord the apart rung, two runs of one thing, rather than
    // typed spaces that sat nearly as wide as the key from its saying
    // (VKCF3Z, 2026-11-05).
    function runsOf(fact) {
        const chords = fact?.keys ?? (fact?.chords ?? []).map(chord => [chord]);
        const runs = [];
        let x = 0;
        chords.forEach((keys, chord) => {
            if (chord > 0)
                x += Theme.space.rung.apart - Theme.space.rung.tie;
            for (const said of keys) {
                runs.push({ said, x });
                x += factFace.advanceWidth(said) + Theme.space.rung.tie;
            }
        });
        return runs;
    }
    // How wide a fact's name is drawn: its runs and the room between, or
    // its label.
    function nameWidth(fact) {
        const runs = runsOf(fact);
        if (runs.length === 0)
            return factFace.advanceWidth(fact?.label ?? "");
        const last = runs[runs.length - 1];
        return last.x + factFace.advanceWidth(last.said);
    }
    // How many of its facts take more than a line, a name or a value
    // wrapped, for the launcher to report when asked: a key list's take one.
    function factsWrapped() {
        let wrapped = 0;
        for (let at = 0; at < factRows.shown; at++) {
            const row = factRows.itemAt(at);
            if (row && !row.heading && row.lines > 1)
                wrapped += 1;
        }
        return wrapped;
    }
    // How many of its values stand under their names rather than beside
    // them, for the launcher to report: a key list's never do.
    function factsUnder() {
        let under = 0;
        for (let at = 0; at < factRows.shown; at++) {
            const row = factRows.itemAt(at);
            if (row && !row.heading && row.under)
                under += 1;
        }
        return under;
    }
    // Whether its facts are laid one way, every value beside its name or
    // every one under, prose aside, for the launcher to report.
    function laidOneWay() {
        const laid = Array.from({ length: factRows.shown }, (_, at) => factRows.itemAt(at))
            .filter(row => row && !row.heading && !proseFact(row.modelData))
            .map(row => row.under);
        return laid.every(under => under) || laid.every(under => !under);
    }
    // Each fact's figure as drawn: its shape where it shows, "" where it
    // carries none or has given way, for the launcher to report.
    function figuresDrawn() {
        return Array.from({ length: factRows.shown }, (_, at) => {
            const row = factRows.itemAt(at);
            return row && row.figureShown ? String(row.figure.shape ?? "") : "";
        });
    }
    // The facts' rows drawn, each a fact row's item.
    function drawnRows() {
        const rows = [];
        for (let at = 0; at < factRows.shown; at += 1) {
            const row = factRows.itemAt(at);
            if (row !== null)
                rows.push(row);
        }
        return rows;
    }
    // Whether every name is drawn on one line, the moment's in their
    // column and the facts' where each is drawn, for the launcher to
    // report when asked.
    function labelsWhole() {
        const moment = [].concat(...groups.map(group => group.facts));
        return moment.every(fact => nameWidth(fact) <= labelsWide)
            && drawnRows().every(row => row.nameWhole);
    }
    // Whether every value drawn keeps each of its words whole, none so
    // wide where it's drawn that it breaks partway, for the launcher to
    // report when asked. An identifier's words are its parts between its
    // own marks, its hyphens among them, since it breaks there as a
    // typesetter breaks a path (DrawerPage.identifierWords), so a UUID
    // broken at its hyphens keeps each of its words whole; a hash's are
    // its letters, since it has no words of its own. An identifier's
    // lines are read as it's laid on them too: each fits, and none after
    // its first opens with a mark but after a space, read from the lines
    // it's drawn on (valueDrawn; Y2YBTS, 2026-11-09). A drawer with no
    // value known yet has nothing to say this of, so it says no.
    function valuesWhole() {
        const fits = row => word => faceOf(row.modelData).advanceWidth(word) <= row.valueWide;
        const known = drawnRows().filter(row => row.known);
        const laid = row => valueDrawn(row).split("\n");
        return known.length > 0 && known.every(row => row.modelData.identifies === true
            ? Page.identifierWords(row.modelData.value).every(fits(row))
                && laid(row).every(line => fits(row)(line.replace(/\u00a0/g, " ")))
                && Identifier.laidWell(row.modelData.value, laid(row))
            : unbroken(row.modelData.value).split(" ")
                .every(word => fits(row)(word) && !breaksAtItsHyphen.test(word)));
    }
    // A fact's value on the lines it's drawn on, an ellipsis while it's
    // asked: one given as items (`valueItems`, `between`) on the lines
    // DrawerPage.valueLines lays them on, or with a `tail` on the one line
    // valueLine gives way on, an identifier as identifierKept lays it, and
    // any other as itemsKept does. Its `value` is still what's copied.
    function valueDrawn(fact) {
        const data = fact.modelData;
        const width = text => faceOf(data).advanceWidth(text);
        if (!fact.known) {
            return "\u2026";
        }
        if (data.valueItems) {
            return (data.tail ? Page.valueLine(data.valueItems, data.tail, fact.valueWide, width)
                : Page.valueLines(data.valueItems, data.between ?? " \u00b7 ", fact.valueWide,
                    width)).join("\n");
        }
        return data.identifies === true
            ? identifierKept(data.value, faceOf(data), fact.valueWide)
            : itemsKept(data.value, faceOf(data), fact.valueWide);
    }
    // A hyphen inside a word, "Wi-Fi" or "v7.17.4-5", that a line may still
    // break after, leaving "Wi-" at a line's end and "Fi" alone on the next.
    readonly property var breaksAtItsHyphen: /[A-Za-z0-9]-(?!\u2060)[A-Za-z0-9]/
    // Whether every value beside its name takes one line there, so the
    // facts are each beside or under and never a mix, for the launcher
    // to report when asked.
    function besideOnOneLine() {
        return drawnRows().every(row => row.oneLineBeside);
    }
    // Whether every value in its script's own digits has them stand as
    // tall as the figures' do, to a twentieth or the pixel a face's bounds
    // are rounded to, so one column reads at one size, for the launcher to
    // report when asked.
    function digitsStandAsFigures() {
        const digits = scriptDigits[script] ?? "";
        const figures = latinFace.tightBoundingRect("0123456789").height;
        const own = value => digits.split("").some(digit => String(value).includes(digit));
        return drawnRows().every(row => !row.known || !own(row.modelData.value)
            || Math.abs(faceOf(row.modelData).tightBoundingRect(digits).height - figures)
                <= Math.max(1, figures / 20));
    }
    // Where the last line it shows ends, down from its top.
    readonly property real foot: Theme.border + drawer.viewTop + view.height
    // Between the names' column and the values', two columns, so the two
    // never read as one.
    readonly property real labelsGap: Theme.space.rung.groups
    // Where any thing in a list takes more than a line as it's laid out,
    // every thing in it stands a beat further from the one before, so a
    // wrapped line sits nearer its own thing than the next and the list
    // reads by thing rather than by line; a heading keeps a pitch above
    // its first (VKCF3Z, 2026-11-05).
    function apartIn(wraps, first) {
        return wraps && !first ? Theme.space.beat : 0;
    }
    // Whether any of a Repeater's things takes more than a line, of those
    // it shows where it keeps more hidden.
    function anyWraps(things) {
        return Array.from({ length: things.shown ?? things.count }, (_, at) => things.itemAt(at))
            .some(thing => (thing?.lines ?? 1) > 1);
    }
    // A number and its unit kept on one line, so "8 s" never leaves its s
    // alone on the next (VKCF3Z, 2026-11-03), and a word with a hyphen in it
    // kept whole, a word joiner after its hyphen, so "Wi-Fi" never leaves
    // its Fi alone on the next. The joiner draws nothing, and only what's
    // drawn carries it: what Return copies is the value as it was given.
    function unbroken(words) {
        return String(words).replace(/(\d) (?=[A-Za-z\u00b5\u00b0%])/g, "$1\u00a0")
            .replace(/([A-Za-z0-9])-(?=[A-Za-z0-9])/g, "$1-\u2060");
    }
    // An identifier, as a supply's "ucsi-source-psy-USBC000:001" or a
    // desktop entry's "com.onepassword.OnePassword", breaks at its own
    // marks rather than draw past the frame (VKCF3Z, 2026-11-08 and 09), as
    // DrawerPage.identifierLines lays it; a line that fits is held whole.
    function identifierKept(words, face, wide) {
        return Page.identifierLines(words, wide, text => face.advanceWidth(text))
            .map(line => face.advanceWidth(line) <= wide ? line.replace(/ /g, "\u00a0") : line)
            .join("\n");
    }

    // A value made of items, "38 s \u00b7 4.1k out \u00b7 0.9 s first token",
    // wraps between them, never inside one, the dot going with the break
    // (DrawerFigure.runLines); a line that fits is held whole, and an item
    // wider than its room still wraps by its words, never running past the
    // edge.
    function itemsKept(words, face, wide) {
        return Figure.runLines(unbroken(words), wide, text => face.advanceWidth(text))
            .map(line => face.advanceWidth(line) <= wide ? line.replace(/ /g, "\u00a0") : line)
            .join("\n");
    }

    // The time, read as it turns each second while the moment shows.
    SystemClock {
        id: clock
        enabled: drawer.moment && drawer.visible
        precision: SystemClock.Seconds
    }
    // The time's capitals, which are centred on Now's row.
    FontMetrics {
        id: timeFace
        font: timeText.font
    }

    // How far down the view may show, from its top: to the foot of a line
    // whose baseline stands the inset above the frame's inside bottom, as
    // the text keeps it on every other side, so the bottom is set rather
    // than what whole lines leave. The last line shown is the last whole
    // one there, which shares the card's foot baseline when it falls on
    // that beat; its keys, gone into, bring up the rest (VKCF3Z,
    // 2026-11-05).
    readonly property real viewRoom: drawer.tall - 2 * Theme.border - Theme.space.rung.inset
        + drawer.pitch / 2 - drawer.dropped - drawer.viewTop
    // Where the view begins, down from the frame's inside top: a line's
    // top over the eyebrow's baseline, so a line scrolled to the view's
    // top takes the eyebrow's baseline, its capitals the inset under the
    // frame as the head's are, and the lines scrolled past hide whole
    // above it, as the bottom hides them, the frame's room the same
    // scrolled or not (VKCF3Z, 2026-11-05). At rest it shows its content
    // from there, the head's, which begins below it, so nothing moves as
    // the drawer comes.
    readonly property real viewTop: drawer.fieldBase - Theme.border - drawer.pitch / 2
        - drawer.dropped
    // The top of the body's first line, from which the pitch its tops
    // scroll to past the rest is counted, so each is a line's top.
    readonly property real stops: drawer.bodyBase - Theme.border - drawer.pitch / 2
        - drawer.dropped
    // The furthest its top goes: its rest where everything fits, else the
    // first of its stops whose view reaches the last line, so scrolled to
    // its end its lines are still the card's.
    readonly property real viewEnd: view.natural <= viewTop + viewRoom ? viewTop
        : stops + Math.ceil((view.natural - viewRoom - stops) / pitch - rounding) * pitch
    // The top of every line's box it lays on the pitch past its title, down
    // the content: the facts', the moment's, the preview's and the words'.
    // Each a fact takes as { top, bottom }, all its lines, a heading held
    // with the thing under it; and each line of the moment, the preview
    // and the words, which run on as reading does.
    function lineSpans() {
        const spans = [];
        // A line of the words says which it is, since they're counted by
        // paragraph, not line.
        const run = (top, count, words) => {
            for (let at = 0; at < count; at++)
                spans.push({ top: top + at * drawer.pitch, bottom: top + (at + 1) * drawer.pitch,
                    line: words ? at : -1 });
        };
        let heading = null;
        for (let at = 0; at < factRows.shown; at++) {
            const row = factRows.itemAt(at);
            if (!row)
                continue;
            const top = flow.y + factsShown.y + row.y + row.base - drawer.pitch / 2
                - drawer.dropped;
            const span = { top: heading ?? top, bottom: top + row.lines * drawer.pitch };
            heading = row.heading ? span.top : null;
            if (!row.heading)
                spans.push(span);
        }
        for (let at = 0; at < groupsShown.count; at++) {
            const group = groupsShown.itemAt(at);
            for (const base of group?.bases ?? [])
                run(group.y + base - drawer.pitch / 2 - drawer.dropped, 1);
        }
        if (previewLines.visible)
            run(flow.y + previewBox.y, previewBox.lines.length);
        if (wordsBox.visible)
            run(flow.y + wordsBox.y, bodyText.lineCount, true);
        return spans;
    }
    // Where the view ends: at its room, or where a thing would run past it,
    // at that thing's top, so a thing is shown whole, all its lines, or not
    // at all (VKCF3Z, 2026-11-05: a saying's first line alone reads as
    // cut). A list whose things wrap stands them a beat off the pitch the
    // room is counted on, so one may fall across it. A thing begun above
    // the view, taller than it shows, ends at its last whole line instead.
    // Where anything is hidden below, the last line's slot says so, as
    // Vim marks a line cut off, so a reader needn't remember the rest is
    // there (VKCF3Z, 2026-11-05): the lowest of the card's beats the inset
    // above the frame's inside bottom, a thing's step below the last
    // thing shown, which ends a step sooner for it.
    readonly property real moreBase: rowBaseline + Math.floor((tall - Theme.border
        - Theme.space.rung.inset - rowBaseline) / Theme.space.beat + rounding) * Theme.space.beat
    readonly property var shownAndHidden: {
        const spans = lineSpans();
        const top = drawer.shownFrom;
        const ending = end => Math.min(end, ...spans.filter(span => span.top < end - 0.5
            && span.bottom > end + 0.5).map(span => span.top > top + 0.5 ? span.top
                : span.top + Math.floor((end - span.top + 0.5) / drawer.pitch) * drawer.pitch));
        const past = end => spans.filter(span => span.bottom > end + 0.5);
        // Each thing past the end but the words' lines, and the first of
        // those, from which the words' paragraphs are counted.
        const hiddenPast = end => {
            const hidden = past(end);
            const words = hidden.filter(span => span.line >= 0);
            return { shown: end - top, hidden: hidden.length - words.length,
                wordsFrom: words.length > 0 ? words[0].line : -1 };
        };
        const whole = ending(top + viewRoom);
        if (past(whole).length === 0)
            return { shown: whole - top, hidden: 0, wordsFrom: -1 };
        const step = drawer.pitch + drawer.apartIn(drawer.anyWraps(factRows), false);
        // A thing's box ends half a pitch, less the drop, under its last
        // baseline, which stands a step over the slot's.
        const short = ending(Math.min(top + viewRoom, top + moreBase - Theme.border - viewTop
            - step + drawer.pitch / 2 - drawer.dropped));
        return hiddenPast(short);
    }
    readonly property real shownTo: shownAndHidden.shown
    // How many things it hides below, and what its last slot says of
    // them: in the drawer's own keys, how to go in and read them, which
    // go once the keys are in. Each fact is one and each paragraph of its
    // words, a heading none, never a line (VKCF3Z, 2026-11-05: "16 more"
    // counting lines reads as wrong). The words' reader says which letters
    // the first hidden line holds, from which the paragraphs left are
    // counted, the one it cuts among them.
    readonly property int hiddenBelow: {
        const from = shownAndHidden.wordsFrom;
        const reader = bodyText.reader;
        if (from < 0 || !reader)
            return shownAndHidden.hidden;
        const at = reader.positionAt(0, reader.drop + from * bodyText.leading + 1);
        // A run's lines are where it was laid to wrap, one paragraph still.
        const left = reader.getText(at, reader.length);
        return shownAndHidden.hidden
            + Page.paragraphs(drawer.runs ? left.replace(/[\n\u2028]/g, " ") : left);
    }
    // Whether the words are cut at the foot, so their reader is made to
    // count their paragraphs. It's handed to the words once the cut is
    // worked out, never bound: making the reader sets the words' text
    // again, from which their lines and so the cut are worked out, which
    // a binding reads as a loop though no letter moves.
    readonly property bool wordsCut: shownAndHidden.wordsFrom >= 0
    onWordsCutChanged: Qt.callLater(drawer.countWords)
    Component.onCompleted: {
        drawer.countWords();
        drawer.keptTo(factsKept, facts.length);
    }
    function countWords() {
        bodyText.counting = drawer.wordsCut;
    }
    readonly property string moreCount: hiddenBelow === 0 ? "" : `${hiddenBelow} more`
    // The way in, while the drawer doesn't hold the keys already.
    readonly property string moreKey: drawer.keys ? "" : "Ctrl+W l"
    readonly property string moreSays: moreCount === "" ? ""
        : moreCount + (moreKey === "" ? "" : ` \u00b7 ${moreKey}`)
    // What the way in is drawn as, key by key.
    readonly property var moreKeys: moreKey === "" ? [] : moreHints.partsOf(moreKey)

    // How many are hidden, then the way in drawn as the rows and the foot
    // draw a chord, key by key with a tie each side of its + (KeyHints), a
    // muted dot a mark from each as a status line joins its parts, so one
    // chord reads one way everywhere (VKCF3Z, 2026-11-07).
    Row {
        x: column.x
        y: drawer.moreBase - Theme.border - moreText.baselineOffset
        visible: drawer.moreSays !== ""
        spacing: Theme.space.rung.mark

        Text {
            id: moreText
            text: drawer.moreCount
            textFormat: Text.PlainText
            color: Theme.muted
            font.family: Theme.level.note.family
            font.pointSize: Theme.level.note.pointSize
        }

        Text {
            visible: drawer.moreKey !== ""
            text: moreHints.dot
            color: Theme.muted
            font.family: Theme.level.note.family
            font.pointSize: Theme.level.note.pointSize
        }

        KeyHints {
            id: moreHints
            visible: drawer.moreKey !== ""
            keys: drawer.moreKey === "" ? [] : [[drawer.moreKey, ""]]
        }
    }

    Flickable {
        id: view
        y: drawer.viewTop
        width: parent.width
        height: drawer.shownTo
        contentY: drawer.shownFrom
        clip: true
        interactive: false
        contentWidth: width
        readonly property real natural: drawer.moment ? momentShown.ends : flow.y + flow.height
        // Room past every top the keys scroll to (viewEnd), however short
        // the page, never its own height: a Flickable whose content changes
        // pulls contentY back inside its bounds, which would leave the view
        // resting at the page's top rather than viewTop until the next key.
        contentHeight: drawer.viewTop + drawer.viewEnd + drawer.tall

        Item {
            id: column
            // The letters the inset from the frame's inside edge on either
            // side, as on every frame. Its width is what the screen leaves
            // beside the card (Launcher.qml's drawerWidth), so the card never
            // moves as it comes and goes; a plate whose lines are short
            // leaves that room at their right, given by the screen rather
            // than taken by the plate.
            x: Theme.space.rung.inset
            width: view.width - 2 * x
            height: flow.y + flow.height + Theme.space.rung.inset

            // V's bands, under the letters: the selection's ground while it
            // chooses, and the accent while y flashes them, as his Neovim's
            // Visual and on_yank are; the letters keep their own ink.
            Repeater {
                id: bandsShown
                model: drawer.bands
                delegate: Rectangle {
                    required property var modelData
                    z: -1
                    x: -Theme.space.rung.mark
                    y: modelData.top
                    width: column.width + 2 * Theme.space.rung.mark
                    height: modelData.height
                    color: drawer.choosing === "" ? Theme.accent
                        : Qt.alpha(Theme.selection, Theme.floating.selection)
                }
            }
            // The cursor's line while it holds the keys: one rule in the
            // accent, a hairline, a mark from the letters, the drawer's
            // pitch tall on its capitals' middle, one height on every level
            // (VKCF3Z, 2026-11-05). It stays lit, as a cursor line does,
            // rather than blinking as a caret.
            Rectangle {
                id: ruleShown
                visible: drawer.block !== null
                x: -Theme.space.rung.mark - width
                y: drawer.block?.ruleTop ?? 0
                width: Theme.border
                height: drawer.pitch
                color: Theme.accent
            }
            // Visual's block, on the cursor's letter while it chooses, as his
            // Neovim shows its cursor only there: the accent under the letter
            // in the ground's ink, steady as the rule is.
            Rectangle {
                id: blockShown
                z: 1
                visible: drawer.block !== null && drawer.choosing !== ""
                x: drawer.block?.x ?? 0
                y: drawer.block?.y ?? 0
                width: drawer.block?.width ?? 0
                height: drawer.block?.height ?? 0
                color: Theme.accent

                Text {
                    y: (drawer.block?.base ?? 0) - (drawer.block?.y ?? 0) - baselineOffset
                    text: drawer.block?.letter ?? ""
                    textFormat: Text.PlainText
                    color: Theme.background
                    font: drawer.block?.font ?? titleText.font
                }
            }

            // When or where, in capitals over the plate, on the field's
            // centre: a label, set apart by case and tracking rather than
            // size alone, so it reads as one before the name; or, naming
            // something, as it is written (eyebrowLevel).
            PageText {
                id: eyebrowText
                sought: drawer.washing
                width: parent.width
                y: drawer.fieldBase - Theme.border - baselineOffset
                visible: drawer.eyebrow !== ""
                plain: drawer.eyebrowSaid
                leading: 0
                wraps: false
                ink: Theme.muted
                font.family: drawer.eyebrowLevel.family
                font.pointSize: drawer.eyebrowLevel.pointSize
                font.weight: drawer.eyebrowLevel.weight
                font.italic: drawer.eyebrowLevel.italic
                font.variableAxes: drawer.eyebrowLevel.variableAxes
                font.capitalization: drawer.eyebrowLevel.capitalization
                font.letterSpacing: drawer.eyebrowLevel.letterSpacing
                font.features: drawer.eyebrowLevel.features
            }

            // The moment: the time, its seconds, what needs Joel and the
            // facts behind it. It was set on home's rows once (look 3), to
            // tie it to the list beside it; that made every gap on it one,
            // and Now the one plate set by its own rule, for an alignment
            // nobody sees once the list scrolls (VKCF3Z, 2026-11-05). The
            // room under its last group is the card's height, which the
            // drawer keeps so the card never moves; a day with less to say
            // leaves it, rather than spreading the plate to fill it.
            Item {
                id: momentShown
                width: parent.width
                visible: drawer.moment
                // Where its last line's box ends, for the view to hold.
                readonly property real ends: groupsShown.count === 0
                    ? timeText.y + timeText.height : groupsShown.lastBottom

                // The time, the one large thing, at Display in the mono, as
                // the lock's clock is, since figures are the mono's: all one
                // width so they never jiggle, a Display pitch under the
                // eyebrow, as a title is its own under one.
                PageText {
                    id: timeText
                    sought: drawer.washing
                    readonly property real base: drawer.fieldBase + Theme.level.display.pitch
                    y: base - Theme.border - baselineOffset
                    plain: Moment.clock(clock.date)
                    leading: 0
                    wraps: false
                    ink: Theme.strong
                    font.family: Theme.level.display.family
                    font.pointSize: Theme.level.display.pointSize
                    font.weight: Theme.level.display.weight
                    font.letterSpacing: Theme.level.display.letterSpacing
                    font.features: Theme.level.display.features
                }

                // Its seconds after it, quiet and counting, at Note on the
                // time's baseline: upright, since italic is only the voice.
                // A mark from the time's ink to its own, a smaller figure
                // beside the time rather than a thing apart from it.
                PageText {
                    id: secondsText
                    sought: drawer.washing
                    x: timeText.x + timeInk.drawnEnd + Theme.space.rung.mark
                        - secondsInk.tightBoundingRect.x
                    width: parent.width - x
                    y: timeText.base - Theme.border - baselineOffset
                    plain: String(drawer.shown?.seconds ?? "")
                        .replace("{seconds}", String(clock.date.getSeconds()))
                    leading: 0
                    elides: true
                    ink: Theme.muted
                    font.family: Theme.level.note.family
                    font.pointSize: Theme.level.note.pointSize
                    font.features: { "tnum": 1 }
                }

                // The time's ink and the seconds', read by TextMetrics,
                // which follow the face as it loads (HVD6O3, 2026-11-05).
                TextMetrics {
                    id: timeInk
                    // Where the time's ink ends as it's drawn: its box leaves
                    // out the letter spacing the Text puts between its
                    // letters, the Display level's tracking, which set the
                    // seconds 2 past their mark (10 at 2, 9.3 at 1.5). The
                    // spacing is read off the time's Text, since a binding
                    // here that reads this object's own font loops.
                    readonly property real drawnEnd: tightBoundingRect.x
                        + tightBoundingRect.width
                        + Math.max(0, timeText.plain.length - 1) * timeText.font.letterSpacing
                    font: timeText.font
                    text: timeText.plain
                }
                TextMetrics {
                    id: secondsInk
                    font: secondsText.font
                    text: secondsText.plain
                }

                // Each group of facts as a body under the head, the first
                // on the first of the card's beats two Title pitches under
                // it, as every plate's body is, and each after the
                // drawer's groups apart, two pitches from the last baseline
                // of the one before.
                Repeater {
                    id: groupsShown
                    model: drawer.groups
                    // Where the last group's last line's box ends.
                    readonly property real lastBottom: {
                        const group = count > 0 ? itemAt(count - 1) : null;
                        return group ? group.y + group.height : 0;
                    }
                    // Every group's words, for the keys' lines, joined
                    // by concat, since Qt's engine has no Array's flat.
                    function texts() {
                        return [].concat(...Array.from({ length: count },
                            (_, at) => itemAt(at)?.texts() ?? []));
                    }

                    Column {
                        id: group
                        required property var modelData
                        required property int index
                        readonly property real headBase: timeText.base
                        readonly property real base: index === 0
                            ? drawer.onBeat(headBase + 2 * Theme.level.title.pitch)
                            : (groupsShown.itemAt(index - 1)?.lastBase ?? headBase)
                                + 2 * drawer.pitch
                        // Each fact's first baseline, down from its top.
                        readonly property var bases: Array.from({ length: facts.count },
                            (_, at) => {
                                const row = facts.itemAt(at);
                                return row ? row.y + row.base : 0;
                            })
                        // Its last fact's first baseline.
                        readonly property real lastBase: base + height - drawer.pitch
                        // Its facts' names and figures.
                        function texts() {
                            return [].concat(...Array.from({ length: facts.count }, (_, at) => {
                                const row = facts.itemAt(at);
                                return row ? [row.name, row.figure] : [];
                            }));
                        }

                        // Named, not its parent, which a Repeater's
                        // delegate has none of while it's made or let go.
                        width: momentShown.width
                        y: base - Theme.border - drawer.pitch / 2 - drawer.dropped

                        Repeater {
                            id: facts
                            model: group.modelData.facts
                            readonly property bool wraps: drawer.anyWraps(facts)

                            Item {
                                id: momentFact
                                required property var modelData
                                required property int index
                                readonly property Item name: factName
                                readonly property Item figure: figure
                                readonly property int lines: Math.max(1, factName.lineCount,
                                    figure.lineCount)
                                readonly property real apart: drawer.apartIn(facts.wraps,
                                    index === 0)
                                // Its first baseline, down from its top.
                                readonly property real base: apart + drawer.pitch / 2
                                    + drawer.dropped
                                width: group.width
                                height: apart + lines * drawer.pitch

                                PageText {
                                    id: factName
                                    sought: drawer.washing
                                    width: drawer.labelsWide
                                    y: momentFact.base - baselineOffset
                                    plain: momentFact.modelData.label ?? ""
                                    leading: drawer.pitch
                                    ink: Theme.muted
                                    font.family: Theme.level.note.family
                                    font.pointSize: Theme.level.note.pointSize
                                }

                                PageText {
                                    id: figure
                                    x: drawer.labelsWide + drawer.labelsGap
                                    width: parent.width - x
                                    y: momentFact.base - baselineOffset
                                    plain: drawer.qualified(momentFact.modelData)
                                    styled: true
                                    leading: drawer.pitch
                                    ink: Theme.text
                                    font.family: Theme.level.figure.family
                                    font.pointSize: Theme.level.figure.pointSize
                                    font.features: Theme.level.figure.features
                                }
                            }
                        }
                    }
                }
            }

            // The title, then its facts, words and picture, each a group
            // of whole lines one empty line apart, from the title's line
            // on, or the body's where there's no title.
            Column {
                id: flow
                y: (titleBlock.visible ? drawer.titleBase : drawer.bodyBase) - Theme.border
                    - drawer.pitch / 2 - drawer.dropped
                width: parent.width
                spacing: drawer.pitch
                visible: !drawer.moment

                // The thing's name at Title, sized by capitals as every
                // serif step is, a Title pitch under the eyebrow, whatever
                // its own line box would put it on. A name that wraps sets
                // its lines a Title's leading apart, and the block ends a
                // line above the body's first, so what follows starts on
                // the card's beat.
                Item {
                    id: titleBlock
                    width: parent.width
                    height: drawer.bodyBase - drawer.titleBase - drawer.pitch
                    visible: drawer.title !== ""

                    PageText {
                        id: titleText
                        sought: drawer.washing
                        marked: "# "
                        width: parent.width
                        y: drawer.pitch / 2 + drawer.dropped - baselineOffset
                        // A word too wide for a line, as a server's name or
                        // a path, may break after a dot or a slash rather
                        // than mid-word; the rest break at their spaces. A
                        // binding doesn't see what a call reads, so it reads
                        // the face, to choose again as the face loads.
                        plain: {
                            void titleFace.font;
                            return Identifier.titleBreaks(drawer.title, width,
                                words => titleFace.advanceWidth(words));
                        }
                        leading: drawer.titleLeading
                        // A right-to-left title starts under its eyebrow
                        // too, rather than at the plate's far edge.
                        horizontalAlignment: Text.AlignLeft
                        ink: Theme.level.title.colour
                        font.family: drawer.script === "" ? Theme.level.title.family
                            : Theme.script.serif[drawer.script]
                        font.pointSize: Theme.level.title.pointSize * drawer.scriptGrown
                        font.letterSpacing: Theme.level.title.letterSpacing
                        // The serif's axes are its own, so a script's cut
                        // takes its own weight instead.
                        font.variableAxes: drawer.script === "" ? Theme.level.title.variableAxes
                            : drawer.noAxes
                    }
                }

                // Its preview, right under its name (VKCF3Z, 2026-11-03): a
                // picture in its own shape, as wide as the plate at most and
                // never so tall it pushes the facts after it out of sight, or
                // lines in the mono at body size, each whole or cut short,
                // a folder's items quiet. Lines take whole lines of the
                // pitch, so everything after them stays on the card's; a
                // picture its own height, what follows a gap below it on
                // the next of the card's beats (DrawerPage.previewHeight).
                Item {
                    id: previewBox
                    readonly property string picture: drawer.preview?.picture ?? ""
                    // As many lines as the room holds, so the facts stay
                    // in sight under them.
                    readonly property var lines: (drawer.preview?.lines ?? [])
                        .slice(0, Math.floor(room / drawer.pitch))
                    // The room it may take: what's left of the view once
                    // the name and the facts have theirs, a line apart.
                    readonly property real room: Math.max(4 * drawer.pitch, drawer.viewTop
                        + drawer.viewRoom - flow.y - (titleBlock.visible ? titleBlock.height
                        + drawer.pitch : 0) - (factsShown.visible ? factsShown.height
                        + drawer.pitch : 0) - drawer.pitch)

                    width: parent.width
                    // A picture waits for its size, held empty rather than
                    // drawn square and then jumping to its shape.
                    visible: (picture !== "" && previewFrame.height > 0) || lines.length > 0
                    height: Page.previewHeight(picture !== "" ? previewFrame.height : 0,
                        lines.length, {
                            pitch: drawer.pitch,
                            beat: Theme.space.beat,
                            gap: Theme.space.rung.groups
                        })

                    Rectangle {
                        id: previewFrame
                        readonly property real across: drawer.preview?.across > 0
                            ? drawer.preview.across : parent.width
                        // Its size as given, else its shape once it's read;
                        // nothing before either.
                        readonly property real down: drawer.preview?.down > 0
                            ? drawer.preview.down
                            : previewDrawn.status === Image.Ready && previewDrawn.implicitWidth > 0
                                ? across * previewDrawn.implicitHeight / previewDrawn.implicitWidth
                                : 0
                        readonly property real shrunk: down <= 0 ? 0 : Math.min(1,
                            parent.width / across,
                            Math.floor(previewBox.room / drawer.pitch) * drawer.pitch / down)

                        visible: previewBox.picture !== ""
                        width: Math.round(across * shrunk)
                        height: Math.round(down * shrunk)
                        radius: Theme.inner
                        color: "transparent"

                        // It comes in once it's drawn, over the short
                        // duration, rather than as a box waiting for it.
                        Image {
                            id: previewDrawn
                            anchors.fill: parent
                            // A path or a URL, a player's art a remote one,
                            // which loads beside the keys, never holding one.
                            source: Pictures.source(previewBox.picture)
                            asynchronous: true
                            fillMode: Image.PreserveAspectFit
                            // Read at the width it may take, never its
                            // drawn size, which waits on its shape, so it's
                            // read once rather than again as it's sized.
                            sourceSize.width: 2 * previewFrame.across
                            opacity: status === Image.Ready ? 1 : 0
                            layer.enabled: true
                            layer.effect: MultiEffect {
                                maskEnabled: true
                                maskSource: previewCorners
                            }
                            Behavior on opacity {
                                NumberAnimation {
                                    duration: Theme.motion.short
                                    easing.type: Easing.BezierSpline
                                    easing.bezierCurve: Theme.motion.curve
                                }
                            }
                            onStatusChanged: {
                                if (status === Image.Ready)
                                    Report.say(`previewed ${drawer.title}`);
                            }
                        }

                        Rectangle {
                            id: previewCorners
                            anchors.fill: parent
                            radius: Theme.inner
                            visible: false
                            layer.enabled: true
                        }
                    }

                    Item {
                        id: previewLines
                        width: parent.width
                        height: parent.height
                        visible: previewBox.picture === "" && previewBox.lines.length > 0

                        Repeater {
                            id: previewRows
                            model: previewBox.lines
                            // Each of its lines, for the keys'.
                            function texts() {
                                return Array.from({ length: count }, (_, at) => itemAt(at));
                            }

                            PageText {
                                required property string modelData
                                required property int index

                                width: previewLines.width
                                y: index * drawer.pitch + drawer.pitch / 2 + drawer.dropped
                                    - baselineOffset
                                plain: modelData
                                sought: drawer.washing
                                leading: 0
                                elides: true
                                ink: drawer.preview?.quiet === true ? Theme.muted : Theme.text
                                font.family: Theme.level.note.family
                                font.pointSize: Theme.level.note.pointSize
                            }
                        }
                    }
                }

                Column {
                    id: factsShown
                    width: parent.width
                    visible: drawer.facts.length > 0
                    // Whole lines, however many beats its things stand
                    // apart, so what follows starts on a line the view's
                    // tops count from (stops, viewEnd): two facts a beat
                    // apart left the words half a line off them, and the
                    // keys could scroll none of the words into view.
                    bottomPadding: Math.ceil(factRows.taken() / drawer.pitch - drawer.rounding)
                        * drawer.pitch - factRows.taken()

                    Repeater {
                        id: factRows
                        model: ListModel {
                            id: factsKept
                        }
                        readonly property bool wraps: drawer.anyWraps(factRows)
                        // How many of its rows show a fact, the rest kept
                        // hidden for a thing with more.
                        readonly property int shown: Math.min(count, drawer.facts.length)
                        // How tall its facts stand together.
                        function taken() {
                            return Array.from({ length: shown }, (_, at) => itemAt(at))
                                .reduce((sum, row) => sum + (row?.height ?? 0), 0);
                        }
                        // Every fact's name and value, for the keys' lines.
                        function texts() {
                            return [].concat(...Array.from({ length: shown }, (_, at) => {
                                const row = itemAt(at);
                                return row ? [row.over, row.name, ...row.laterChords(), row.value]
                                    : [];
                            }));
                        }

                        Item {
                            id: fact
                            required property int index
                            readonly property var modelData: drawer.facts[index] ?? drawer.noFact
                            visible: index < drawer.facts.length
                            Component.onCompleted: drawer.factsMade += 1
                            // A heading over the facts after it, as the
                            // keys' list names each mode's, set as the
                            // eyebrow is and an empty line below any
                            // fact before it.
                            readonly property bool heading: modelData.heading !== undefined
                            readonly property real headingRoom: heading && index > 0 ? drawer.pitch
                                : 0
                            // Room above a fact for a list that wraps,
                            // but under a heading, which stays a pitch
                            // above its first.
                            readonly property real apart: heading ? 0
                                : drawer.apartIn(factRows.wraps, index === 0
                                    || drawer.facts[index - 1]?.heading !== undefined)
                            // How many lines it takes as it's laid out.
                            // A figure under its value takes a line more.
                            readonly property int lines: heading ? Math.max(1, headingText.lineCount)
                                : (under
                                ? label.lineCount + value.lineCount
                                : Math.max(1, label.lineCount, value.lineCount))
                                + (figureBelow ? 1 : 0)
                            readonly property Item over: headingText
                            readonly property Item name: label
                            readonly property Item value: value
                            readonly property bool known: modelData.value !== null
                                && modelData.value !== undefined
                            // Its first baseline, down from its top.
                            readonly property real base: drawer.pitch / 2 + drawer.dropped
                                + headingRoom + apart
                            // A name and its value share one line, or the
                            // value takes the plate's whole width on the
                            // line under its name, as the drawer lays every
                            // fact (besideAll), and as prose always stands.
                            // A key list's sayings stay beside their keys,
                            // however many lines, so each key heads its own
                            // (VKCF3Z, 2026-11-04).
                            readonly property bool under: !drawer.listsKeys
                                && (!drawer.besideAll || drawer.proseFact(modelData))
                            // A key's name, drawn as runs of its own.
                            readonly property var runs: drawer.runsOf(modelData)
                            readonly property real valueFrom: under ? 0
                                : drawer.labelsWide + drawer.labelsGap
                            // How wide its value may run where it's drawn.
                            readonly property real valueWide: width - valueFrom
                            // Whether its name is drawn on one line, and
                            // its value, beside the name, on one too, but
                            // for a key's saying, which may take two.
                            readonly property bool nameWhole: heading || label.lineCount <= 1
                            readonly property bool oneLineBeside: heading || under || !known
                                || drawer.listsKeys || value.lineCount <= 1

                            width: parent.width
                            height: headingRoom + apart + lines * drawer.pitch

                            PageText {
                                id: headingText
                                sought: drawer.washing
                                marked: "## "
                                width: parent.width
                                y: fact.base - baselineOffset
                                visible: fact.heading
                                plain: fact.modelData.heading ?? ""
                                // A heading wraps at a word rather than
                                // being cut, keeping the spec's names
                                // whole: "Normal: moving and acting" lost
                                // its last letters at 2 (VKCF3Z, 2026-11-06).
                                leading: drawer.pitch
                                ink: Theme.muted
                                font.family: Theme.level.label.family
                                font.pointSize: Theme.level.label.pointSize
                                font.weight: Theme.level.label.weight
                                font.capitalization: Theme.level.label.capitalization
                                font.letterSpacing: Theme.level.label.letterSpacing
                                font.features: Theme.level.label.features
                            }

                            PageText {
                                id: label
                                sought: drawer.washing
                                visible: !fact.heading
                                width: fact.under ? fact.width : drawer.labelsWide
                                y: fact.base - baselineOffset
                                // A key's first run, or the whole name.
                                plain: fact.runs.length > 0 ? fact.runs[0].said
                                    : fact.modelData.label ?? ""
                                leading: drawer.pitch
                                ink: drawer.listsKeys ? Theme.text : Theme.muted
                                font.family: Theme.level.note.family
                                font.pointSize: Theme.level.note.pointSize
                            }
                            // Every run of a key's name after its first.
                            function laterChords() {
                                return Array.from({ length: chordsShown.count },
                                    (_, at) => chordsShown.itemAt(at)).filter(text => text);
                            }
                            Repeater {
                                id: chordsShown
                                model: fact.heading ? [] : fact.runs.slice(1)

                                PageText {
                                    required property var modelData
                                    sought: drawer.washing
                                    x: modelData.x
                                    y: fact.base - baselineOffset
                                    plain: modelData.said
                                    leading: drawer.pitch
                                    wraps: false
                                    ink: label.ink
                                    font: label.font
                                }
                            }

                            // Its figure, or a placeholder while it's
                            // asked, on the one edge every value starts
                            // at, a long one breaking onto the pitch: one
                            // given as items (`valueItems`, `between`) on
                            // the lines DrawerPage.valueLines lays them on,
                            // or with a `tail` on the one line valueLine
                            // gives way on, its `value` still what's copied.
                            PageText {
                                id: value
                                sought: drawer.washing
                                visible: !fact.heading
                                x: fact.valueFrom
                                width: parent.width - x
                                y: fact.base - baselineOffset
                                    + (fact.under ? label.lineCount * drawer.pitch : 0)
                                plain: drawer.valueDrawn(fact)
                                leading: drawer.pitch
                                // A fact that warns, as an entry's weak
                                // password does, says so in the warning's
                                // role, as a stroke past its limit does; a
                                // served one names the role it warns in,
                                // its hue (ready-rows.md's drawers).
                                ink: !fact.known || drawer.listsKeys ? Theme.muted
                                    : fact.modelData.warns === true ? Theme.warning
                                    : fact.modelData.hue ? Theme[fact.modelData.hue] : Theme.text
                                font: drawer.faceOf(fact.modelData).font
                                // A right-to-left script's words start at the
                                // column too, as every other value's do,
                                // rather than at the plate's far edge.
                                horizontalAlignment: Text.AlignLeft
                            }

                            // Its figure, where it carries one, drawn from
                            // what it holds, never asked for: Meter's stroke
                            // for a share, as Context's is, or Graph's spark
                            // for a history, as Today's (VKCF3Z, 2026-11-07),
                            // as `figure: { shape, values, warn }`. It
                            // follows its value on the value's line, its
                            // least room counted in whether the drawer's facts
                            // stand beside their names (laying); where it
                            // still won't fit, it takes the line under the
                            // value rather than going (DrawerFigure.place).
                            // It's no line for the cursor, as a picture isn't.
                            readonly property var figure: !heading && known
                                ? modelData.figure ?? null : null
                            readonly property real figureFrom: {
                                const face = drawer.faceOf(modelData);
                                void face.font;
                                // A value and its figure are two runs of one
                                // thing, apart as Meter's figure and stroke.
                                return valueFrom + (known ? face.advanceWidth(
                                    drawer.unbroken(modelData.value)) : 0)
                                    + Theme.space.rung.apart;
                            }
                            // A spark keeps Graph's ten cells; a stroke,
                            // having no letters, takes what room the line
                            // leaves, as Meter's does, up to as wide, and
                            // down to three of them, short of which a share
                            // no longer reads.
                            readonly property var figureLaid: {
                                void figureInk.advanceWidth;
                                const usual = figureInk.advanceWidth;
                                return Figure.place(figure === null ? null : {
                                    shape: figure.shape,
                                    usual,
                                    least: drawer.figureLeast(modelData)
                                        - Theme.space.rung.apart
                                }, value.lineCount, width - figureFrom, valueWide);
                            }
                            readonly property bool figureShown: figureLaid.place !== "none"
                            readonly property bool figureBelow: figureLaid.place === "under"
                            readonly property real figureWide: figureLaid.wide
                            readonly property real figureX: figureBelow ? valueFrom : figureFrom
                            // The value's baseline, down from the fact's top.
                            readonly property real valueBase: value.y + value.baselineOffset
                            // The figure's: the value's, or a line under
                            // its last where it stands under it.
                            readonly property real figureBase: valueBase
                                + (figureBelow ? value.lineCount * drawer.pitch : 0)
                            readonly property real share: Math.max(0, Math.min(1,
                                Number(figure?.values?.[0] ?? 0) || 0))

                            Graph {
                                x: fact.figureX
                                y: fact.figureBase - baselineOffset
                                reading: fact.figureShown && fact.figure.shape === "spark"
                                    ? fact.figure : null
                                // In the values' ink, its newest a step
                                // brighter, as a selected row's is.
                                selected: true
                            }

                            // Meter's stroke: four hairlines, the edge's
                            // track under the share, centred on the capitals
                            // on whole pixels, the warning's where it warns.
                            Rectangle {
                                visible: fact.figureShown && fact.figure.shape === "stroke"
                                x: Math.round(fact.figureX)
                                width: Math.floor(fact.figureWide)
                                height: 4 * Theme.border
                                y: Math.round(fact.figureBase
                                    - figureCapital.tightBoundingRect.height / 2 - height / 2)
                                color: Theme.edge

                                Rectangle {
                                    width: parent.width * fact.share
                                    height: parent.height
                                    color: fact.figure?.warn === true ? Theme.warning : Theme.text
                                }
                            }
                        }
                    }
                }

                // Its words, plain unless the row made them and says
                // they're styled: another program's are never read as
                // markup. They keep the pitch, each line on the card's.
                Item {
                    id: wordsBox
                    width: parent.width
                    height: bodyText.lineCount * drawer.pitch
                    visible: drawer.body !== ""

                    PageText {
                        id: bodyText
                        sought: drawer.washing
                        width: parent.width
                        y: drawer.pitch / 2 + drawer.dropped - baselineOffset
                        plain: drawer.bodySaid
                        styled: drawer.shown?.styled === true
                        leading: drawer.pitch
                        ink: Theme.text
                        // Prose, such as a notice's, at Words, as its card
                        // reads it and the reader sets its text. The rest,
                        // lines another program wrote, at Figure.
                        readonly property var level: drawer.prose ? Theme.level.words
                            : Theme.level.figure
                        font.family: bodyText.level.family
                        font.pointSize: bodyText.level.pointSize
                        font.variableAxes: bodyText.level.variableAxes
                    }
                }

                // A picture in its own shape, as wide as the drawer at
                // most, its corners a part's, the raised colour until it's
                // drawn. One whose size isn't given is as wide as the
                // drawer, a screen's 16:10 until it's read, the shape most
                // it shows are, and then its own, so a wide picture isn't
                // drawn in a taller box's middle with bands above and below.
                Rectangle {
                    id: frame
                    readonly property real across: drawer.shown?.across > 0
                        ? drawer.shown.across : parent.width
                    readonly property real down: drawer.shown?.down > 0 ? drawer.shown.down
                        : drawn.status === Image.Ready && drawn.implicitWidth > 0
                            ? across * drawn.implicitHeight / drawn.implicitWidth
                            : across * 10 / 16
                    readonly property real shrunk: Math.min(1, parent.width / across)

                    visible: drawer.picture !== ""
                    width: Math.round(across * shrunk)
                    height: Math.round(down * shrunk)
                    radius: Theme.inner
                    color: drawn.status === Image.Ready ? "transparent" : Theme.raised

                    Image {
                        id: drawn
                        anchors.fill: parent
                        source: Pictures.source(drawer.picture)
                        asynchronous: true
                        fillMode: Image.PreserveAspectFit
                        // Its width alone, so it keeps its own shape and
                        // its height never waits on the one it gives.
                        sourceSize.width: 2 * width
                        layer.enabled: true
                        layer.effect: MultiEffect {
                            maskEnabled: true
                            maskSource: corners
                        }
                        onStatusChanged: {
                            if (status === Image.Ready)
                                Report.say(`showed ${drawer.title}`);
                        }
                    }

                    Rectangle {
                        id: corners
                        anchors.fill: parent
                        radius: Theme.inner
                        visible: false
                        layer.enabled: true
                    }
                }
            }
        }
    }
}
