// The one field every part of the shell asks through: the launcher's command
// line, the lock screen's password and polkit's. A glyph leads it and says
// what it's for, since a line you act on is the one place a glyph starts a
// line (docs/design.md, Shape and Kinds). A secret shows as dots right beside
// the padlock. A ghost after what's typed offers how it could end, or, with
// nothing typed, a hint. An aside on the right, muted, says something about
// what's typed, such as where the launcher's selection is among its matches.
// Where the field's own keys do more than type (the launcher's Ctrl+Y and
// normal's left arrow), they're said at its right edge, each a key and its word, as the
// foot says its keys.
// A secret's aside says when caps lock is on, its glyph in the warning
// colour, since a password typed in capitals fails with no reason given.
// Chips before what's typed hold what's being acted on: a thing, then a verb
// waiting for its target. They share what the line has once what's typed
// has room for its hint: a short one keeps its width, the others split what
// remains, and one too long for its share ends in an ellipsis.
// The launcher's line is modal, as Neovim's command line is: it opens in
// insert, where letters type, and Escape gives normal, where nothing types
// and the keys act (QueryKeys.js holds which do what). The caret says which:
// a bar a quarter of a letter wide in insert and a block on a letter in
// normal, as his Neovim's guicursor draws them, blinking as his kitty
// blinks them. In normal, / and ? take a search on the line, as his
// command line does: it's typed in place of what the field holds, which
// it puts back as it ends, so what's typed and the rows it found never
// change under a search.
// Its type ranks it (docs/design.md, Type): where Joel is at Title, the
// largest thing on the line, its capitals centred on the field; what's
// typed at Figure, his letters rather than a name; the hint in the voice;
// and the aside and the field's keys at Note, a step under the rest.
pragma ComponentBehavior: Bound
import QtQuick
import "Reading.js" as Reading
import "QueryKeys.js" as QueryKeys

