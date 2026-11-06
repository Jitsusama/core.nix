// Draws a palette's row (Choice.qml) and its field (Prompt.qml) through
// Quickshell itself, in the theme's own face and sizes, at a panel's width,
// with words too long for them, and holds them to what design.md says of
// every surface: nothing wraps, overlaps or runs past its edge, and what
// doesn't fit ends in an ellipsis. A row's name and what it is both keep
// within the row and apart, however long either is, and a field's ghost or
// hint stops at the field's end. A middle folder in a path says all of
// itself it has room for beside the hint, giving way only to what's typed
// past it, and past
// the path's least what's typed scrolls, the ask and the last step whole.
// A field's path keeps its steps whole beside a long note, and a short note keeps to the field's right end
// however wide a room it holds. The launcher's note, with nothing typed,
// ends where a row's value ends and says all it has room for; typed, it
// stands before the field's keys. Words that fit are left whole. Selected,
// a row with both its name and what it is cut reads each on in turn, the
// name first, never both moving at once, and its name at rest ends in its
// ellipsis rather than cut through a letter. What the shell says in its voice,
// a quiet row's words and a field's hint, is drawn in the serif face
// on the baseline the mono's would take. The launcher's modal field says
// no key, its hints the foot's, its caret a bar a quarter of a
// letter wide on the find and none in normal, resting on the last letter,
// and a modal caret fades and then, with no key, holds steady. The letters
// a find matched in a row's name are washed beneath them, run by run, a
// selected row's on the ground a shade past the selection, and a cut name's
// only before its ellipsis. A list's gutter in normal numbers the rows in
// view, relative to the cursor row's absolute one in text, or all absolute,
// right-aligned on each row's baseline, from a handful of numbers however
// many rows there are, and none at all off normal. A row's mark, whatever
// its shape, stands at one optical size, its ink on the capitals' middle. It
// exits 0 once every case holds, and otherwise 1, saying which didn't.
import Quickshell
import QtQuick
import QtQuick.Shapes
import "Bands.js" as Bands

