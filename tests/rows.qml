// Draws a palette's row (Choice.qml) and its field (Prompt.qml) through
// Quickshell itself, in the theme's own face and sizes, at a panel's width,
// with words too long for them, and holds them to what design.md says of
// every surface: nothing wraps, overlaps or runs past its edge, and what
// doesn't fit ends in an ellipsis. A row's name and what it is both keep
// within the row and apart, however long either is, and a field's ghost or
// hint stops at the field's end. A field's path keeps its steps whole
// beside a long note, and a short note keeps to the field's right end
// however wide a room it holds. The launcher's note, with nothing typed,
// ends where a row's value ends and says all it has room for; typed, it
// stands before the field's keys. Words that fit are left whole. Selected,
// a row with both its name and what it is cut reads each on in turn, the
// name first, never both moving at once, and its name at rest ends in its
// ellipsis rather than cut through a letter. What the shell says in its voice,
// a quiet row's words and a field's hint, is drawn in the serif face
// on the baseline the mono's would take. The launcher's modal field says
// Ctrl+Y in insert and the left arrow in normal, its caret a bar a quarter
// of a letter wide in insert and a block a letter wide in normal, resting on
// the last letter, and a modal caret fades and then, with no key, holds
// steady. The letters a find matched in a row's name are washed beneath
// them, run by run, a selected row's on the ground a shade past the
// selection, and a cut name's only before its ellipsis. It exits 0 once
// every case holds,
// and otherwise 1, saying which didn't.
import Quickshell
import QtQuick

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
    readonly property var keyWords: ({ complete: "complete", clear: "clear" })

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
            Prompt {
                id: ghosted
                width: root.panel
                text: "spend"
                ghost: " = Today: $484 · usual, ends near $1,457 ($800 to $2,216), billed"
            }
            Prompt { id: hinted; width: root.panel; hint: root.sentence }
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
                leftClears: true
                modal: true
                keyWords: root.keyWords
                glyphWidth: Theme.space.xxl
                chips: [{ text: "Now" }]
                hint: "which?"
            }
            Choice { id: valued; width: root.panel; name: "Date"; side: root.day; glyph: "$" }
            // The same with letters typed, so the field says its keys.
            // A modal line blinking quickly, which fades and then, with no
            // key, holds steady, as his kitty's cursor does.
            Prompt {
                id: blinker
                width: root.panel
                modal: true
                fade: 100
                stopsAfter: 600
            }
            Prompt {
                id: typing
                width: root.panel
                steady: true
                leftClears: true
                modal: true
                keyWords: root.keyWords
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
            const right = steadied.mapToItem(null, steadied.width - Theme.space.l, 0).x;
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
            expect("typed, the launcher's note stands before the field's keys",
                counted >= 0 && counted < texts.length - 1
                    && texts.every((t, i) => i === 0 || texts[i - 1].to <= t.from), said(texts));
            expect("typed in insert, the launcher's field says Ctrl+Y completes",
                texts.length > 1 && texts[texts.length - 2].text === "Ctrl+Y", said(texts));
            // A key stands a mark from its word, nearer than one hint is
            // to the next, so the two read as one hint (docs/design.md,
            // The Ladder).
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
            expect("typed in normal, the launcher's field says the left arrow clears",
                texts.length > 0 && texts[texts.length - 1].text.includes("clear"), said(texts));
            typing.mode = "find";
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
