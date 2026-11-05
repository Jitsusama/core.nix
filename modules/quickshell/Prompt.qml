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
// a bar a quarter of a letter wide in insert, as his Neovim's guicursor
// draws it, blinking as his kitty blinks it, and none in normal, where the
// keys are on the rows.
// Its type ranks it (docs/design.md, Type): where Joel is at Title, the
// largest thing on the line, its capitals centred on the field; what's
// typed at Figure, his letters rather than a name; the hint in the voice;
// and the aside and the field's keys at Note, a step under the rest.
pragma ComponentBehavior: Bound
import QtQuick
import "Reading.js" as Reading
import "Editing.js" as Editing
import "KeyNames.js" as KeyNames
import "QueryKeys.js" as QueryKeys
import "Operators.js" as Operators
import "Typing.js" as Typing

Item {
    id: prompt

    property string glyph
    property color hue: Theme.muted
    // Room between the glyph and what's typed: the launcher lines what's
    // typed up with its results' column, and a secret sits close by its lock.
    property int glyphWidth: glyphText.implicitWidth + Theme.space.m
    // Where its mark stands from its left, so a frame holding it sets the
    // mark on its own column by this rather than by a number of its own.
    readonly property real markAt: glyphText.x
    // A glyph after the first, leading the path: a held thing's, where the
    // find's / leads the line.
    property string lead
    property color leadHue: Theme.muted
    readonly property real leadWidth: lead === "" ? 0 : leadText.implicitWidth + Theme.space.m
    // Where what follows the glyphs starts, from the line's own inset.
    readonly property real ledWidth: glyphWidth + leadWidth
    property bool secret: false
    // How what's typed could end, drawn after it in muted type.
    property string ghost
    // Just copied by y, as an answer's ghost is, for the key timing's
    // yank-flash.
    property bool ghostFlashed: false
    // Said in the voice while nothing's typed, as the desktop speaking:
    // the launcher's question, or the lock's "try again".
    property string hint
    // Said in muted type at the right end of the line.
    property string aside
    // How far a find narrows the rows, as "3 of 12", said after what's
    // typed in normal, so a list narrowed always says so; nothing on the
    // find, where the rows narrow as it's typed (the vi-paradigm spec).
    property string narrowed
    readonly property bool narrowSaid: modal && mode === "normal" && narrowed !== ""
        && input.text !== ""
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
    // The mode it's in: "find", the / line whose letters narrow the rows,
    // "insert" while it holds a prompt's words (`words`), "normal", "visual"
    // while visual line chooses rows, or "page" while the launcher's drawer holds the keys, gone into with
    // Space and the right arrow, as his Neovim goes to the window beside,
    // and "pageVisual" or "pageVisualLine" while choosing there. It opens
    // on the find (take()), or in insert where it opens on words.
    property string mode: "find"
    // The kind the drawer last chose by, for gv to choose by again, let go
    // once the keys leave it, as the drawer lets what it chose go.
    property string lastChosen: ""
    // Whether the line holds a prompt's words rather than a find, as a
    // save's name, a password or a code a row asks for: they're typed in
    // insert, normal edits them as Vim edits a line (QueryKeys.js, the
    // words' rows), and u takes back each change, the newest first, kept in
    // `changes` as the words and the caret before it.
    property bool words: false
    // Whether normal edits those words now: only while the cursor row is
    // the one they name, so on any other row h and l step out and in as
    // everywhere else.
    property bool editsWords: words
    property var changes: []
    // Visual's operator just done, its key and when, for the same pressed
    // again to be taken with it (Operators.takenWith); null after any key.
    property var visualOperated: null
    // Whether there's a selection for gv to bring back (Launcher.qml's
    // lastVisual).
    property bool reselectable: false
    // Whether the keys are on the rows, in normal and visual alike.
    readonly property bool overRows: modal && (mode === "normal" || mode === "visual")
    // Whether there's a drawer to go into: out, with lines to sit on. Gone
    // while it holds the keys, they come back to the rows.
    property bool drawerHere: false
    onDrawerHereChanged: {
        if (!drawerHere && QueryKeys.paged(mode))
            mode = "normal";
    }
    // The mode typing goes back to: the find, or insert on words.
    readonly property string typingMode: words ? "insert" : "find"
    // Words handed over or left go back to the find, never to an insert
    // with nothing asking, and words come on a find typed in insert.
    onWordsChanged: {
        changes = [];
        if (!words && mode === "insert")
            mode = "find";
        else if (words && mode === "find")
            mode = "insert";
    }
    // Whether letters type in the mode it's in.
    readonly property bool typingNow: mode === "find" || mode === "insert"
    // The keys of a sequence begun in normal, as cc's first c, and a count
    // typed before a move.
    property var standing: QueryKeys.rest()
    // Whether an ask stands (Launcher.qml's askOnce), which takes every
    // key as its answer, as Vim's ask_yesno does, until one answers it.
    property bool confirming: false
    signal answered(string key)
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
        - ledWidth - Theme.space.m - pathWanted - Math.max(typingRoom, typedWidest)
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
    // clears what's typed (keysSaid says which). Each has a place as wide
    // as the widest it can say, kept whether it says anything or not, so
    // nothing in the field moves as they come and go with each letter.
    // They're set at the right edge, past the aside, so neither moves as
    // the aside changes. A modal line says none: it holds only the find and
    // how far it narrows, its hints the foot's, by key and lowercase word
    // (VKCF3Z, 2026-11-05).
    readonly property bool keyed: leftClears && !modal
    // Whether their place is kept: only while one of them could be said,
    // something typed or something the left arrow took to put back. With
    // neither, no key shows until a letter is typed, so the aside ends at
    // the line's end, where the rows' sides end, rather than short of an
    // empty place (Joel, 2026-11-04: "the text in being inset left from the
    // right edge of the launcher, it looks like shit"). The first letter
    // changes what the aside says anyway, so it moves over then, once.
    // A question asked once takes no key of the field's, so none is said.
    // Nor do a prompt's words, which nothing clears or brings back. Where
    // even the keys alone would cut the path, they give way, since a key
    // Joel learns once never costs him where he is.
    readonly property bool keysHeld: keyed && asking === "" && !words && keysFit && (typed
        || (input.text === "" && cleared !== ""))
    readonly property real keysBare: keySlots.reduce((sum, slot) => sum + slotWidth(slot, false),
        0) + (keySlots.length - 1) * fieldKeys.spacing
    readonly property bool keysFit: chips.length === 0 || width - 2 * Theme.space.l - ledWidth
        - Theme.space.m - pathWanted - Math.max(typingRoom, typedWanted) - Theme.space.xl
        >= keysBare + Theme.space.xl
    // Their words take a third of the launcher's field, so they're said only
    // where everything else the field says leaves room for them: the path,
    // what's typed with its completion, and the aside, each whole. Where
    // they don't fit, the keys are said alone, since words for keys Joel
    // learns once never cost him the path or the answer to what he typed.
    // Once a step has run out of room they stay away until the step ends,
    // so they go at most once rather than coming and going with letters.
    // Each place's keys and words: in a modal line one place, the find's
    // completing; otherwise Tab's place, then the left arrow's and the
    // right's.
    readonly property string completeKey: modal
        ? prompt.drawn(QueryKeys.keyOf("complete", "find")) : "\u21e5"
    readonly property string clearKey: "\u2190"
    readonly property var keySlots: [[[completeKey, keyWords.complete ?? ""]],
        [[clearKey, keyWords.clear ?? ""], ["\u2192", keyWords.restore ?? ""]]]
    function slotWidth(slot, worded) {
        return Math.max(...slot.map(([key, word]) =>
            fieldKeys.advance(key, worded ? word : "")));
    }
    readonly property var keysWorded: keySlots.map(slot => slotWidth(slot, true))
    readonly property real keysWordedWidth: keysWorded.reduce((sum, wide) => sum + wide, 0)
        + (keysWorded.length - 1) * fieldKeys.spacing
    readonly property real asideWanted: asideWhole
    readonly property bool wordsFit: width - 2 * Theme.space.l - ledWidth - Theme.space.m
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

    // What the field says for its keys now: Tab while a completion shows,
    // then the left arrow while it clears what's typed or the right while it
    // puts back what the left took.
    readonly property var keysSaid: [
        typed && ghost !== "" ? [completeKey, keyWord(keyWords.complete)] : ["", ""],
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

    // Return. A modal line hears it only bare: with Shift, Ctrl or Alt it
    // does nothing (QueryKeys.js).
    signal accepted
    // Tab, which takes the ghost.
    signal completed

    // Return pressed, said as accepted only for a press of its own, never
    // for a repeat.
    function returned(event: KeyEvent) {
        if (!event.isAutoRepeat)
            prompt.accepted();
    }

    signal cancelled
    // The left arrow before what's typed starts (or anywhere, with
    // leftClears, once what's typed is gone) or Backspace with nothing
    // typed: back out; in a modal line, normal's -. Nothing steps in by an
    // arrow, since Return goes in (the vi-paradigm spec, Normal).
    signal backward
    // A modal line's keys for the rows below it, by their act's name, with
    // the count typed before it, 0 for none: the moves Operators.moved
    // takes; or, in the page mode, the drawer's, which DrawerPage.moved
    // takes.
    signal stepped(string act, int count)
    // An operator done: "ending" for d and "copy" for y, with the count
    // typed before it, 0 for none, and the motion it took, { act, count },
    // or null for dd and yy (QueryKeys.spanned takes them); in visual, d, y
    // or "run" for Return, with the motion { act: "visual" }, over the
    // selection.
    signal operated(string act, int count, var motion)
    // : in normal, which opens the command line in the line's place
    // (CommandLine.qml), with the range a count before it gives, as 3:
    // gives ".,.+2", or "" for none. The line holds what's typed as it
    // is, and resume() gives it the keys back.
    signal commanded(string range)
    // The drawer's search run, by Return on / or ?: what it seeks, empty
    // for the last again, the way it goes, and the count typed before it.
    signal sought(string pattern, bool forward, int count)
    // A key that means nothing in normal, by its name, which the launcher
    // answers with "not here", so no key is silent (the vi-paradigm spec).
    signal idled(string key)

    // A question asked once, said in the field's place, "Replace notes.md?
    // (y/n)", or "" while none is; the launcher's ask (Launcher.qml's
    // askOnce) answers it, by the keys `confirming` hands over.
    property string asking: ""
    property double askedAt: 0
    onAskingChanged: askedAt = Date.now()

    function clear() {
        cleared = "";
        input.text = "";
    }

    // Takes the keys back from whatever else in its window had them, as it
    // opens, so nothing cleared before is offered back, and a modal line
    // opens on the find, so typing finds at once.
    function take() {
        cleared = "";
        changes = [];
        standing = QueryKeys.rest();
        mode = typingMode;
        input.forceActiveFocus();
        woken();
    }

    // The keys back from the command line, in the mode it was opened from,
    // or in normal, what's typed as it was left.
    function resume(to: string) {
        standing = QueryKeys.rest();
        mode = to;
        input.forceActiveFocus();
        woken();
    }

    // Into insert, as a line asking for words wants, the caret at its end.
    function typing() {
        standing = QueryKeys.rest();
        if (mode !== "insert") {
            mode = "insert";
            input.cursorPosition = input.text.length;
        }
    }

    // Off the line into normal, as Escape leaves it, for a Return from the
    // find that lands on a row rather than doing it.
    function toNormal() {
        standing = QueryKeys.rest();
        if (mode === "normal")
            return;
        input.deselect();
        const at = input.cursorPosition;
        mode = "normal";
        input.cursorPosition = Typing.caretAfter("normal", input.text, at);
    }

    // Where the caret is, between letters in insert or on one in normal.
    readonly property int caret: input.cursorPosition

    // Puts the caret at a place in what's typed, as a save's name puts it
    // before the extension.
    function caretTo(at: int) {
        input.cursorPosition = Math.max(0, Math.min(at, input.text.length));
        rest();
    }

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
        [Qt.Key_Backtab]: "Tab",
        // Space by its name, since it types nothing a name could tell from
        // another key's, so normal's leader begins with it; insert's is
        // still the field's to type, the table giving it nothing there.
        [Qt.Key_Space]: "Space",
        // The bracket by its mark, for Ctrl+], which types a control
        // character rather than the mark; bare, it goes by what it typed.
        [Qt.Key_BracketRight]: "]",
        [Qt.Key_F1]: "F1"
    })
    // A key as the table names it (KeyNames.js).
    function named(event: KeyEvent): string {
        return KeyNames.nameOf({
            control: (event.modifiers & Qt.ControlModifier) !== 0,
            shift: (event.modifiers & Qt.ShiftModifier) !== 0,
            alt: (event.modifiers & Qt.AltModifier) !== 0,
            text: event.text,
            letter: event.key >= Qt.Key_A && event.key <= Qt.Key_Z
                ? String.fromCharCode(event.key + 32) : "",
            named: keyNames[event.key] ?? ""
        });
    }

    // A secret's keys, as Vim's inputsecret() takes them: Ctrl+C gives it
    // up as Escape does, Ctrl+U deletes back to the start, Ctrl+W the word
    // before and Ctrl+H the letter before. Everything else is the field's,
    // so the arrows only move the caret and Backspace on nothing does
    // nothing: no key of a secret's steps anywhere.
    function secretKey(event: KeyEvent) {
        const at = input.cursorPosition;
        const key = named(event);
        const from = key === "Ctrl+u" ? 0
            : key === "Ctrl+w" ? Typing.wordStart(input.text, at)
            : key === "Ctrl+h" ? Math.max(0, at - 1) : -1;
        if (key === "Ctrl+c") {
            prompt.cancelled();
            event.accepted = true;
        } else if (from >= 0) {
            input.remove(from, at);
            event.accepted = true;
        } else if (event.key === Qt.Key_Backspace && input.text === "") {
            event.accepted = true;
        }
    }
    // Whichever way the line's modes come and go, it comes back typing: on
    // the find, or in insert where it holds a prompt's words.
    onModalChanged: mode = typingMode

    // A modal line's key: what the table says it does in this mode, done
    // here, or left to the field to type.
    // A key Qt gives a handler of its own, read from the table all the
    // same, and kept from the field whether the table has it or not, since
    // the field would only take it again in Keys.onPressed.
    function tabled(event: KeyEvent) {
        modalKey(event);
        event.accepted = true;
    }

    // How many keys the line has taken, so what a key said, as the command
    // line's E492 or the act a landing's second Return does, stands until
    // the next one.
    property int keysTaken: 0

    function modalKey(event: KeyEvent) {
        const key = named(event);
        keysTaken += 1;
        woken();
        // A held key's repeats never answer it.
        if (confirming) {
            event.accepted = true;
            if (!event.isAutoRepeat)
                prompt.answered(key);
            return;
        }
        // The operator pressed again just after visual's is taken with it;
        // a key with no name, as Shift alone, lets nothing go.
        const after = visualOperated;
        if (key !== "")
            visualOperated = null;
        if (Operators.takenWith(after, key, Date.now(), wait)) {
            event.accepted = true;
            return;
        }
        // The find's first colon opens the command line, as normal's does,
        // from normal, where its Escape gives the line back.
        if (QueryKeys.opensCommands(key, mode, input.text)) {
            event.accepted = true;
            input.deselect();
            mode = "normal";
            standing = QueryKeys.rest();
            prompt.commanded("");
            return;
        }
        const read = QueryKeys.read(standing, key, mode, editsWords);
        // A key held where nothing types repeats only a move, as Vim's
        // motions do: any other act, and a count or a sequence begun, is a
        // press's once, so a key let go late by a busy shell, which
        // repeats as if held (modules/keyboard/repeat.nix), never does it
        // twice.
        if (event.isAutoRepeat && QueryKeys.still(mode) && !QueryKeys.repeats(read.act)) {
            event.accepted = true;
            return;
        }
        // With no drawer out, Space and the right arrow do nothing, as
        // his Ctrl+W l does with no window to the right.
        if (read.act === "drawer" && !drawerHere) {
            standing = read.standing;
            event.accepted = true;
            return;
        }
        standing = read.standing;
        if (key !== "")
            cleared = "";
        if (!read.ours)
            return;
        event.accepted = true;
        if (read.idle) {
            prompt.idled(key);
            return;
        }
        const text = input.text;
        const at = input.cursorPosition;
        // gv on the rows with no selection before it does nothing, as
        // Vim's; with one, the rows take it back before visual begins round
        // the cursor. The drawer's gv goes back to the mode it chose in.
        const onRows = !QueryKeys.paged(mode);
        if (read.act === "reselect" && onRows) {
            if (!reselectable)
                return;
            prompt.stepped(read.act, read.count);
        }
        // Backspace with nothing on the find leaves it for normal, as on
        // Vim's / line.
        const emptied = read.act === "deleteBack" && mode === "find" && text === "";
        const next = emptied ? "normal"
            : read.act === "reselect" && !onRows && lastChosen !== "" ? lastChosen
            : QueryKeys.modeAfter(read.act, mode, typingMode);
        if (next !== mode) {
            // Visual's d, y and Return take the selection before it ends,
            // as Vim's visual operators do (Launcher.qml's operated, which
            // the file picker's takes(rows) hooks for Return).
            if (mode === "visual" && QueryKeys.takesSelection(read.act)) {
                prompt.operated(read.act, 0, { act: "visual", count: 0 });
                visualOperated = { key, at: Date.now() };
            }
            // y copies what the drawer chose and gv brings the last back
            // before the mode moves, while it still holds what they act on,
            // and d x c p say they can't change it as the choice ends.
            else if (["copy", "copyLines", "cannotChange"].includes(read.act)
                || read.act === "reselect" && !onRows)
                prompt.stepped(read.act, read.count);
            input.deselect();
            // Going into insert on a prompt's words is a change u takes
            // back whole, as Vim's is, unless nothing's typed before it ends.
            if (words && next === "insert")
                changed(text, at);
            if (words && next === "normal" && changes.length > 0
                && changes[changes.length - 1].text === text)
                changes = changes.slice(0, -1);
            if (read.act === "change" || read.act === "freshFind")
                input.text = "";
            if (read.act === "changeWord") {
                const made = Editing.edited(read.act, text, at, read.count);
                input.text = made.text;
                mode = next;
                input.cursorPosition = made.at;
                return;
            }
            mode = next;
            input.cursorPosition = Typing.caretAfter(emptied ? "normal" : read.act,
                input.text, at);
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
            input.remove(Typing.wordStart(text, at), at);
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
        case "out":
            prompt.backward();
            break;
        case "dismiss":
            prompt.cancelled();
            break;
        case "ending":
        case "copy":
            prompt.operated(read.act, read.count, read.motion);
            break;
        case "command":
            prompt.commanded(QueryKeys.rangeFor(read.count, mode));
            break;
        case "letterBack":
        case "letterNext":
        case "wordNext":
        case "wordBack":
        case "wordEnd":
        case "lineStart":
        case "lineEnd":
        case "deleteLetter":
        case "deleteWordOn": {
            const made = Editing.edited(read.act, text, at, read.count);
            if (made.text !== text) {
                changed(text, at);
                input.text = made.text;
            }
            input.cursorPosition = made.at;
            break;
        }
        case "undo":
            undo();
            break;
        case "seek":
            prompt.sought(read.sought.pattern, read.sought.forward, read.count);
            break;
        case "":
            break;
        default:
            // ZZ is Vim's :x, writing what there is to write as it closes,
            // where Space w and q only close (modules/keys/language.toml).
            prompt.stepped(read.act === "close" && key === "Z" ? "xit" : read.act, read.count);
        }
    }

    // A change to a prompt's words, kept so u can take it back.
    function changed(text: string, at: int) {
        changes = changes.concat([{ text, at }]);
    }

    // Takes back the newest change to a prompt's words, the caret going
    // where it was before it, as Vim's u does; with none, nothing.
    function undo() {
        if (changes.length === 0)
            return;
        const last = changes[changes.length - 1];
        changes = changes.slice(0, -1);
        input.text = last.text;
        input.cursorPosition = Typing.resting(last.text, last.at);
    }

    // Normal's block rests on a letter, never past the last.
    function rest() {
        if (modal && mode === "normal") {
            const at = Typing.resting(input.text, input.cursorPosition);
            if (at !== input.cursorPosition)
                input.cursorPosition = at;
        }
    }
    onModeChanged: {
        rest();
        if (QueryKeys.chosen(mode))
            lastChosen = mode;
        else if (!QueryKeys.paged(mode))
            lastChosen = "";
    }

    // The caret's blink, as his kitty blinks a cursor (modules/kitty/
    // kitty.conf): a key shows it whole, it fades out and back in over
    // `fade` each way, linearly, and `stopsAfter` with no key it holds
    // steady, so a line left still draws nothing more. A fade of 0 holds
    // it steady throughout.
    property int fade: 800
    property int stopsAfter: 15000
    // Only a caret drawn blinks, so normal asks for no frames.
    readonly property bool blinks: modal && awake && fade > 0 && caretShown
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

    // How wide a letter is in the typed face, the mono's, a quarter of
    // which insert's bar is, as guicursor's ver25 says. A binding doesn't
    // see what a call reads, so it reads the face itself, to be measured
    // again once the theme's face arrives.
    readonly property real letterWidth: {
        void typedFace.font;
        return typedFace.advanceWidth("M");
    }
    readonly property real caretWidth: modal ? Math.max(1, Math.round(letterWidth / 4)) : 2
    // Whether the field draws its caret: not in normal, where the keys are
    // on the rows. A block there sat on the hint's first letter, in the
    // hint's face, before anything was typed (Joel, 2026-11-04). Nor in
    // visual, nor while the drawer holds the keys, as Neovim draws one
    // cursor, in the window the keys are in.
    readonly property bool caretShown: !(modal && QueryKeys.still(mode))

    FontMetrics {
        id: typedFace
        font: input.font
    }

    // A secret's dots stand a fifth of their size apart, so each reads as
    // one character. Points are 4/3 of a pixel at 96 dots an inch.
    readonly property real dotted: Theme.level.figure.pointSize * 4 / 3 * 0.2

    implicitWidth: 480
    implicitHeight: Theme.space.xxxl
    // The line's baseline, which the typed letters, the hint, the aside,
    // the glyph and a place's name all sit on, and the launcher's drawer
    // sets its eyebrow on. It's the largest level's, the place's name at
    // Title, set by centring its capitals on the field (docs/design.md, One
    // Baseline a Line), and it's there out of a place too, where no name
    // shows, so nothing on the line moves as one opens (Joel, 2026-11-05:
    // "which? &etc. need to be baselined with the text to their left").
    readonly property real lineBaseline: (height + titleCapitals) / 2
    baselineOffset: lineBaseline

    Text {
        id: glyphText
        x: Theme.space.l
        anchors.baseline: input.baseline
        text: prompt.glyph
        color: prompt.hue
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
    }

    Text {
        id: leadText
        x: Theme.space.l + prompt.glyphWidth
        anchors.baseline: input.baseline
        visible: prompt.lead !== ""
        text: prompt.lead
        color: prompt.leadHue
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
    }

    KeyHints {
        id: fieldKeys
        anchors.right: parent.right
        anchors.rightMargin: Theme.space.l
        anchors.baseline: input.baseline
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
        // drawn whole, over the hint that took the room.
        visible: prompt.asideRoom > 0
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
    // A question asked once is read whole, so the path gives way to it.
    readonly property real typingRoom: Math.max(
        voiceFace.advanceWidth(hint),
        12 * figureFace.averageCharacterWidth,
        asking === "" ? 0 : Math.ceil(figureFace.advanceWidth(asking)) + 1)

    // How wide the path wants to be, whole, with the gap after it.
    readonly property real pathWanted: chips.length === 0 ? 0
        : chipsWanted.reduce((sum, wide) => sum + wide, 0)
            + betweenDrawn.implicitWidth * (chips.length - 1) + pathGap - Theme.space.m

    // The path's steps as they're drawn, given way at whole names where
    // they don't fit (Reading.crumbs): a folder's path a folder at a time,
    // then the middle steps folded into one mark, so no name is cut.
    readonly property var crumbsShown: {
        void titleFace.font;
        const room = Math.max(0, end - Theme.space.l - ledWidth - typingRoom - pathGap);
        const stretch = titleStretch;
        // A step's whole words as their Text draws them, and words given
        // way as the face measures them, stretched as the title's axes do.
        const whole = new Map(chips.map((chip, at) => [chip.text, chipsWanted[at]]));
        return Reading.crumbs(chips.map(chip => ({ text: chip.text, yields: chip.yields === true })),
            room, text => whole.get(text) ?? Math.ceil(titleFace.advanceWidth(text) * stretch) + 1,
            betweenDrawn.implicitWidth);
    }
    // How much wider the title's axes draw its letters than its face's
    // metrics say, the most of any step's whole words, so a step given
    // way is measured as it will be drawn.
    readonly property real titleStretch: {
        void titleFace.font;
        let most = 1;
        chips.forEach((chip, at) => {
            const measured = titleFace.advanceWidth(chip.text);
            if (measured > 0)
                most = Math.max(most, ((chipsWanted[at] ?? 0) - 1) / measured);
        });
        return most;
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
    readonly property var chipsWhole: crumbsShown.whole


    // Where Joel is, the place's name at Title in the text colour, with no
    // chip, since its size says it's where he is; the marks between a
    // path's steps muted. Its capitals are centred on the field, setting
    // the line's baseline (lineBaseline), and its line box may run past the
    // field's rather than growing it, so the card is one height in a place
    // and out of it.
    Item {
        id: held
        x: Theme.space.l + prompt.ledWidth
        visible: prompt.chips.length > 0
        width: visible ? path.width : 0
        height: parent.height

        Row {
            id: path
            y: prompt.lineBaseline - titleFace.ascent

            Repeater {
                id: stepsShown
                model: prompt.crumbsShown.said

                Row {
                    id: step

                    required property string modelData
                    required property int index
                    // Whether its words are drawn cut, and the room it
                    // takes past them, for the launcher's report.
                    readonly property bool cut: stepName.truncated
                    readonly property real spare: stepName.width - stepName.implicitWidth

                    Text {
                        visible: step.index > 0
                        text: prompt.between
                        color: Theme.muted
                        font: titleFace.font
                    }

                    // Its words as they're given way, whole, so nothing
                    // is cut and the next mark follows them at its own
                    // spacing.
                    Text {
                        id: stepName
                        text: step.modelData
                        color: Theme.text
                        font: titleFace.font
                    }
                }
            }
        }
    }

    // How each of the path's steps is drawn, for the launcher's report: its
    // words, whether they're cut, and the room past them before the next.
    function pathDrawn(): var {
        const drawn = [];
        for (let at = 0; at < stepsShown.count; at += 1) {
            const step = stepsShown.itemAt(at);
            if (step)
                drawn.push({ said: step.modelData, cut: step.cut, spare: step.spare });
        }
        return drawn;
    }

    // Whether the question asked is drawn whole, for the launcher's report.
    readonly property bool askedWhole: askedText.implicitWidth <= askedText.width + 0.5

    // A question asked once, where the field's words were, in the field's
    // face and ink.
    Text {
        id: askedText
        x: input.x
        width: input.width
        anchors.baseline: input.baseline
        visible: prompt.asking !== ""
        text: prompt.asking
        textFormat: Text.PlainText
        elide: Text.ElideLeft
        color: Theme.strong
        font: input.font
    }

    TextInput {
        id: input
        x: Theme.space.l + prompt.ledWidth
            + (prompt.chips.length > 0 ? held.width + prompt.pathGap : 0)
        width: prompt.end - x
        y: prompt.lineBaseline - baselineOffset
        focus: true
        clip: true
        // A question asked draws in its place, and the field keeps the
        // keys, so it's hidden by its light alone: one not visible would
        // let its focus go.
        opacity: prompt.asking === "" ? 1 : 0
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
                && !(prompt.modal && QueryKeys.still(prompt.mode))) {
                event.accepted = true;
                return;
            }
            if (prompt.modal) {
                prompt.modalKey(event);
                return;
            }
            if (prompt.secret) {
                prompt.secretKey(event);
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
        // In a modal line it blinks, and only insert draws it.
        cursorDelegate: Rectangle {
            width: prompt.caretWidth
            color: Theme.accent
            opacity: prompt.caretLight
            visible: input.cursorVisible && prompt.caretShown
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
        // y over an answer copies what the ghost says, so the ghost flashes
        // as a row y copies does (Choice.qml's flashed): the accent's ground
        // under its own ink, fill and colour only. Like a find's wash
        // (Wash.qml) it lies under the letters alone, from the first to the
        // last, so the space that sets "= 84" off what's typed stays bare.
        // The letters are measured as a Text set exactly as the ghost's is,
        // its axes too, which a FontMetrics given the ghost's font left out,
        // measuring them narrow; the ghost ends in its letters, so the
        // ground ends where it does.
        Rectangle {
            readonly property real before: Math.max(0, hintText.implicitWidth - inked.implicitWidth)
            visible: prompt.ghostFlashed && hintText.visible && inked.text !== ""
            x: hintText.x + before
            y: hintText.y
            width: Math.max(0, Math.min(hintText.width, hintText.implicitWidth) - before)
            height: hintText.height
            color: Theme.accent

            Text {
                id: inked
                visible: false
                textFormat: Text.PlainText
                text: hintText.text.trim()
                font.family: hintText.level.family
                font.italic: hintText.level.italic
                font.pointSize: hintText.level.pointSize
                font.variableAxes: hintText.level.variableAxes
            }
        }

        Text {
            id: hintText
            readonly property bool voiced: input.text === "" && prompt.cleared === ""
            readonly property var level: hintText.voiced ? Theme.level.voice : Theme.level.figure
            // With nothing typed, past the caret where one is drawn, a mark
            // after it, so the caret never covers the hint's first letter
            // (VKCF3Z's space audit of the picker, fault 11).
            x: input.text !== "" ? input.contentWidth
                : prompt.caretShown ? prompt.caretWidth + Theme.space.rung.mark : 0
            width: Math.max(0, input.width - x)
            elide: Text.ElideRight
            anchors.baseline: parent.baseline
            textFormat: Text.PlainText
            text: input.text === "" ? prompt.cleared !== "" ? prompt.cleared : prompt.hint
                : prompt.ghost
            visible: text !== "" && (input.text === "" || !prompt.secret)
                && !(input.text !== "" && prompt.narrowSaid)
            color: prompt.ghostFlashed ? Theme.on_accent : Theme.muted
            font.family: hintText.level.family
            font.italic: hintText.level.italic
            font.pointSize: hintText.level.pointSize
            font.variableAxes: hintText.level.variableAxes
        }

        // The narrowed find's count, at Note, apart from what's typed, as
        // the ladder sets typed text and its count (docs/design.md).
        Text {
            x: input.contentWidth + Theme.space.rung.apart
            width: Math.max(0, input.width - x)
            elide: Text.ElideRight
            anchors.baseline: parent.baseline
            visible: prompt.narrowSaid
            text: `\u00b7 ${prompt.narrowed}`
            color: Theme.muted
            font.family: Theme.level.note.family
            font.pointSize: Theme.level.note.pointSize
            font.features: { "tnum": 1 }
        }
    }
}