Item {
    id: prompt

    property string glyph
    property color hue: Theme.muted
    // Room between the glyph and what's typed: the launcher lines what's
    // typed up with its results' column, and a secret sits close by its lock.
    property int glyphWidth: glyphText.implicitWidth + Theme.space.m
    property bool secret: false
    // How what's typed could end, drawn after it in muted type.
    property string ghost
    // Said in the voice while nothing's typed, as the desktop speaking:
    // the launcher's question, or the lock's "try again".
    property string hint
    // Said in muted type at the right end of the line.
    property string aside
    readonly property bool capitals: secret && Lights.caps
    readonly property string said: capitals ? "caps lock" : aside
    // How wide an aside is at Note, a pixel to spare, so what's measured to
    // fit is never cut again as it's drawn.
    function measured(words) {
        return Math.ceil(noteFace.advanceWidth(words)) + 1;
    }
    // How wide the aside is whole, measured rather than read off what's
    // drawn, since what's drawn is cut to the room this gives it.
    readonly property real asideWhole: said === "" ? 0 : measured(said)
    onAsideWholeChanged: {
        if (steady)
            asideWidest = Math.max(asideWidest, Math.min(asideWhole, asideMost));
    }
    // What's being acted on, each drawn as a chip: { text }.
    property var chips: []
    // Whether the left arrow takes what's typed away whole before it goes
    // back out, as the launcher's levels want: what's typed there is a few
    // letters narrowing a list, never text worth walking through, so the
    // arrow is the one key for "not that", and a second press goes up.
    // Only the plain arrow does, the field saying so at its right edge, as
    // it says Tab's, and the right arrow just after puts it back (Joel,
    // 2026-10-26: the left arrow clears only where it says so, and the right
    // arrow right after brings the text back). With Ctrl or Alt it moves the
    // caret, as in any field.
    property bool leftClears: false
    // What the left arrow just took away, which the right arrow puts back
    // until any other key is pressed or anything else is typed; in a modal
    // line it's normal's u that puts it back, as vi's undo does.
    property string cleared: ""

    // Whether the line has vi's modes, as the launcher's does; the lock's
    // and a password's prompts are only typed in.
    property bool modal: false
    // The mode it's in: "insert", "normal", or "search" while a search is
    // typed. It opens in insert (take()).
    property string mode: "insert"
    // The search being typed, { forward, count, pattern, at }, at being
    // the caret's place in the pattern, or null while none is.
    property var searching: null
    // How many keys the line has taken, a press each, so a word said for
    // one key, as a search's "not found", goes with the next.
    property int keysTaken: 0
    // The keys of a sequence begun in normal, as cc's first c, and a count
    // typed before a move.
    property var standing: QueryKeys.rest()
    // How long a sequence begun waits for its next key before it's let go,
    // as his Neovim's timeoutlen (modules/keys/language.toml).
    property int wait: 300
    onStandingChanged: {
        if (standing.typed.length > 0)
            waiting.restart();
        else
            waiting.stop();
    }

    Timer {
        id: waiting
        interval: prompt.wait
        onTriggered: prompt.standing = QueryKeys.waited(prompt.standing)
    }
    // Whether the caret blinks: while the line is on screen and modal.
    property bool awake: true
    // The field's keys' words, by what each does (launcher/words.json),
    // for a line that says them.
    property var keyWords: ({})
    // Says how far an aside may run, so a long one never crowds what's typed
    // or where Joel is: the path's steps and room to type come first, and
    // the aside has all they leave. With the aside first, a step into a
    // thing whose rows have long notes cut every step of the path to a
    // letter (Joel, 2026-10-25: "the breadcrumb quickly goes single letter
    // per entry"). Held to two fifths of the line besides, it gave way with
    // room to spare beside a short hint (Joel, 2026-11-04).
    // What's typed with its completion comes before the aside too, so
    // "weather tokyo" keeps "= no forecast" whole and the aside says less.
    property real asideMost: Math.max(0, width - 2 * Theme.space.l
        - glyphWidth - Theme.space.m - pathWanted - Math.max(typingRoom, typedWidest)
        - Theme.space.xl - keysRoom)
    // The widest what's typed has been with its completion since forget(),
    // so the aside gives way once as it grows, not back and forth with
    // each letter.
    property real typedWidest: 0
    readonly property real typedWanted: figureFace.advanceWidth(input.text + ghost)
    onTypedWantedChanged: {
        if (typedWanted > typedWidest)
            typedWidest = typedWanted;
    }
    // Whether the aside keeps the room its widest has taken until forget():
    // the launcher's says the selected row's note, which changes with each
    // arrow, and an aside sized to each one moved where what's typed had to
    // end and so every chip's share of the line.
    property bool steady: false
    property real asideWidest: 0
    property alias text: input.text

    // The aside's room: its own width, or, held steady, its widest since
    // forget(), so what's typed ends at the same place whatever it says. Its
    // words keep to the right end of that room, since a short note set at
    // the start of a long one's room sat far in from the edge (Joel,
    // 2026-10-25: "just now is inset an insane amount").
    readonly property real asideRoom: Math.min(prompt.asideMost, steady
        ? Math.max(asideWidest, asideWhole) : asideWhole)

    // Lets the aside's room go, as a new step starts, keeping what it says now.
    function forget() {
        crowded = false;
        typedWidest = typedWanted;
        asideWidest = Math.min(asideWhole, asideMost);
    }

    // Where what's typed has to end: before the aside's room, and before
    // caps lock's glyph when it shows, and before the field's keys.
    readonly property real end: asideRoom === 0 ? width - Theme.space.l - keysRoom
        : asideText.x - Theme.space.xl
            - (capitals ? Theme.space.s + capitalsGlyph.implicitWidth : 0)

    // The field's keys, said at its right edge (Joel, 2026-10-28: hints
    // "flush against the text feels wrong and they are not super obvious
    // what they mean"): what completes while a completion shows, and what
    // clears what's typed (keysSaid says which, in each kind of line). Each
    // has a place as wide as the widest it can
    // say, kept whether it says anything or not, so nothing in the field
    // moves as they come and go with each letter. They're set at the right
    // edge, past the aside, so neither moves as the aside changes.
    readonly property bool keyed: leftClears || modal
    // Whether their place is kept: only while one of them could be said,
    // something typed or something the left arrow took to put back. With
    // neither, no key shows until a letter is typed, so the aside ends at
    // the line's end, where the rows' sides end, rather than short of an
    // empty place (Joel, 2026-11-04: "the text in being inset left from the
    // right edge of the launcher, it looks like shit"). The first letter
    // changes what the aside says anyway, so it moves over then, once.
    // A modal line says no key for putting back what was cleared, since
    // normal's u is vi's own way back.
    readonly property bool keysHeld: keyed && (typed
        || (!modal && input.text === "" && cleared !== ""))
    // Their words take a third of the launcher's field, so they're said only
    // where everything else the field says leaves room for them: the path,
    // what's typed with its completion, and the aside, each whole. Where
    // they don't fit, the keys are said alone, since words for keys Joel
    // learns once never cost him the path or the answer to what he typed.
    // Once a step has run out of room they stay away until the step ends,
    // so they go at most once rather than coming and going with letters.
    // Each place's keys and words: in a modal line one place, insert's
    // completing or normal's clearing, as wide as the wider, so nothing
    // moves as the mode changes; otherwise Tab's place, then the left
    // arrow's and the right's.
    readonly property string completeKey: modal
        ? prompt.drawn(QueryKeys.keyOf("complete", "insert")) : "\u21e5"
    readonly property string clearKey: modal
        ? prompt.drawn(QueryKeys.keyOf("out", "normal")) : "\u2190"
    readonly property var keySlots: modal
        ? [[[completeKey, keyWords.complete ?? ""], [clearKey, keyWords.clear ?? ""]]]
        : [[[completeKey, keyWords.complete ?? ""]],
            [[clearKey, keyWords.clear ?? ""], ["\u2192", keyWords.restore ?? ""]]]
    function slotWidth(slot, worded) {
        return Math.max(...slot.map(([key, word]) =>
            noteFace.advanceWidth(worded && word !== "" ? `${key} ${word}` : key)));
    }
    readonly property var keysWorded: keySlots.map(slot => slotWidth(slot, true))
    readonly property real keysWordedWidth: keysWorded.reduce((sum, wide) => sum + wide, 0)
        + (keysWorded.length - 1) * fieldKeys.spacing
    readonly property real asideWanted: asideWhole
    readonly property bool wordsFit: width - 2 * Theme.space.l - glyphWidth - Theme.space.m
        - pathWanted - Math.max(typingRoom, typedWanted)
        - Theme.space.xl - (asideWanted > 0 ? asideWanted + Theme.space.xl : 0)
        >= keysWordedWidth + Theme.space.xl
    // Set as the words stop fitting, and let go by forget(), as a step ends.
    property bool crowded: false
    onWordsFitChanged: {
        if (!wordsFit && keysHeld)
            crowded = true;
    }
    readonly property bool worded: wordsFit && !crowded
    readonly property var keyPlaces: !keysHeld ? [] : worded ? keysWorded
        : keySlots.map(slot => slotWidth(slot, false))
    readonly property real keysRoom: keysHeld
        ? keyPlaces.reduce((sum, wide) => sum + wide, 0)
            + (keyPlaces.length - 1) * fieldKeys.spacing + Theme.space.xl : 0
    readonly property bool typed: input.text !== "" && !secret

    // A key's word, where there's room for the words.
    function keyWord(word: string): string {
        return worded ? word ?? "" : "";
    }

    // What the field says for its keys now: a modal line's one place says
    // insert's Ctrl+Y while a completion shows, or normal's left arrow while
    // something's typed; otherwise Tab while a completion shows, then the
    // left arrow while it clears what's typed or the right while it puts
    // back what the left took.
    readonly property var keysSaid: modal
        ? [mode === "insert" && typed && ghost !== ""
            ? [completeKey, keyWord(keyWords.complete)]
            : mode === "normal" && typed ? [clearKey, keyWord(keyWords.clear)] : ["", ""]]
        : [typed && ghost !== "" ? [completeKey, keyWord(keyWords.complete)] : ["", ""],
            typed ? [clearKey, keyWord(keyWords.clear)]
            : input.text === "" && cleared !== "" ? ["\u2192", keyWord(keyWords.restore)]
            : ["", ""]]

    // A key as the field draws it, from the table's name for it: an arrow
    // by its mark, and a letter with Ctrl as a chord is spelled.
    function drawn(key: string): string {
        const marks = { Left: "\u2190", Right: "\u2192", Up: "\u2191", Down: "\u2193",
            Tab: "\u21e5", Return: "\u21b5" };
        return key.split("+").map(part => marks[part] ?? (part.length === 1
            ? part.toUpperCase() : part)).join("+");
    }

    // Return, or Shift+Return, which asks for it below the focused window.
    signal accepted(bool below)
    // Tab, which takes the ghost.
    signal completed

    // Return pressed, said as accepted only for a press of its own, never
    // for a repeat.
    function returned(event: KeyEvent) {
        if (!event.isAutoRepeat)
            prompt.accepted((event.modifiers & Qt.ShiftModifier) !== 0);
    }

    signal cancelled
    // The right arrow past the end of what's typed, and the left arrow before
    // its start (or anywhere, with leftClears, once what's typed is gone) or
    // Backspace with nothing typed: into what's selected, and back out. In
    // a modal line, normal's arrows, l and h, and Backspace.
    signal forward
    signal backward
    // A modal line's keys for the rows below it, by their act's name, with
    // the count typed before it, 0 for none: the moves QueryKeys.moved
    // takes, drawerUp and drawerDown, and n and N's searchNext and
    // searchPrevious.
    signal stepped(string act, int count)
    // A search, as it's typed: "begun" as / or ? opens it, "typing" as
    // what it seeks changes, and "found" or "cancelled" as Return or
    // Escape ends it. Forward is /'s way, and the count is the one typed
    // before it, 0 for none.
    signal searched(string pattern, bool forward, int count, string how)

    function clear() {
        cleared = "";
        input.text = "";
    }

    // Takes the keys back from whatever else in its window had them, as it
    // opens, so nothing cleared before is offered back, and a modal line
    // opens in insert, so typing finds at once.
    function take() {
        cleared = "";
        standing = QueryKeys.rest();
        searching = null;
        mode = "insert";
        input.forceActiveFocus();
        woken();
    }

    // Into insert, as a line asking for words wants, the caret at its end.
    function typing() {
        standing = QueryKeys.rest();
        searching = null;
        if (mode !== "insert") {
            mode = "insert";
            input.cursorPosition = input.text.length;
        }
    }

    // Where the caret is, between letters in insert or on one in normal.
    readonly property int caret: input.cursorPosition

    // A key as the table names it (QueryKeys.js): a modifier, then the
    // letter it typed, or else its name, with Shift unnamed beside a letter
    // typed bare, which says it by its case.
    readonly property var keyNames: ({
        [Qt.Key_Return]: "Return",
        [Qt.Key_Enter]: "Enter",
        [Qt.Key_Backspace]: "Backspace",
        [Qt.Key_Escape]: "Escape",
        [Qt.Key_Left]: "Left",
        [Qt.Key_Up]: "Up",
        [Qt.Key_Right]: "Right",
        [Qt.Key_Down]: "Down",
        [Qt.Key_Home]: "Home",
        [Qt.Key_End]: "End",
        [Qt.Key_PageUp]: "PageUp",
        [Qt.Key_PageDown]: "PageDown",
        [Qt.Key_Tab]: "Tab",
        // Qt names Shift+Tab a key of its own.
        [Qt.Key_Backtab]: "Tab"
    })
    function named(event: KeyEvent): string {
        const control = (event.modifiers & Qt.ControlModifier) !== 0;
        const shift = (event.modifiers & Qt.ShiftModifier) !== 0;
        const bare = !control && /^[!-~]$/.test(event.text);
        // With Ctrl a letter types a control character, so it goes by its
        // key, which is the layout's letter.
        const letter = bare ? event.text
            : control && event.key >= Qt.Key_A && event.key <= Qt.Key_Z
                ? String.fromCharCode(event.key + 32) : "";
        const name = letter || (keyNames[event.key] ?? "");
        if (name === "")
            return "";
        return (control ? "Ctrl+" : "") + (shift && !bare ? "Shift+" : "") + name;
    }

    // A modal line's key: what the table says it does in this mode, done
    // here, or left to the field to type.
    // A key Qt gives a handler of its own, read from the table all the
    // same, and kept from the field whether the table has it or not, since
    // the field would only take it again in Keys.onPressed.
    function tabled(event: KeyEvent) {
        modalKey(event);
        event.accepted = true;
    }

    function modalKey(event: KeyEvent) {
        const key = named(event);
        woken();
        if (mode === "search") {
            searchKey(event, key);
            return;
        }
        const read = QueryKeys.read(standing, key, mode);
        // A key held in normal repeats only a move, as Vim's motions do:
        // any other act, and a count or a sequence begun, is a press's
        // once, so a key let go late by a busy shell, which repeats as if
        // held (modules/keyboard/repeat.nix), never does it twice.
        if (event.isAutoRepeat && mode === "normal" && !QueryKeys.repeats(read.act)) {
            event.accepted = true;
            return;
        }
        keysTaken++;
        standing = read.standing;
        if (key !== "" && read.act !== "restore" && read.act !== "out")
            cleared = "";
        if (!read.ours)
            return;
        event.accepted = true;
        const text = input.text;
        const at = input.cursorPosition;
        const next = QueryKeys.modeAfter(read.act, mode);
        if (next === "search") {
            input.deselect();
            mode = next;
            searching = { forward: read.act === "searchForward", count: read.count, pattern: "",
                at: 0 };
            prompt.searched("", searching.forward, read.count, "begun");
            return;
        }
        if (next !== mode) {
            input.deselect();
            if (read.act === "change")
                input.text = "";
            mode = next;
            input.cursorPosition = QueryKeys.caretAfter(read.act, input.text, at);
            return;
        }
        switch (read.act) {
        case "caretLeft":
            input.cursorPosition = Math.max(0, at - 1);
            break;
        case "caretRight":
            input.cursorPosition = Math.min(text.length, at + 1);
            break;
        case "caretStart":
            input.cursorPosition = 0;
            break;
        case "caretEnd":
            input.cursorPosition = text.length;
            break;
        case "deleteBack":
            if (input.selectionStart !== input.selectionEnd)
                input.remove(input.selectionStart, input.selectionEnd);
            else if (at > 0)
                input.remove(at - 1, at);
            break;
        case "deleteWord":
            input.remove(QueryKeys.wordStart(text, at), at);
            break;
        case "deleteToStart":
            input.remove(0, at);
            break;
        case "complete":
            prompt.completed();
            break;
        case "run":
            prompt.returned(event);
            break;
        case "into":
            prompt.forward();
            break;
        case "out":
            if (leftClears && text !== "") {
                input.text = "";
                cleared = text;
            } else {
                cleared = "";
                prompt.backward();
            }
            break;
        case "restore":
            if (text === "" && cleared !== "") {
                input.text = cleared;
                cleared = "";
            }
            break;
        case "back":
            prompt.cancelled();
            break;
        case "":
            break;
        default:
            prompt.stepped(read.act, read.count);
        }
    }

    // A key while a search is typed: the search's own, never the field's,
    // so what the field holds stands as it was. A key held repeats only as
    // it edits, and Return and Escape are a press's once, as normal's acts
    // are; the field's own guard already drops a held letter.
    function searchKey(event: KeyEvent, key: string) {
        event.accepted = true;
        const read = QueryKeys.read(standing, key, mode);
        if (event.isAutoRepeat && (read.act === "find" || read.act === "cancel"))
            return;
        keysTaken++;
        standing = read.standing;
        const line = QueryKeys.typedInSearch(searching, read.act,
            read.act === "" ? event.text : "");
        const { forward, count } = searching;
        if (line.ends !== "") {
            searching = null;
            mode = "normal";
            prompt.searched(line.pattern, forward, count, line.ends);
            return;
        }
        const changed = line.pattern !== searching.pattern;
        searching = { forward, count, pattern: line.pattern, at: line.at };
        if (changed)
            prompt.searched(line.pattern, forward, count, "typing");
    }

    // Normal's block rests on a letter, never past the last.
    function rest() {
        if (modal && mode === "normal") {
            const at = QueryKeys.resting(input.text, input.cursorPosition);
            if (at !== input.cursorPosition)
                input.cursorPosition = at;
        }
    }
    onModeChanged: rest()

    // The caret's blink, as his kitty blinks a cursor (modules/kitty/
    // kitty.conf): a key shows it whole, it fades out and back in over
    // `fade` each way, linearly, and `stopsAfter` with no key it holds
    // steady, so a line left still draws nothing more. A fade of 0 holds
    // it steady throughout.
    property int fade: 800
    property int stopsAfter: 15000
    readonly property bool blinks: modal && awake && fade > 0
    property real caretLight: 1
    function woken() {
        blinking.stop();
        caretLight = 1;
        if (blinks) {
            blinking.start();
            stilled.restart();
        }
    }
    onBlinksChanged: woken()
    Component.onCompleted: woken()
    // Whether it's blinking now, rather than holding steady.
    readonly property bool blinkingNow: blinking.running

    SequentialAnimation {
        id: blinking
        loops: Animation.Infinite
        NumberAnimation {
            target: prompt
            property: "caretLight"
            from: 1
            to: 0
            duration: prompt.fade
        }
        NumberAnimation {
            target: prompt
            property: "caretLight"
            from: 0
            to: 1
            duration: prompt.fade
        }
    }

    Timer {
        id: stilled
        interval: prompt.stopsAfter
        onTriggered: {
            blinking.stop();
            prompt.caretLight = 1;
        }
    }

    // How wide a letter is in the typed face, which normal's block covers
    // and a quarter of which insert's bar is, as guicursor's ver25 says.
    readonly property real letterWidth: typedFace.advanceWidth("M")
    readonly property real caretWidth: modal && mode === "normal" ? letterWidth
        : modal ? Math.max(1, Math.round(letterWidth / 4)) : 2

    FontMetrics {
        id: typedFace
        font: input.font
    }

    // A secret's dots stand a fifth of their size apart, so each reads as
    // one character. Points are 4/3 of a pixel at 96 dots an inch.
    readonly property real dotted: Theme.level.figure.pointSize * 4 / 3 * 0.2

    implicitWidth: 480
    implicitHeight: Theme.space.xxxl

    Text {
        id: glyphText
        x: Theme.space.l
        anchors.verticalCenter: parent.verticalCenter
        text: prompt.glyph
        color: prompt.hue
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
    }

    KeyHints {
        id: fieldKeys
        anchors.right: parent.right
        anchors.rightMargin: Theme.space.l
        anchors.baseline: input.baseline
        level: Theme.level.note
        visible: prompt.keysHeld
        widths: prompt.keyPlaces
        endsRight: true
        keys: prompt.keysSaid
    }

    Text {
        id: asideText
        anchors.right: prompt.keysHeld ? fieldKeys.left : parent.right
        anchors.rightMargin: prompt.keysHeld ? Theme.space.xl : Theme.space.l
        anchors.baseline: input.baseline
        width: prompt.asideRoom
        // Given no room, it isn't drawn: a text of no width isn't cut but
        // drawn whole, over the hint that took the room. A search being
        // typed has the line to itself, as his command line does, its room
        // kept so nothing moves as it ends.
        visible: prompt.asideRoom > 0 && prompt.searching === null
        horizontalAlignment: Text.AlignRight
        // Too long for its room, it gives way at a whole word (Reading.js);
        // the elision only guards a face that draws wider than measured.
        elide: Text.ElideRight
        text: Reading.wholeWords(prompt.said, prompt.asideRoom, words => prompt.measured(words))
        color: Theme.muted
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.features: { "tnum": 1 }
    }

    Text {
        id: capitalsGlyph
        anchors.right: asideText.left
        anchors.rightMargin: Theme.space.s
        anchors.baseline: input.baseline
        visible: prompt.capitals
        text: Theme.glyph.capitals
        color: Theme.warning
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
    }

    // What's typed, the aside and the path, each at its own level, to
    // measure what each takes of the line.
    FontMetrics {
        id: figureFace
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
    }
    FontMetrics {
        id: noteFace
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
    }
    FontMetrics {
        id: titleFace
        font.family: Theme.level.title.family
        font.pointSize: Theme.level.title.pointSize
        font.variableAxes: Theme.level.title.variableAxes
        font.letterSpacing: Theme.level.title.letterSpacing
    }
    // How tall the title's capitals stand, which are centred on the field.
    readonly property real titleCapitals: titleFace.tightBoundingRect("H").height

    // What stands between two steps of the path, so they read as one place
    // reached through the other rather than as tags side by side.
    readonly property string between: " \u203a "
    // How far what's typed stands after the path.
    readonly property int pathGap: Theme.space.l

    // The hint's face: the voice's, since it's the shell speaking to Joel.
    FontMetrics {
        id: voiceFace
        font.family: Theme.level.voice.family
        font.italic: Theme.level.voice.italic
        font.pointSize: Theme.level.voice.pointSize
        font.variableAxes: Theme.level.voice.variableAxes
    }

    // The room kept for typing: the hint's, or a dozen characters.
    readonly property real typingRoom: Math.max(
        voiceFace.advanceWidth(hint),
        12 * figureFace.averageCharacterWidth)

    // How wide the path wants to be, whole, with the gap after it.
    readonly property real pathWanted: chips.length === 0 ? 0
        : chipsWanted.reduce((sum, wide) => sum + wide, 0)
            + betweenDrawn.implicitWidth * (chips.length - 1) + pathGap - Theme.space.m

    // Each step's words' width, as Reading.sharedSteps shares the room: a
    // folder's path gives way first, down to four characters, cut in its
    // middle.
    readonly property var fitted: {
        const room = Math.max(0, end - Theme.space.l - glyphWidth - typingRoom
            - betweenDrawn.implicitWidth * Math.max(0, chips.length - 1) - pathGap);
        return Reading.sharedSteps(chipsWanted, chips.map(chip => chip.yields === true), room,
            4 * titleFace.averageCharacterWidth);
    }

    // Each step's words' width, whole, as a Text draws them: a face's
    // metrics leave out the title's axes, which change its letters'
    // widths, so measured by them a step came out a hair short and was cut.
    // Until a step's Text is made for its words, the face's measure stands.
    readonly property var chipsWanted: stepsMade >= 0 ? chips.map((chip, at) => {
        const drawn = stepsDrawn.itemAt(at);
        return Math.ceil(drawn && drawn.text === chip.text ? drawn.implicitWidth
            : titleFace.advanceWidth(chip.text)) + 1;
    }) : []
    // How many steps' Texts have been made, so each new one is measured.
    property int stepsMade: 0

    Repeater {
        id: stepsDrawn
        model: prompt.chips
        onItemAdded: prompt.stepsMade += 1

        Text {
            required property var modelData
            visible: false
            text: modelData.text
            font: titleFace.font
        }
    }
    Text {
        id: betweenDrawn
        visible: false
        text: prompt.between
        font: titleFace.font
    }
    // Whether each step is said whole, for the launcher's report.
    readonly property var chipsWhole: chipsWanted.map((width, index) =>
        (fitted[index] ?? 0) >= width)


    // Where Joel is, the place's name at Title in the text colour, with no
    // chip, since its size says it's where he is; the marks between a
    // path's steps muted. Its capitals are centred on the field, as an
    // eyebrow's are, and its line box may run past the field's rather than
    // growing it, so the card is one height in a place and out of it.
    Item {
        id: held
        x: Theme.space.l + prompt.glyphWidth
        visible: prompt.chips.length > 0
        width: visible ? path.width : 0
        height: parent.height

        Row {
            id: path
            y: (prompt.height + prompt.titleCapitals) / 2 - titleFace.ascent

            Repeater {
                model: prompt.chips

                Row {
                    id: step

                    required property var modelData
                    required property int index

                    Text {
                        visible: step.index > 0
                        text: prompt.between
                        color: Theme.muted
                        font: titleFace.font
                    }

                    Text {
                        width: prompt.fitted[step.index] ?? implicitWidth
                        text: step.modelData.text
                        // A path keeps both ends, where it starts and the
                        // folder it's at, cut in its middle.
                        elide: step.modelData.yields === true ? Text.ElideMiddle : Text.ElideRight
                        color: Theme.text
                        font: titleFace.font
                    }
                }
            }
        }
    }

    // A search being typed, where the field's words were: its / or ?,
    // then what it seeks, in the field's face and ink, so it takes no
    // size or weight of its own, and insert's caret, blinking, since
    // letters type there. Run past its room, it slides left to keep the
    // caret in view, as the field does.
    Item {
        id: searchLine
        x: input.x
        width: input.width
        height: input.height
        anchors.verticalCenter: parent.verticalCenter
        visible: prompt.searching !== null
        clip: true

        readonly property string said: prompt.searching === null ? ""
            : (prompt.searching.forward ? "/" : "?") + prompt.searching.pattern
        // Where the caret is: after the / or ? and the letters before it.
        readonly property real caretAt: prompt.searching === null ? 0
            : typedFace.advanceWidth(said.slice(0, prompt.searching.at + 1))
        readonly property real caretBar: Math.max(1, Math.round(prompt.letterWidth / 4))
        readonly property real slid: Math.max(0, caretAt + caretBar - width)

        Text {
            id: searchText
            x: -searchLine.slid
            y: input.baselineOffset - baselineOffset
            text: searchLine.said
            textFormat: Text.PlainText
            color: Theme.strong
            font: input.font
        }

        Rectangle {
            x: searchLine.caretAt - searchLine.slid
            width: searchLine.caretBar
            height: input.cursorRectangle.height
            y: input.cursorRectangle.y
            color: Theme.accent
            opacity: prompt.caretLight
        }
    }

    TextInput {
        id: input
        x: Theme.space.l + prompt.glyphWidth
            + (prompt.chips.length > 0 ? held.width + prompt.pathGap : 0)
        width: prompt.end - x
        anchors.verticalCenter: parent.verticalCenter
        focus: true
        clip: true
        // A search being typed draws in its place, and the field keeps the
        // keys, so it's hidden by its light alone: one not visible would
        // let its focus go.
        opacity: prompt.searching === null ? 1 : 0
        echoMode: prompt.secret ? TextInput.Password : TextInput.Normal
        passwordCharacter: "\u25cf"
        passwordMaskDelay: 0
        color: Theme.strong
        selectionColor: Theme.selection
        selectedTextColor: Theme.strong
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
        font.weight: Theme.level.figure.weight
        font.features: Theme.level.figure.features
        font.letterSpacing: prompt.secret ? prompt.dotted : Theme.level.figure.letterSpacing
        // Return does what it does once a press: let go late by a busy
        // shell, it repeats as a held key does (Keys.onPressed below), and
        // each repeat would do it again, a setting turned on and off.
        // Qt calls these before Keys.onPressed, so a modal line hands them
        // to its table here, or Escape would close the launcher from
        // insert and Tab would complete rather than move.
        Keys.onReturnPressed: event => prompt.modal ? prompt.tabled(event)
            : prompt.returned(event)
        Keys.onEnterPressed: event => prompt.modal ? prompt.tabled(event)
            : prompt.returned(event)
        Keys.onTabPressed: event => prompt.modal ? prompt.tabled(event) : prompt.completed()
        Keys.onEscapePressed: event => prompt.modal ? prompt.tabled(event) : prompt.cancelled()
        onTextChanged: {
            if (text !== "")
                prompt.cleared = "";
        }
        onCursorPositionChanged: prompt.rest()
        Keys.onPressed: event => {
            // A key whose release comes late, past the repeat delay, while
            // the shell is busy, repeats as if held
            // (modules/keyboard/repeat.nix). No field here wants a
            // character held for many, so only keys that move or delete
            // repeat. In a modal line's normal nothing types, and its table
            // says which keys repeat: the moves (modalKey).
            if (event.isAutoRepeat && event.text !== "" && event.text >= " "
                && event.text !== "\u007f" && (event.modifiers & Qt.ControlModifier) === 0
                && !(prompt.modal && prompt.mode === "normal")) {
                event.accepted = true;
                return;
            }
            if (prompt.modal) {
                prompt.modalKey(event);
                return;
            }
            const none = input.selectionStart === input.selectionEnd;
            const plainly = (event.modifiers & ~Qt.KeypadModifier) === Qt.NoModifier;
            const restoring = event.key === Qt.Key_Right && plainly && input.text === ""
                && prompt.cleared !== "";
            if (!restoring)
                prompt.cleared = "";
            if (restoring) {
                input.text = prompt.cleared;
                input.cursorPosition = input.text.length;
                event.accepted = true;
            } else if (event.key === Qt.Key_Right && none
                && input.cursorPosition === input.text.length) {
                prompt.forward();
                event.accepted = true;
            } else if (event.key === Qt.Key_Left && plainly && prompt.leftClears
                && input.text !== "") {
                const was = input.text;
                input.text = "";
                prompt.cleared = was;
                event.accepted = true;
            } else if ((event.key === Qt.Key_Left && none && input.cursorPosition === 0)
                || (event.key === Qt.Key_Backspace && input.text === "")) {
                prompt.backward();
                event.accepted = true;
            }
        }

        // The caret in the accent, since it's where the keys go. It takes
        // no room of its own, so a ghost reads on as one word. Qt leaves
        // blinking to a caret drawn as its own delegate, so this one holds
        // steady and a launcher left still draws nothing (the launcher
        // audit's finding 34, held by goes-places' frame count).
        // In a modal line its shape says the mode, and it blinks: normal's
        // block wears the letter it's on in the ground's colour, as a
        // terminal's cursor does, or the hint's first where nothing's typed.
        cursorDelegate: Rectangle {
            id: caretDrawn
            width: prompt.caretWidth
            color: Theme.accent
            opacity: prompt.caretLight
            visible: input.cursorVisible

            Text {
                readonly property bool typedOn: input.text !== ""
                visible: prompt.modal && prompt.mode === "normal"
                text: typedOn ? input.text.charAt(input.cursorPosition) : hintText.text.charAt(0)
                textFormat: Text.PlainText
                color: Theme.background
                font: typedOn ? input.font : hintText.font
                y: (typedOn ? input.baselineOffset : hintText.y + hintText.baselineOffset)
                    - caretDrawn.y - baselineOffset
            }
        }

        // A hint starts where the first letter typed will, on the names'
        // column, so a letter takes its place rather than landing beside it
        // (the launcher audit's finding 35, Joel's yes on 2026-10-26: it
        // stood a step right of it). A ghost runs on from what's typed as
        // one word, and ends in an ellipsis where the field does.
        // Once the left arrow has cleared what's typed, what it took stands
        // in its place as the ghost, which the right arrow puts back. The
        // keys that take them are said at the field's right edge.
        // The hint is the shell's voice, in every field, sized by capitals
        // and sat on the typed letters' baseline, since the two faces' line
        // boxes differ; the cleared words and the ghost are letters typed,
        // in the field's own face (look 3's piece 1).
        Text {
            id: hintText
            readonly property bool voiced: input.text === "" && prompt.cleared === ""
            readonly property var level: hintText.voiced ? Theme.level.voice : Theme.level.figure
            x: input.text === "" ? 0 : input.contentWidth
            width: Math.max(0, input.width - x)
            elide: Text.ElideRight
            anchors.baseline: parent.baseline
            textFormat: Text.PlainText
            text: input.text === "" ? prompt.cleared !== "" ? prompt.cleared : prompt.hint
                : prompt.ghost
            visible: text !== "" && (input.text === "" || !prompt.secret)
            color: Theme.muted
            font.family: hintText.level.family
            font.italic: hintText.level.italic
            font.pointSize: hintText.level.pointSize
            font.variableAxes: hintText.level.variableAxes
        }
    }
}