ShellRoot {
    id: root

    // Joel's panel, in logical pixels.
    readonly property int panel: 677
    readonly property string long: "Heavy turns in Walgit Push Policy: Protect the Main Branch"
    readonly property string voice: "Nothing needs you."
    readonly property string sentence: "$4,199 billed in the last 7 days against an allowance"
        + " of $15,000, which resets on Monday at midnight"
    readonly property string asked: "Say which issue sources to read"
    readonly property string day: "Wednesday 4 November 2026"

    FloatingWindow {
        visible: true
        implicitWidth: root.panel
        implicitHeight: 700
        color: "black"

        Column {
            width: root.panel

            Choice { id: bothLong; name: root.long; side: root.sentence; glyph: "$" }
            Choice { id: sideLong; name: "Weekly cap"; side: root.sentence; glyph: "$" }
            Choice { id: nameLong; name: root.long + root.long; side: "7.0× usual"; glyph: "$" }
            Choice { id: fits; name: "Today"; side: "$484 · usual"; glyph: "$" }
            Choice {
                id: taking
                name: root.long + root.long
                side: root.sentence
                glyph: "$"
                selected: true
            }
            // A window: its title is what's looked for, so it keeps its room
            // and its program and place are cut first.
            Choice {
                id: titled
                name: "\u03c0 - Launcher - agentic-harness.pi"
                side: "Kitty · workspace 2, 3rd column, 2nd down"
                glyph: "$"
            }
            // A side of words too long for its room, as Slack Theme's how-to
            // is: it gives way at a whole word, never through one (VKCF3Z,
            // 2026-11-05: "pick Aubergine, then Custom the\u2026").
            Choice {
                id: sideWords
                width: 480
                name: "Slack Theme"
                side: "pick Aubergine, then Custom theme, Import"
                glyph: "$"
            }
            // A whole side in a row as wide, whose right edge a cut one
            // ends at too, its ellipsis there (VKCF3Z, 2026-11-05).
            Choice {
                id: sideWhole
                width: 480
                name: "Slack"
                side: "program"
                glyph: "$"
            }
            // A noun: its kind at the right edge, and what else it says after
            // its name, in its own words.
            Choice {
                id: noun
                name: "zsh"
                kind: "window"
                side: "kitty · workspace 1, 1st column"
                count: "2"
                also: "term"
                glyph: "$"
            }
            // The margin's row for nothing on, as Margin.qml draws it inside
            // its 440 points: whimsy that ends in an ellipsis isn't whimsy.
            Choice {
                id: nothing
                width: 440 - 2 * Theme.inset
                quiet: true
                glyph: String.fromCodePoint(0xf0674)
                name: "Nothing's on or waiting."
            }
            // The same words said quietly in the mono and in the voice.
            // Rows whose names a find matched, as the launcher gives them:
            // "fire" twice, the same selected, a name cut to its room with
            // a run past the cut, and none.
            Choice {
                id: washedTwice
                name: "Firefox, firewall"
                side: "program"
                glyph: "$"
                finds: [[0, 4], [9, 13]]
            }
            Choice {
                id: washedSelected
                selected: true
                name: "Firefox, firewall"
                side: "program"
                glyph: "$"
                finds: [[0, 4], [9, 13]]
            }
            Choice {
                id: washedCut
                name: root.long + root.long
                side: "7.0\u00d7 usual"
                glyph: "$"
                finds: [[0, 5], [2 * root.long.length - 6, 2 * root.long.length]]
            }
            Choice { id: washedNone; name: "Firefox"; side: "program"; glyph: "$" }
            Choice { id: plainSaid; quiet: true; glyph: "$"; name: root.voice }
            Choice { id: voicedSaid; quiet: true; voiced: true; glyph: "$"; name: root.voice }
            Prompt { id: typedField; width: root.panel; text: root.voice }
            Prompt { id: voicedField; width: root.panel; hint: root.voice }
            // The launcher's head: its field the band from the frame to the
            // hairline over the rows, as tall as the rows' rhythm sets it.
            Prompt {
                id: headed
                width: root.panel
                height: Bands.above(4 * Theme.space.beat, nameCaps.tightBoundingRect.height,
                    headed.restingLift, Theme.border)
                text: root.voice
            }
            Prompt {
                id: ghosted
                width: root.panel
                text: "spend"
                ghost: " = Today: $484 · usual, ends near $1,457 ($800 to $2,216), billed"
            }
            Prompt { id: hinted; width: root.panel; hint: root.sentence }
            PromptFoot { id: footed; width: root.panel; leads: "Asked by sudo" }
            StatusLine {
                id: lined
                width: root.panel
                height: 48
                mark: "NOR"
                hints: [["\u21b5", "open"]]
            }
            PromptHead {
                id: asking
                width: root.panel
                asked: "Joel needs a password"
                askedOpens: "J"
                reason: "to install a program"
            }
            // Three steps in, beside a note longer than the line has room
            // for, as Machine's processors' cores say.
            Prompt {
                id: pathed
                width: root.panel
                steady: true
                chips: [{ text: "Machine" }, { text: "Processor" }, { text: "Cores" }]
                hint: "what to do?"
                aside: root.sentence
            }
            // A name typed beside a place, as the file picker's save name
            // beside where it saves.
            Prompt {
                id: named
                width: root.panel
                chips: [{ text: "Save the Notes" }, { text: "~/saving" }]
                text: "a.md"
            }
            // Three steps in, a folder in the middle whose whole path the
            // line has no room for, at rest; then the file picker's own
            // line, its file mark leading the path, typed in far past its
            // hint.
            Prompt {
                id: foldered
                width: root.panel
                steady: true
                chips: [{ text: "Open the Notes" },
                    { text: "~/gathering/more", yields: true }, { text: "Found" }]
                hint: "what to do?"
            }
            Prompt {
                id: folderTyped
                width: root.panel
                steady: true
                glyph: "/"
                lead: Theme.glyph.category
                chips: [foldered.chips[0], foldered.chips[1], { text: "Gathered" }]
                hint: foldered.hint
                text: "abcdefghijklmno"
            }
            // Three steps in with a long hint, which leaves a note no room:
            // the note gives way rather than lying over the hint.
            Prompt {
                id: crowded
                width: root.panel
                chips: [{ text: "agents" }, { text: "orchestrators" }, { text: "Lighthouse" }]
                hint: "say something to Lighthouse"
                aside: "listening"
            }
            // A short note in the room a long one took before it.
            Prompt {
                id: steadied
                width: root.panel
                steady: true
                asideWidest: root.panel / 3
                aside: "just now"
            }
            // The launcher's field as it opens a step, nothing typed, its
            // note wider than two fifths of the line but well within what
            // the path and the hint leave it, above a row saying its value
            // (Joel, 2026-11-04: the note "inset left from the right edge
            // of the launcher, it looks like shit").
            Prompt {
                id: opened
                width: root.panel
                steady: true
                modal: true
                glyphWidth: Theme.space.xxl
                chips: [{ text: "Now" }]
                hint: "which?"
            }
            Choice { id: valued; width: root.panel; name: "Date"; side: root.day; glyph: "$" }
            // Marks of every shape down one column: wide, square-ish, tall and
            // narrow, and one already the em's size.
            Row {
                id: marks
                Choice { width: 160; name: "Keyboard"; glyph: Theme.glyph.keyboard }
                Choice { width: 160; name: "People"; glyph: Theme.glyph.people }
                Choice { width: 160; name: "Bluetooth"; glyph: Theme.glyph.bluetooth }
                Choice { width: 160; name: "Things"; glyph: Theme.glyph.category }
            }
            // Each mark's outline, which a Column passes over unseen.
            Repeater {
                id: outlines
                model: Array.from(marks.children).map(row => row.glyph)
                Shape {
                    required property string modelData
                    visible: false
                    ShapePath {
                        PathText {
                            text: modelData
                            font.family: Theme.level.figure.family
                            font.pointSize: Theme.level.figure.pointSize * 8
                        }
                    }
                }
            }
            // A collection's count at its right, before its chord.
            Choice { id: countedChord; width: root.panel; name: "Agents"; side: "12"
                chord: [{ text: "Super" }, { text: "0" }]; glyph: "$" }
            // A modal line blinking quickly, which fades and then, with no
            // key, holds steady, as his kitty's cursor does.
            Prompt {
                id: blinker
                width: root.panel
                modal: true
                fade: 100
                stopsAfter: 600
            }
            // A list's gutter beside three rows in view, the cursor on the
            // second, as normal draws it, with a slot to spare.
            Gutter {
                id: gutter
                height: 60
                rows: [{ at: 0, base: 12 }, { at: 1, base: 30 }, { at: 2, base: 48 }]
                slots: 4
                current: 1
                shown: true
            }
            // The launcher's field as it opens, with letters typed.
            Prompt {
                id: typing
                width: root.panel
                steady: true
                modal: true
                glyphWidth: Theme.space.xxl
                chips: [{ text: "Now" }]
                hint: "which?"
            }
            // A foot's keys: a chord of two, a key and its word, and Return
            // in a place held wider than what it says.
            KeyHints {
                id: footHints
                keys: [[":", "commands"], ["Space ?", "keys"], ["\u21b5", "open"]]
                widths: [0, 0, 200]
            }
        }
    }

    // Every piece of text drawn under an item, with where it's drawn.
    function drawn(item, into) {
        for (const child of item.children) {
            if (child.visible && typeof child.text === "string" && child.text !== ""
                && child.truncated !== undefined) {
                // Words set to the right end start where they leave off.
                const lead = child.horizontalAlignment === Text.AlignRight
                    ? child.width - child.contentWidth : 0;
                const at = child.mapToItem(null, lead, 0);
                into.push({ text: child.text, truncated: child.truncated,
                    from: at.x, to: at.x + child.contentWidth });
            }
            drawn(child, into);
        }
        return into;
    }

    // Where the words are drawn under an item: their baseline, down from
    // the item's top, and how wide they are.
    function setting(item, words, under) {
        const top = under ?? item;
        for (const child of item.children) {
            if (child.visible && child.text === words && child.contentWidth !== undefined)
                return { base: child.mapToItem(top, 0, child.baselineOffset).y,
                    wide: child.contentWidth };
            const found = setting(child, words, top);
            if (found !== null)
                return found;
        }
        return null;
    }

    // The field a line types in.
    function inputOf(item) {
        for (const child of item.children) {
            if (child.cursorRectangle !== undefined)
                return child;
            const found = inputOf(child);
            if (found !== null)
                return found;
        }
        return null;
    }

    // The voice is in a face of its own, as wide as the serif draws it
    // rather than the mono, on the mono's baseline within a device pixel
    // at a scale of 2.
    function voicedLike(plain, voiced) {
        return plain !== null && voiced !== null && Math.abs(plain.base - voiced.base) <= 0.5
            && voiced.wide < plain.wide * 0.9;
    }

    function said(texts) {
        return texts.map(t => `"${t.text}" ${Math.round(t.from)} to ${Math.round(t.to)}`)
            .join(", ");
    }

    // The piece under an item that reads these words on (ReadsOn.qml).
    function readsOn(item, words) {
        for (const child of item.children) {
            if (child.over !== undefined && child.text === words)
                return child;
            const found = readsOn(child, words);
            if (found !== null)
                return found;
        }
        return null;
    }

    // A row's texts keep inside it and clear of each other.
    function apart(row, edge) {
        const texts = drawn(row, []).sort((a, b) => a.from - b.from);
        const start = row.mapToItem(null, 0, 0).x;
        const inside = texts.every(t => t.from >= start && t.to <= start + edge);
        const clear = texts.every((t, i) => i === 0 || texts[i - 1].to <= t.from);
        if (!inside || !clear)
            throw new Error(`overlapping or past the edge: ${said(texts)}`);
        return texts;
    }

    FontMetrics {
        id: mono
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
    }

    // A head's face, the Title, and what's typed, the Figure: their
    // capitals and small letters, which centre a field's line.
    // Title's ink, read at eight times its size for its fractions.
    TextMetrics {
        id: titleInk
        font.family: Theme.level.title.family
        font.pointSize: Theme.level.title.pointSize * 8
        font.variableAxes: Theme.level.title.variableAxes
        font.letterSpacing: Theme.level.title.letterSpacing * 8
    }
    TextMetrics {
        id: titleCaps
        text: "H"
        font.family: Theme.level.title.family
        font.pointSize: Theme.level.title.pointSize
        font.variableAxes: Theme.level.title.variableAxes
    }
    TextMetrics {
        id: titleSmall
        text: "x"
        font: titleCaps.font
    }
    TextMetrics {
        id: figureCaps
        text: "H"
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
        font.weight: Theme.level.figure.weight
    }
    TextMetrics {
        id: figureSmall
        text: "x"
        font: figureCaps.font
    }
    // Words, a prompt's head: its capitals and small letters.
    TextMetrics {
        id: wordsCaps
        text: "H"
        font.family: Theme.level.words.family
        font.pointSize: Theme.level.words.pointSize
        font.variableAxes: Theme.level.words.variableAxes
    }
    TextMetrics {
        id: wordsSmall
        text: "x"
        font: wordsCaps.font
    }
    // A row's name, whose capitals are centred in the row.
    TextMetrics {
        id: nameCaps
        text: "H"
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
    }
    TextMetrics {
        id: noteCaps
        text: "H"
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
    }
    TextMetrics {
        id: noteSmall
        text: "x"
        font: noteCaps.font
    }

    // What a row says at its right, in the face it's drawn in.
    FontMetrics {
        id: sideFace
        font.family: Theme.level.note.family
        font.italic: Theme.level.note.italic
        font.pointSize: Theme.level.note.pointSize
        font.weight: Theme.level.note.weight
        font.variableAxes: Theme.level.note.variableAxes
        font.features: Theme.level.note.features
    }

    // Places as a row's wash lays them, for a failure to show.
    function places(washed) {
        return JSON.stringify(washed, (key, value) =>
            typeof value === "number" ? Math.round(value * 10) / 10 : value);
    }

    function expect(what, holds, shown) {
        if (!holds)
            throw new Error(`${what}: ${shown}`);
    }

    // A mark's ink as drawn, to hold a row's marks to one size: its top as
    // Qt measures it, and its height from its outline, since Qt's measure
    // runs down to the baseline from a mark standing above it.
    TextMetrics {
        id: inkProbe
    }

    // Every mark stands at one optical size beside the type: its ink's area
    // the square of fifteen sixteenths of the em unless that would make it
    // taller than that or wider than its cell, and its middle on the
    // name's capitals'.
    function marksHold() {
        // Measured at eight times the size and read back down, since a
        // measure at the size drawn comes in whole pixels.
        const fine = 8;
        const size = Theme.level.figure.pointSize * 4 / 3 * 15 / 16;
        const cell = Theme.level.name.pitch;
        inkProbe.font.family = Theme.level.name.family;
        inkProbe.font.pointSize = Theme.level.name.pointSize * fine;
        inkProbe.text = "H";
        const caps = inkProbe.tightBoundingRect.height / fine;
        for (const [at, row] of Array.from(marks.children).entries()) {
            const mark = row.children.find(child => child.text === row.glyph);
            inkProbe.font.family = mark.font.family;
            inkProbe.font.pointSize = mark.font.pointSize * fine;
            inkProbe.text = mark.text;
            const box = inkProbe.tightBoundingRect;
            const drawnAt = mark.font.pointSize / Theme.level.figure.pointSize;
            const tall = outlines.itemAt(at).boundingRect.height / fine * drawnAt;
            const ink = { width: box.width / fine, height: Math.min(tall, box.height / fine),
                y: box.y / fine };
            const shown = `${row.name}: ${ink.width}x${ink.height} at ${ink.y}, size ${size}`;
            const sized = Math.abs(Math.sqrt(ink.width * ink.height) - size) <= 0.5
                || Math.abs(ink.height - size) <= 0.5 || Math.abs(ink.width - cell) <= 0.5;
            expect("a mark is the size's area, or as tall as the size or as wide as its cell",
                sized && ink.height <= size + 0.5 && ink.width <= cell + 0.5, shown);
            const middle = mark.y + mark.baselineOffset + ink.y + ink.height / 2;
            const capsMiddle = (row.height + row.capitals) / 2 - caps / 2;
            expect("a mark's ink stands on the capitals' middle",
                Math.abs(middle - capsMiddle) <= 0.5,
                `${shown}, middle ${middle} against ${capsMiddle}`);
        }
    }

    // A row with a chord says its count at its right before it, joined as
    // the status line joins its facts, a muted dot a mark from each, on the
    // name's baseline, and nothing after its name.
    function textItem(item, words) {
        for (const child of item.children) {
            if (child.visible && child.text === words)
                return child;
            const found = textItem(child, words);
            if (found)
                return found;
        }
        return null;
    }
    // A count's ink, which ends short of its advance by its last figure's
    // bearing, read at eight times the size for its fractions.
    TextMetrics {
        id: countProbe
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize * 8
        text: "12"
    }

    function countBeforeChord() {
        const texts = drawn(countedChord, []).sort((a, b) => a.from - b.from);
        const count = texts.find(t => t.text === "12");
        const inkTo = count === undefined ? NaN : count.from
            + (countProbe.tightBoundingRect.x + countProbe.tightBoundingRect.width) / 8;
        const keys = texts.filter(t => count !== undefined && t.from > count.from
            && t.text !== "\u00b7");
        const dot = textItem(countedChord, "\u00b7");
        countProbe.text = "\u00b7";
        const dotBox = countProbe.tightBoundingRect;
        const dotFrom = dot ? dot.mapToItem(null, 0, 0).x + dotBox.x / 8 : NaN;
        const dotTo = dotFrom + dotBox.width / 8;
        countProbe.text = "12";
        expect("a mark from the count to the dot's ink, and from it to the chord",
            count !== undefined && keys.length > 0
                && Math.abs(dotFrom - inkTo - Theme.space.rung.mark) <= 0.5
                && Math.abs(keys[0].from - dotTo - Theme.space.rung.mark) <= 0.5,
            `${said(texts)}, the count's ink to ${inkTo}, the dot's ${dotFrom} to ${dotTo}`);
        // A key after the first stands past its + by its padding, which
        // the words drawn leave out, so the chord ends where its last
        // key's item does.
        const last = textItem(countedChord, "0");
        const ends = last ? last.mapToItem(null, last.width, 0).x : NaN;
        expect("a count stands before the chord, at the row's right",
            count !== undefined && keys.length > 0 && keys.every(t => t.from >= count.to - 0.5)
                && Math.abs(ends - (root.panel - Theme.space.rung.inset)) <= 1,
            `${said(texts)}, the chord ending at ${ends}`);
        const name = setting(countedChord, "Agents");
        const figure = setting(countedChord, count.text);
        expect("on the name's baseline", Math.abs(name.base - figure.base) <= 0.5,
            `${name.base} against ${figure.base}`);
        expect("and nothing after its name",
            texts.filter(t => t.text.includes("12")).length === 1, said(texts));
    }

    // The gutter, as his Neovim's number, relativenumber and
    // cursorlineopt=number draw it, then absolute, then hidden.
    function gutterHolds() {
        const texts = () => gutter.said().map(number => number.text);
        expect("the gutter counts from the cursor row, which says its own number",
            JSON.stringify(texts()) === JSON.stringify(["1", "2", "1"]), texts());
        const inks = gutter.said().map(number => number.ink);
        expect("the cursor row's number is in text and the rest muted",
            JSON.stringify(inks) === JSON.stringify(["muted", "text", "muted"]), inks);
        const said = gutter.said();
        expect("each number is on its row's baseline",
            said.every((number, at) => Math.abs(number.base - gutter.rows[at].base) <= 0.5),
            JSON.stringify(said));
        expect("the numbers end together, right-aligned",
            said.every(number => Math.abs(number.ends - said[0].ends) <= 0.5),
            JSON.stringify(said));
        expect("its digits fit its room, and no more",
            Math.abs(gutter.widest("999") - said[0].ends) <= 0.5,
            `${gutter.widest("999")} against ${said[0].ends}`);
        // The numbers stand a tie from the text they number, as Vim's
        // number column stands a space from the text (VKCF3Z, 2026-11-05).
        expect("the numbers end a tie before the rows",
            Math.abs(gutter.width - said[0].ends - Theme.space.rung.tie) <= 0.5,
            `${gutter.width - said[0].ends} before the rows`);

        expect("it makes a number a slot, not a row", gutter.made === gutter.slots,
            `${gutter.made} for ${gutter.slots} slots`);
        gutter.relative = false;
        expect("absolute, each row says its own number",
            JSON.stringify(texts()) === JSON.stringify(["1", "2", "3"]), texts());
        gutter.shown = false;
        expect("with no rows to number it draws no number and makes none",
            texts().length === 0 && gutter.made === 0, `${texts()}, ${gutter.made} made`);
        // A gutter keeping its room with nothing in it was an empty gap
        // before the rows (Joel, 2026-11-05: "when not present it looks
        // like shit").
        expect("and keeps no room", gutter.width === 0, `${gutter.width}`);
        gutter.shown = true;
        // As wide as the most its list can number, held by the launcher.
        const three = gutter.width;
        gutter.digits = 2;
        expect("two digits take one digit's advance less than three",
            Math.abs(three - gutter.width - (gutter.widest("999") - gutter.widest("99"))) <= 0.5,
            `${three} then ${gutter.width}`);
        gutter.digits = 3;
    }

    function check() {
        try {
            const cut = texts => texts.filter(t => t.truncated).map(t => t.text);
            let texts = apart(bothLong, bothLong.width);
            expect("a long name and side both cut", cut(texts).length === 2, said(texts));
            texts = apart(sideLong, sideLong.width);
            expect("a long side cut, its name whole", cut(texts).length === 1
                && cut(texts)[0] === root.sentence, said(texts));
            texts = apart(nameLong, nameLong.width);
            expect("a long name cut, its side whole", cut(texts).length === 1
                && cut(texts)[0] === nameLong.name, said(texts));
            texts = apart(titled, titled.width);
            expect("a window's title kept whole, its program and place cut first",
                cut(texts).length === 1 && cut(texts)[0] === titled.side, said(texts));
            texts = apart(sideWords, sideWords.width);
            const sideCut = texts.find(t => t.text === sideWords.side);
            const drawnWide = sideCut === undefined ? 0 : sideCut.to - sideCut.from;
            const words = sideWords.side.split(" ");
            const whole = words.slice(1).map((_, count) => `${words.slice(0, count + 1)
                .join(" ").replace(/[,;:\u00b7]+$/, "")}\u2026`);
            expect("a side too long gives way at a whole word, not through one",
                sideCut !== undefined && sideCut.truncated
                    && whole.some(cut => Math.abs(sideFace.advanceWidth(cut) - drawnWide) < 1),
                `${drawnWide} wide, whole cuts ${JSON.stringify(whole.map(cut =>
                    [cut, Math.round(sideFace.advanceWidth(cut))]))}: ${said(texts)}`);
            const wholeSide = apart(sideWhole, sideWhole.width)
                .find(t => t.text === sideWhole.side);
            expect("and it ends at the right edge every side ends at",
                sideCut !== undefined && wholeSide !== undefined
                    && Math.abs(sideCut.to - wholeSide.to) < 1,
                `cut ends at ${sideCut?.to}, a whole side at ${wholeSide?.to}`);
            texts = apart(fits, fits.width);
            expect("words that fit left whole", cut(texts).length === 0, said(texts));
            texts = apart(noun, noun.width);
            expect("a noun says its words after its name and its kind at the edge",
                texts.map(t => t.text).join(" | ")
                    === "$ | zsh | \u201cterm\u201d · 2 · kitty · workspace 1, 1st column | window",
                said(texts));
            // The mono's letters are each a letter wide, so where a run
            // starts and how wide it is are counted in letters.
            const em = mono.advanceWidth("f");
            const near = (got, want) => Math.abs(got - want) < 0.5;
            const twice = washedTwice.washed;
            expect("a find's letters are washed run by run, under them", twice.length === 2
                && near(twice[0][0], 0) && near(twice[0][1], 4 * em)
                && near(twice[1][0], 9 * em) && near(twice[1][1], 4 * em),
                `${root.places(twice)}, a letter ${em} wide: ${said(drawn(washedTwice, []))}`);
            apart(washedTwice, washedTwice.width);
            expect("a row's find takes Search's ground",
                twice.every(place => place[2] === Theme.wash.ground), root.places(twice));
            const onSelection = washedSelected.washed;
            expect("a selected row's find takes the ground a shade past the selection",
                onSelection.length === 2
                    && onSelection.every(place => place[2] === Theme.wash.selected),
                root.places(onSelection));
            texts = apart(washedCut, washedCut.width);
            const cutName = texts.find(t => t.truncated);
            const room = cutName === undefined ? 0
                : cutName.to - cutName.from - mono.advanceWidth("\u2026");
            const cutWash = washedCut.washed;
            expect("a cut name is washed only where its letters show, before its ellipsis",
                cutName !== undefined && cutWash.length === 1
                    && cutWash.every(([start, wide]) => start + wide <= room + 0.5),
                `${root.places(cutWash)} within ${room}: ${said(texts)}`);
            expect("a row with no find washes nothing", washedNone.washed.length === 0,
                root.places(washedNone.washed));
            texts = apart(nothing, nothing.width);
            expect("the margin's words for nothing on whole", cut(texts).length === 0,
                said(texts));
            for (const field of [ghosted, hinted]) {
                const end = field.mapToItem(null, field.end, 0).x;
                texts = drawn(field, []);
                // A ghost is drawn with Tab's key after it, so it's found by
                // how it starts.
                const after = texts.find(t => t.text.startsWith(field.ghost || field.hint));
                expect("a field's ghost or hint ends at its end", after !== undefined
                    && after.truncated && after.to <= end, `${said(texts)}, end ${end}`);
            }
            texts = drawn(pathed, []);
            expect("a field's path keeps its steps whole beside a long note",
                ["Machine", "Processor", "Cores"].every(step =>
                    texts.some(t => t.text === step && !t.truncated)), said(texts));
            // A middle folder gives way only as far as it must: at rest, the
            // room kept to type is the hint's as drawn, so it keeps all of
            // its last folders the line has room for, where a dozen
            // characters kept took one;
            // typed past the hint, it gives way to what's typed, and past
            // its least what's typed scrolls, the ask and the last step
            // whole, its caret in sight (VKCF3Z, 2026-11-06).
            texts = drawn(foldered, []);
            expect("a middle folder says all it has room for beside the hint at rest",
                ["Open the Notes", "~/\u2026/more", "Found", "what to do?"].every(step =>
                    texts.some(t => t.text === step && !t.truncated)), said(texts));
            texts = drawn(folderTyped, []);
            const typedIn = root.inputOf(folderTyped);
            // Where the caret ends, in the line, which is never past its end.
            const caret = typedIn === null ? Infinity : typedIn.mapToItem(folderTyped,
                typedIn.cursorRectangle.x, 0).x + folderTyped.caretWidth;
            expect("what's typed scrolls past the path's least, the ask whole",
                ["Open the Notes", "\u2026", "Gathered"].every(step =>
                    texts.some(t => t.text === step && !t.truncated))
                    && typedIn !== null && typedIn.cursorPosition === folderTyped.text.length
                    && caret <= folderTyped.end + 0.5,
                `${said(texts)}, caret ends at ${caret} of ${folderTyped.end}`);
            // Scrolled, a muted mark at the field's start says letters went
            // off it, in room taken from what's typed; at rest there's none.
            const precedes = typedIn === null ? undefined
                : Array.from(typedIn.children).find(child => child.text === "\u2026");
            expect("a field scrolled says so at its start, in its own room",
                folderTyped.scrolled && !foldered.scrolled && precedes !== undefined
                    && precedes.visible && Qt.colorEqual(precedes.color, Theme.muted)
                    && precedes.x + precedes.contentWidth <= typedIn.leftPadding + 0.5
                    && typedIn.leftPadding > 0,
                precedes === undefined ? "no mark"
                    : `mark ${precedes.x} to ${precedes.x + precedes.contentWidth}, `
                        + `typed from ${typedIn.leftPadding}`);
            // A path's mark stands a rung apart from the ink either side, to
            // its own, so a step reads further off than a word does (the
            // picker audit's finding 13).
            // Its ink by its first and last letters' bearings inside the
            // words as drawn, since a face's measure of whole words leaves
            // out the title's axes.
            const inkOf = text => {
                const letters = Array.from(text.text);
                titleInk.text = letters[0];
                const leads = titleInk.tightBoundingRect.x / 8;
                titleInk.text = letters[letters.length - 1];
                const box = titleInk.tightBoundingRect;
                const trails = (titleInk.advanceWidth - box.x - box.width) / 8;
                return { from: text.from + leads, to: text.to - trails };
            };
            texts = drawn(named, []).sort((a, b) => a.from - b.from);
            const steps = ["Save the Notes", "\u203a", "~/saving"]
                .map(words => texts.find(t => t.text === words));
            const inks = steps.every(step => step !== undefined) ? steps.map(inkOf) : [];
            expect("a path's mark stands a rung apart from the steps' ink either side",
                inks.length === 3
                    && Math.abs(inks[1].from - inks[0].to - Theme.space.rung.apart) <= 0.5
                    && Math.abs(inks[2].from - inks[1].to - Theme.space.rung.apart) <= 0.5,
                `${said(texts)}, inks ${JSON.stringify(inks)}`);
            // A note beside a head sits on the head's baseline, as his line
            // reads (Joel, 2026-11-05: "which? &etc. need to be baselined
            // with the text to their left"): the hint beside the place's
            // name, and what's typed in its place, within a device pixel at a
            // scale of 2.
            const onTheHead = (field, head, words) => {
                const placed = root.setting(field, head);
                const beside = words.map(said => root.setting(field, said));
                expect(`a note beside the head ${head} sits on its baseline`, placed !== null
                    && beside.every(one => one !== null && Math.abs(one.base - placed.base) <= 0.5),
                    `${head} ${JSON.stringify(placed)} against ${JSON.stringify(beside)}`);
            };
            onTheHead(opened, "Now", ["which?"]);
            onTheHead(pathed, "Cores", ["what to do?"]);
            onTheHead(named, "~/saving", ["a.md"]);
            // Each line is centred by eye in its field, its largest level's
            // capitals and small letters averaged on the field's middle
            // (docs/design.md: text in a band centres in it; Joel, 2026-11-05:
            // "the launcher find box text is not vertically aligned within its
            // box, it is more heavily top aligned"): what's typed out of a place, and the
            // place's name in one, each measured where it's drawn.
            const centred = (field, words, caps, small) => {
                const set = root.setting(field, words);
                const middle = set === null ? NaN
                    : set.base - (caps.tightBoundingRect.height
                        + small.tightBoundingRect.height) / 4;
                expect(`${words}'s line is centred by eye in its band`,
                    Math.abs(middle - field.height / 2) <= 0.5,
                    `ink's middle ${middle} in a band ${field.height} tall: ${JSON.stringify(set)}`);
            };
            centred(typedField, root.voice, figureCaps, figureSmall);
            centred(opened, "Now", titleCaps, titleSmall);
            centred(named, "~/saving", titleCaps, titleSmall);
            // The launcher's head is centred so, and its baseline stands a
            // row over the first row's, whose capitals the row centres,
            // under the hairline (docs/design.md: a band's height comes
            // from the rows' rhythm).
            centred(headed, root.voice, figureCaps, figureSmall);
            const headSet = root.setting(headed, root.voice);
            const firstRow = headed.height + Theme.border + (4 * Theme.space.beat
                + nameCaps.tightBoundingRect.height) / 2;
            expect("the head's baseline stands a row over the first row's",
                headSet !== null && Math.abs(firstRow - headSet.base - 4 * Theme.space.beat) <= 0.5,
                `head ${JSON.stringify(headSet)}, first row ${firstRow}, band ${headed.height}`);
            // A prompt's foot is a band, its hairline to the frame, its
            // baseline on the rows' rhythm and its bottom where its words
            // centre.
            centred(footed, "Asked by sudo", noteCaps, noteSmall);
            // A card's status line is one too, its mark and its hints on
            // the one baseline.
            centred(lined, "NOR", noteCaps, noteSmall);
            const lineMark = root.setting(lined, "NOR");
            const lineHint = root.setting(lined, "open");
            expect("a status line's hints sit on its mark's baseline", lineMark !== null
                && lineHint !== null && Math.abs(lineMark.base - lineHint.base) <= 0.5,
                `${JSON.stringify(lineMark)} against ${JSON.stringify(lineHint)}`);
            // A prompt's head is a band with no rows under it, its frame
            // over it and a hairline under it: its first capital stands the
            // inset under the frame, and the hairline falls where the lines
            // centre, their capitals' middle and their small letters'
            // averaged on the band's (VKCF3Z, 2026-11-05).
            const caps = wordsCaps.tightBoundingRect.height;
            const small = wordsSmall.tightBoundingRect.height;
            const lastLine = root.setting(asking, "to install a program");
            const lines = lastLine === null ? NaN
                : (2 * asking.inset + caps - small) / 4 + lastLine.base / 2;
            expect("a prompt's head centres its lines under the inset",
                Math.abs(lines - asking.height / 2) <= 0.5,
                `lines' middle ${lines} in a head ${asking.height} tall: ${JSON.stringify(lastLine)}`);
            texts = drawn(crowded, []).sort((a, b) => a.from - b.from);
            expect("a field's note gives way to its hint rather than lying over it",
                texts.every((t, i) => i === 0 || texts[i - 1].to <= t.from), said(texts));
            const plainRow = root.setting(plainSaid, root.voice);
            const voicedRow = root.setting(voicedSaid, root.voice);
            expect("a row's words in the voice, on the mono's baseline",
                root.voicedLike(plainRow, voicedRow),
                `${JSON.stringify(plainRow)} against ${JSON.stringify(voicedRow)}`);
            const typed = root.setting(typedField, root.voice);
            const hint = root.setting(voicedField, root.voice);
            expect("the launcher's hint in the voice, on the typed words' baseline",
                root.voicedLike(typed, hint), `${JSON.stringify(typed)} against ${JSON.stringify(hint)}`);
            // Where a caret is drawn, the hint starts a caret and an s past
            // where letters would, so the caret never cuts its first letter
            // (VKCF3Z's space audit, the picker's fault 11: "open which
            // file?" read with its o cut).
            const line = [...voicedField.children].find(child => child.cursorPosition !== undefined);
            const letters = line.mapToItem(null, 0, 0).x;
            const hinting = drawn(voicedField, []).find(t => t.text === root.voice);
            expect("a hint starts past the caret", hinting !== undefined
                && hinting.from >= letters + voicedField.caretWidth + Theme.space.rung.mark - 0.5,
                `hint at ${hinting?.from}, letters at ${letters}`);
            const right = steadied.mapToItem(null, steadied.width - Theme.space.rung.inset, 0).x;
            const note = drawn(steadied, []).find(t => t.text === "just now");
            expect("a short note keeps to the field's right end", note !== undefined
                && Math.abs(note.to - right) <= 1, `${said(drawn(steadied, []))}, end ${right}`);
            const asked = drawn(opened, []).find(t => t.text.startsWith("Say"));
            const value = drawn(valued, []).find(t => t.text === root.day);
            expect("the launcher's note, nothing typed, ends where a row's value ends",
                asked !== undefined && value !== undefined && Math.abs(asked.to - value.to) <= 1,
                `${said(drawn(opened, []))} over ${said(drawn(valued, []))}`);
            expect("the launcher's note says all it has room for before it gives way",
                asked.text === root.asked && !asked.truncated, said(drawn(opened, [])));
            texts = drawn(typing, []).sort((a, b) => a.from - b.from);
            const counted = texts.findIndex(t => t.text === "1 of 2");
            // A modal line holds only the find and how far it narrows: its
            // hints, Ctrl+Y's completing among them, are the foot's (VKCF3Z,
            // 2026-11-05).
            expect("typed on the find, the launcher's note ends the field, no key after it",
                counted >= 0 && counted === texts.length - 1
                    && texts.every((t, i) => i === 0 || texts[i - 1].to <= t.from), said(texts));
            // Each gap in the keys is its rung (docs/design.md, The Ladder):
            // a chord's keys tie, a key stands a mark from its word, a hint
            // an item from the next, and a place held wider keeps its room
            // after its word, never before its key, so "Space ? keys" reads
            // as a chord and its word, and Return as one hint among them.
            const hints = drawn(footHints, []).sort((a, b) => a.from - b.from);
            const gap = (from, to) => {
                const left = hints.find(t => t.text === from);
                const right = hints.find(t => t.text === to);
                return left && right ? right.from - left.to : NaN;
            };
            const rungs = [["Space", "?", Theme.space.rung.tie],
                ["?", "keys", Theme.space.rung.mark], [":", "commands", Theme.space.rung.mark],
                ["commands", "Space", Theme.space.rung.items],
                ["keys", "\u21b5", Theme.space.rung.items]];
            for (const [from, to, rung] of rungs)
                expect(`the keys' ${from} and ${to} stand ${rung} apart`,
                    Math.abs(gap(from, to) - rung) < 1, said(hints));
            expect("a key and its word are measured as they're drawn",
                Math.abs(footHints.advance("Space ?", "keys")
                    - (hints.find(t => t.text === "keys").to
                        - hints.find(t => t.text === "Space").from)) < 1, said(hints));
            // Insert's caret is a bar a quarter of a letter wide, as his
            // Neovim's guicursor draws it. In normal the keys are on the
            // rows, so the field draws no caret at all: a block there sat
            // on the hint's first letter, in the hint's face (Joel,
            // 2026-11-04: "a text box cursor on a non fixed-width font in
            // command mode").
            const letter = typing.letterWidth;
            expect("insert's caret is a bar a quarter of a letter wide",
                typing.caretWidth === Math.max(1, Math.round(letter / 4)),
                `${typing.caretWidth} for a letter ${letter} wide`);
            expect("insert draws its caret", typing.caretShown === true,
                `caretShown ${typing.caretShown}`);
            typing.mode = "normal";
            expect("normal draws no caret in the field", typing.caretShown === false,
                `caretShown ${typing.caretShown}`);
            expect("normal's caret rests on the last letter", typing.caret === 1,
                `at ${typing.caret} of "${typing.text}"`);
            texts = drawn(typing, []).sort((a, b) => a.from - b.from);
            // Normal's h steps out rather than clearing, so the field names
            // no key of its own there; the foot says normal's.
            expect("typed in normal, the launcher's field names no key",
                texts.length > 0 && !texts.some(t => t.text.includes("\u2190")
                    || t.text.includes("Ctrl+Y")), said(texts));
            // A find narrowing the rows says so in normal, its count at
            // Note after what's typed, as "/ne · 3 of 12"; on the find the
            // rows themselves narrow as it's typed, so it says nothing.
            typing.narrowed = "3 of 12";
            texts = drawn(typing, []).sort((a, b) => a.from - b.from);
            // What's typed is the line's own input, so the count is held to
            // standing past the path and its letters, well before the aside.
            const path = texts.find(t => t.text === "Now");
            const narrowing = texts.find(t => t.text === "\u00b7 3 of 12");
            const aside = texts.find(t => t.text === "1 of 2");
            expect("a narrowed find says its count after what's typed, in normal",
                path !== undefined && narrowing !== undefined && aside !== undefined
                    && narrowing.from >= path.to + typing.letterWidth * 2
                    && narrowing.to < aside.from,
                said(texts));
            typing.mode = "find";
            texts = drawn(typing, []);
            expect("and nothing of it on the find",
                !texts.some(t => t.text.includes("3 of 12")), said(texts));
            typing.narrowed = "";
            root.gutterHolds();
            root.marksHold();
            root.countBeforeChord();
            turns.begin();
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }

    // What the launcher's fields say comes once they're made, as the
    // launcher's does as Joel opens a step and types.
    Timer {
        interval: 100
        running: true
        onTriggered: {
            opened.aside = root.asked;
            typing.text = "ne";
            typing.ghost = "eds You";
            typing.aside = "1 of 2";
        }
    }

    Timer {
        interval: 500
        running: true
        onTriggered: root.check()
    }

    // Watches the blinking line's caret fade, then hold steady once it's
    // been left long enough.
    property bool dimmed: false
    property bool blinkHeld: false
    Timer {
        interval: 20
        repeat: true
        running: !root.blinkHeld
        property int left: 2000 / interval
        onTriggered: {
            root.dimmed = root.dimmed || blinker.caretLight < 0.9;
            left -= 1;
            if (root.dimmed && !blinker.blinkingNow && blinker.caretLight === 1)
                root.blinkHeld = true;
            else if (left === 0) {
                console.warn(`FAIL the caret didn't fade and then hold steady: dimmed `
                    + `${root.dimmed}, blinking ${blinker.blinkingNow}, lit ${blinker.caretLight}`);
                Qt.callLater(() => Qt.exit(1));
                root.blinkHeld = true;
            }
        }
    }

    // Watches the selected row with both cut, a little faster than a
    // frame's worth of rest, until its name has read on and then what it
    // is has, and fails if both are off their start at once or either
    // never moves.
    Timer {
        id: turns
        interval: 50
        repeat: true
        property var rest: null
        property bool named: false
        property bool sided: false
        property int left: 30000 / interval

        function where() {
            const texts = root.drawn(taking, []);
            const name = texts.find(t => t.text === taking.name);
            const side = texts.find(t => t.text === taking.side);
            return { name: name.from, side: side.from, ended: name.truncated,
                resting: root.readsOn(taking, taking.name)?.moving !== true };
        }
        function begin() {
            turns.rest = turns.where();
            turns.start();
        }
        onTriggered: {
            const now = turns.where();
            const nameMoved = now.name < turns.rest.name - 1;
            const sideMoved = now.side < turns.rest.side - 1;
            turns.left -= 1;
            let failed = "";
            if (nameMoved && sideMoved)
                failed = "a row's name and what it is both moved at once";
            else if (now.resting && !now.ended)
                failed = "a selected name at rest cut through a letter, not in its ellipsis";
            else if (sideMoved && !turns.named)
                failed = "what a row is read on before its name";
            else if (turns.left === 0)
                failed = `a cut row didn't read on in turn: name ${turns.named}, side ${sideMoved}`;
            turns.named = turns.named || nameMoved;
            turns.sided = turns.sided || sideMoved;
            if (failed !== "") {
                const from = JSON.stringify(turns.rest);
                console.warn(`FAIL ${failed}: ${JSON.stringify(now)} from ${from}`);
                turns.stop();
                Qt.callLater(() => Qt.exit(1));
            } else if (turns.named && turns.sided && root.blinkHeld) {
                turns.stop();
                Qt.callLater(() => Qt.exit(0));
            }
        }
    }
}
