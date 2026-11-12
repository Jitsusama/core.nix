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
import QtQuick.Shapes
import "Reading.js" as Reading
import "Measures.js" as Measures
import "MarkSize.js" as MarkSize
import "Editing.js" as Editing
import "KeyNames.js" as KeyNames
import "QueryKeys.js" as QueryKeys
import "Operators.js" as Operators
import "Typing.js" as Typing
import "Bands.js" as Bands
import "Confirm.js" as Confirm

Item {
    id: prompt

    property string glyph
    property color hue: Theme.muted
    // Room between the glyph and what's typed: the launcher lines what's
    // typed up with its results' column, and a secret sits close by its lock.
    property real glyphWidth: glyphText.implicitWidth + Theme.space.rung.mark
    // Where its mark stands from its left, so a frame holding it sets the
    // mark on its own column by this rather than by a number of its own.
    readonly property real markAt: glyphText.x
    // A glyph after the first, leading the path: a held thing's, where the
    // find's / leads the line.
    property string lead
    property color leadHue: Theme.muted
    // The mode's mark, standing in the lead's cell while its mode lasts, as
    // Vim's one mode cell: the find's / or nothing, so the lead's glyph
    // and its mark take turns there and nothing after them moves (VKCF3Z,
    // 2026-11-06, for Joel's "an extra space in front of the launcher
    // heading ... that we don't have in find").
    property string cellMark
    property color cellHue: Theme.accent
    // Where the lead's ink centres across, which the mode's mark centres
    // on, whether the lead is drawn or not: a cell with no lead centres
    // its mark in a mark's room.
    readonly property real cellMiddle: lead === ""
        ? leadText.x + Theme.level.name.pitch / 2
        : leadText.x + (leadInk.tightBoundingRect.x + leadInk.tightBoundingRect.width / 2)
            / leadFine * leadScale
    // The mode mark's ink measured, so its middle stands on the cell's.
    TextMetrics {
        id: cellInk
        text: prompt.cellMark
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
    }
    // Given, the lead's column is held at it whether a lead is drawn or
    // not, so what's typed keeps its place as a thing is held and let go,
    // as the launcher's rows keep their glyph column.
    property real leadColumn: 0
    readonly property real leadWidth: leadColumn > 0 ? leadColumn : lead === "" ? 0
        : leadText.implicitWidth + Theme.space.rung.mark
    // Held on the rows' glyph column, the lead reads at the rows' mark size
    // and no wider than their cell, so a place's mark heads its rows at
    // theirs whatever its shape (MarkSize.js). A lock's padlock, given no
    // column, keeps its size, which the splash's prompt is drawn to match.
    readonly property real leadScale: leadColumn <= 0 ? 1
        : MarkSize.scale(leadInk.tightBoundingRect.width / leadFine, leadTall / leadFine,
            Theme.level.figure.pointSize, Theme.level.name.pitch)
    // The lead's ink measured at many times its size and read back down, as
    // a row measures its mark (Choice.qml), the outline giving its height,
    // since Qt's tight measure runs down to the baseline.
    readonly property real leadFine: 8
    readonly property real leadTall: leadOutline.boundingRect.height > 0
        ? Math.min(leadOutline.boundingRect.height, leadInk.tightBoundingRect.height)
        : leadInk.tightBoundingRect.height
    TextMetrics {
        id: leadInk
        text: prompt.lead
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize * prompt.leadFine
    }
    Shape {
        id: leadOutline
        visible: false
        ShapePath {
            PathText {
                text: prompt.lead
                font.family: Theme.level.figure.family
                font.pointSize: Theme.level.figure.pointSize * prompt.leadFine
            }
        }
    }
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
    // How far a find narrows the rows, as "3 found", said after what's
    // typed in normal, so a list narrowed always says so; nothing on the
    // find, where the rows narrow as it's typed (the vi-paradigm spec).
    property string narrowed
    readonly property bool narrowSaid: modal && mode === "normal" && narrowed !== ""
        && input.text !== ""
    // How the place the head is in stands where that presses, as "not
    // answering": said where a find's count is, ahead of it, in every mode,
    // since it's so whatever's typed (VKCF3Z, 2026-11-10).
    property string placeStands
    // The note on the head's baseline after what's typed or asked.
    readonly property string noteSaid: Typing.noteOf(placeStands, narrowed, narrowSaid)
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
    // typed before a move. A sequence begun waits for its next key however
    // long Joel pauses, as his Neovim's which-key holds a begun key once
    // its list shows: let go after timeoutlen, a Space then a pause before
    // the Shift that ? takes found the find upward instead of the key list
    // (Joel, 2026-11-05: "space ? does not work"). Escape lets it go.
    property var standing: QueryKeys.rest()
    // Whether an ask stands (Launcher.qml's askOnce), which takes every
    // key as its answer, as Vim's ask_yesno does, until one answers it.
    property bool confirming: false
    signal answered(string key)
    // How soon after visual's operator the same key again is taken with
    // it (Operators.takenWith), as his Neovim's timeoutlen
    // (modules/keys/language.toml); a sequence begun never times out.
    property int wait: 300
    // Whether the caret blinks: while the line is on screen and modal.
    property bool awake: true
    // Whether it takes keys at all: one it doesn't is dropped, every key
    // alike. A key niri sent just before it heard the launcher let the
    // keys go reached the line as it shut, and a second Return 200 ms
    // after :q's stepped into home's first row where Joel couldn't see it
    // (ZINAWZ, 2026-11-11: "→ Now" took 643 ms to its frame, unseen).
    property bool taking: true
    function dropped(event) {
        if (taking)
            return false;
        event.accepted = true;
        return true;
    }
    // Says how far an aside may run, so a long one never crowds what's typed
    // or where Joel is: the path's steps and room to type come first, and
    // the aside has all they leave. With the aside first, a step into a
    // thing whose rows have long notes cut every step of the path to a
    // letter (Joel, 2026-10-25: "the breadcrumb quickly goes single letter
    // per entry"). Held to two fifths of the line besides, it gave way with
    // room to spare beside a short hint (Joel, 2026-11-04).
    // What's typed with its completion comes before the aside too, so
    // "weather tokyo" keeps "= no forecast" whole and the aside says less.
    property real asideMost: Math.max(0, width - 2 * Theme.space.rung.inset
        - ledWidth - Theme.space.rung.apart - pathWanted - Math.max(typingRoom, typedWidest)
        - Theme.space.rung.groups)
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
        typedWidest = typedWanted;
        asideWidest = Math.min(asideWhole, asideMost);
    }

    // Where what's typed has to end: before the aside's room, and before
    // caps lock's glyph when it shows.
    readonly property real end: asideRoom === 0 ? width - Theme.space.rung.inset
        : asideText.x - Theme.space.rung.groups
            - (capitals ? Theme.space.rung.mark + capitalsGlyph.implicitWidth : 0)

    // The key that takes the completion, as the foot's hints say it: the
    // find's, from the key table, in a modal line, and Tab otherwise. The
    // field itself says no keys: a modal line's hints are the foot's, by
    // key and lowercase word (VKCF3Z, 2026-11-05), and the lock's and a
    // password's prompts are only typed in.
    readonly property string completeKey: modal
        ? prompt.drawn(QueryKeys.keyOf("complete", "find")) : "\u21e5"

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
    // Escape at rest in normal, which closes nothing but lets go what
    // the launcher shows over the rows, as the key list (Launcher.qml).
    signal rested
    // Whether the key pressed last is held, its press a repeat, and its
    // letting go, so what follows the selection waits out a held arrow
    // and comes as it's let go (Launcher.qml's drawer). Qt on Wayland
    // sends each repeat as a release and a press it marks as repeats, so
    // only a release it doesn't mark is the key let go.
    property bool keyHeld: false
    signal letGo
    // Normal's -: up to what holds this place. In a text line the arrows
    // only move the caret, and nothing steps in or out by one, since
    // Return goes in (the vi-paradigm spec, Normal).
    signal backward
    // A modal line's keys for the rows below it, by their act's name, with
    // the count typed before it, 0 for none: the moves Operators.moved
    // takes; or, in the page mode, the drawer's, which DrawerPage.moved
    // takes.
    signal stepped(string act, int count)
    // f or F's letter typed, with the count before the f, for the rows to
    // go to the one it begins (Finding.js).
    signal lettered(string act, int count, string letter)
    // Whether the find goes up, as ? opened it, which n goes on with and
    // N against, as Vim's do; / opens it going down.
    property bool findsBack: false
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
    // The same ask in its parts (Confirm.questionParts), where it says what
    // stays, so that clause alone gives way when the line can't hold it.
    property var askingParts: null
    property double askedAt: 0
    onAskingChanged: askedAt = Date.now()

    function clear() {
        input.text = "";
    }

    // Takes the keys back from whatever else in its window had them, as it
    // opens, with no change or half-pressed key left from before, and a
    // modal line opens on the find, so typing finds at once. That find is
    // a fresh /, going down, whatever way the last one opened went.
    function take() {
        changes = [];
        findsBack = false;
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
    // Whether what's typed is scrolled, its first letter off the field's
    // start, as it is once it's wider than the room the path leaves. Set
    // as the field moves rather than bound, since the mark it shows takes
    // room from the field, which moves what's typed again.
    property bool scrolled: false
    // Scrolled, the mark stands in the place of the first letter whole
    // inside the field, as Vim's precedes mark takes the first cell, and
    // the field is shown from the first letter past it: Qt draws a letter
    // scrolled off into the field's padding, where the mark would lie
    // over it, and a mark at the field's very start would leave a gap
    // the width of a letter wherever the letters fall.
    property real markFrom: 0
    property real shownFrom: 0
    function seeScrolled() {
        scrolled = input.text !== "" && input.positionToRectangle(0).x < input.leftPadding - 0.5;
        const letters = scrolled ? input.text.length : -1;
        let at = 0;
        while (at < letters && input.positionToRectangle(at).x < -0.5)
            at++;
        const mark = at <= letters ? input.positionToRectangle(at).x : 0;
        while (at < letters && input.positionToRectangle(at).x
            < mark + scrolledMark.implicitWidth - 0.5)
            at++;
        markFrom = scrolled ? mark : 0;
        shownFrom = scrolled ? input.positionToRectangle(at).x : 0;
    }
    // Where the field starts in the line, after the path.
    readonly property real fieldX: Theme.space.rung.inset + ledWidth
        + (held.visible ? held.width + pathGap : 0)

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
    // The last of them, by name, as Vim writes it.
    property string keyTaken: ""
    // Whether the last key only moved through the rows, as j and Down do,
    // which changes nothing a source holds, so the launcher never takes a
    // source changing beside it for the key's doing.
    property bool keyMoved: false

    // A key reaching a modal line, before it does anything, so what the
    // line acts on can be brought up to now first, as a launcher drawing
    // its first frame from what it kept is (Launcher.qml's filled).
    signal pressing

    function modalKey(event: KeyEvent) {
        prompt.pressing();
        const key = named(event);
        keyMoved = false;
        keyTaken = key;
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
        keyMoved = QueryKeys.repeats(read.act);
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
        if (!read.ours)
            return;
        event.accepted = true;
        if (read.rested) {
            prompt.rested();
            return;
        }
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
            if (read.act === "change" || read.act === "freshFind" || read.act === "freshFindBack")
                input.text = "";
            if (read.act === "freshFind" || read.act === "freshFindBack")
                findsBack = read.act === "freshFindBack";
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
        case "ending":
        case "copy":
            prompt.operated(read.act, read.count, read.motion);
            break;
        case "command":
            prompt.commanded(Operators.rangeFor(read.count, mode));
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
        case "toLetter":
        case "toLetterBack":
            prompt.lettered(read.act, read.count, read.letter);
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
    implicitHeight: 4 * Theme.space.beat
    // The line's baseline, which the typed letters, the hint, the aside,
    // the glyph and a place's name all sit on, and the launcher's drawer
    // sets its eyebrow on (Joel, 2026-11-05: "which? &etc. need to be
    // baselined with the text to their left"). It's the largest level's,
    // centred by eye in the field, the band a surface sizes it to: a
    // place's name at Title in a place, and what's typed out of one, so
    // each reads centred, the line settling as a place opens and its name
    // arrives (docs/design.md: one baseline a line).
    readonly property real lineBaseline: chips.length > 0
        ? Bands.baseline(height, titleCaps.tightBoundingRect.height,
            titleSmall.tightBoundingRect.height)
        : Bands.baseline(height, figureCaps.tightBoundingRect.height,
            figureSmall.tightBoundingRect.height)
    baselineOffset: lineBaseline
    // How far the line stands below the field's middle out of a place,
    // what's typed at the Figure: the line at rest, which a surface sizes
    // the field's band by, so the band holds still as a place opens and
    // only the line settles (docs/design.md: one baseline a line).
    readonly property real restingLift: Bands.lift(figureCaps.tightBoundingRect.height,
        figureSmall.tightBoundingRect.height)

    // Its glyph an inset from its left and its keys an inset from its right,
    // as a frame holds its text (docs/design.md, The Ladder), so a surface
    // sets it at its frame's inside edge.
    Text {
        id: glyphText
        x: Theme.space.rung.inset
        anchors.baseline: shown.baseline
        text: prompt.glyph
        color: prompt.hue
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
    }

    Text {
        id: leadText
        x: Theme.space.rung.inset + prompt.glyphWidth
        anchors.baseline: shown.baseline
        visible: prompt.lead !== "" && prompt.cellMark === ""
        text: prompt.lead
        color: prompt.leadHue
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize * prompt.leadScale
    }

    Text {
        id: cellText
        x: prompt.cellMiddle - cellInk.tightBoundingRect.x - cellInk.tightBoundingRect.width / 2
        anchors.baseline: shown.baseline
        visible: prompt.cellMark !== ""
        text: prompt.cellMark
        color: prompt.cellHue
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
    }

    Text {
        id: asideText
        anchors.right: parent.right
        anchors.rightMargin: Theme.space.rung.inset
        anchors.baseline: shown.baseline
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
        anchors.rightMargin: Theme.space.rung.mark
        anchors.baseline: shown.baseline
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
    // How tall the Title's and the Figure's capitals and small letters
    // stand, which centre the line: TextMetrics rather than a FontMetrics'
    // call, which a binding asks once, before the face has loaded, and
    // reads small ever after (HVD6O3, 2026-11-05).
    TextMetrics {
        id: titleCaps
        text: "H"
        font: titleFace.font
    }
    TextMetrics {
        id: titleSmall
        text: "x"
        font: titleFace.font
    }
    TextMetrics {
        id: figureCaps
        text: "H"
        font: figureFace.font
    }
    TextMetrics {
        id: figureSmall
        text: "x"
        font: figureFace.font
    }

    // What stands between two steps of the path, so they read as one place
    // reached through the other rather than as tags side by side: a rung
    // apart from each step's ink either side, measured to its own, so a
    // step is told from a word by its space as well as its mark (the
    // picker audit's finding 13: a word space either side read as one).
    readonly property string between: "\u203a"
    readonly property real betweenWide: 2 * Theme.space.rung.apart
        + betweenInk.tightBoundingRect.width / titleInkFine
    // Title's ink measured at eight times its size and read back down,
    // since a measure at the size drawn comes in whole pixels.
    readonly property real titleInkFine: 8
    // A step of the path, saying its words and how they're drawn, so
    // pathDrawn can read them by type.
    component PathStep: Row {
        required property string modelData
        property bool cut
        property real spare
    }
    component TitleInk: TextMetrics {
        font.family: Theme.level.title.family
        font.pointSize: Theme.level.title.pointSize * prompt.titleInkFine
        font.variableAxes: Theme.level.title.variableAxes
        font.letterSpacing: Theme.level.title.letterSpacing * prompt.titleInkFine
    }
    TitleInk {
        id: betweenInk
        text: prompt.between
    }
    // How far what's typed stands after the path.
    readonly property int pathGap: Theme.space.rung.apart

    // The hint as it's drawn, in the voice's face, since it's the shell
    // speaking to Joel: a face's metrics leave out the voice's axes, which
    // measured it narrow.
    Text {
        id: hintWide
        visible: false
        text: prompt.hint
        textFormat: Text.PlainText
        font.family: Theme.level.voice.family
        font.italic: Theme.level.voice.italic
        font.pointSize: Theme.level.voice.pointSize
        font.variableAxes: Theme.level.voice.variableAxes
    }

    // The room kept for typing: the hint's, or what's typed at its widest
    // since the step began, so the path gives way once as what's typed
    // grows past the hint and never for room nothing is in. A dozen
    // characters kept ahead folded a middle folder to a mark with room
    // for its last name beside the hint (VKCF3Z, 2026-11-06: at rest the
    // path says "\u2026/more", so Joel sees which folder he's in). A
    // question asked once is read whole, so the path gives way to it.
    readonly property real typingRoom: Math.max(
        (caretShown ? caretWidth + Theme.space.rung.mark : 0) + Math.ceil(hintWide.implicitWidth),
        typedWidest + caretWidth,
        asking === "" ? 0 : Math.ceil(figureFace.advanceWidth(asking)) + 1)

    // How wide the path wants to be, whole, with the gap after it.
    readonly property real pathWanted: chips.length === 0 ? 0
        : chipsWanted.reduce((sum, wide) => sum + wide, 0)
            + betweenWide * (chips.length - 1) + pathGap - Theme.space.rung.apart

    // The path's steps as they're drawn, given way at whole names where
    // they don't fit (Reading.crumbs): a folder's path a folder at a time,
    // then the middle steps folded into one mark, so no name is cut. What's
    // typed takes room from the path only down to its least, the ask and
    // where Joel is whole; past it, what's typed scrolls in what's left, as
    // Vim's command line scrolls a long line (VKCF3Z, 2026-11-06).
    readonly property var crumbsShown: {
        void crumbsMade;
        const line = end - Theme.space.rung.inset - ledWidth - pathGap;
        const width = text => Math.ceil(Measures.drawnWidth(crumbsFace, text)
            ?? titleFace.advanceWidth(text)) + 1;
        const steps = crumbSteps;
        const least = Reading.crumbsLeast(steps, width, betweenWide);
        const room = Math.max(0, line - typingRoom, Math.min(least, line));
        return Reading.crumbs(steps, room, width, betweenWide);
    }
    readonly property var crumbSteps: chips.map(chip =>
        ({ text: chip.text, yields: chip.yields === true }))
    // Every way the path's steps may be said, each as a Text draws it: a
    // face's metrics leave out the title's axes, and a face hinted fully
    // draws in whole pixels, so a width scaled from the face's measure
    // came out several pixels wide and folded a folder with room for it.
    // Until a way's Text is made, the face's measure stands. Each is kept
    // once drawn (Measures.drew), so only a way not drawn before is made.
    readonly property string crumbsFace: Measures.drawnFace(titleFace.font, "plain")
    // How many ways and steps have been drawn and kept, read by what
    // they're measured for, and by the launcher, which counts those an
    // opening made.
    property int crumbsMade: 0
    property int stepsMade: 0

    Repeater {
        id: crumbsDrawn
        model: Reading.crumbTexts(prompt.crumbSteps)
            .filter(text => Measures.drawnWidth(prompt.crumbsFace, text) === undefined)

        Text {
            required property string modelData
            visible: false
            text: modelData
            textFormat: Text.PlainText
            font: titleFace.font
            Component.onCompleted: kept()
            onImplicitWidthChanged: kept()
            function kept() {
                Measures.drew(prompt.crumbsFace, text, implicitWidth);
                prompt.crumbsMade += 1;
            }
        }
    }

    // Each step's words' width, whole, as a Text draws them: a face's
    // metrics leave out the title's axes, which change its letters'
    // widths, so measured by them a step came out a hair short and was cut.
    // Until a step's Text is made for its words, the face's measure stands.
    readonly property string stepsFace: Measures.drawnFace(titleFace.font, "auto")
    readonly property var chipsWanted: stepsMade >= 0 ? chips.map(chip =>
        Math.ceil(Measures.drawnWidth(stepsFace, chip.text)
            ?? titleFace.advanceWidth(chip.text)) + 1) : []

    Repeater {
        id: stepsDrawn
        model: prompt.chips
            .filter(chip => Measures.drawnWidth(prompt.stepsFace, chip.text) === undefined)

        Text {
            required property var modelData
            visible: false
            text: modelData.text
            font: titleFace.font
            Component.onCompleted: kept()
            onImplicitWidthChanged: kept()
            function kept() {
                Measures.drew(prompt.stepsFace, text, implicitWidth);
                prompt.stepsMade += 1;
            }
        }
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
        x: Theme.space.rung.inset + prompt.ledWidth
        // An ask takes the whole line while it stands, as the command line
        // does, and the path comes back whole once it's answered, never
        // cut to make room for it (VKCF3Z and Y2YBTS, 2026-11-07, after
        // Close's ask was cut to its end beside a step-in's path).
        visible: prompt.chips.length > 0 && prompt.asking === ""
        width: visible ? path.width : 0
        height: parent.height

        Row {
            id: path
            y: prompt.lineBaseline - titleFace.ascent

            Repeater {
                id: stepsShown
                model: prompt.crumbsShown.said

                PathStep {
                    id: step

                    required property int index
                    // Whether its words are drawn cut, and the room it
                    // takes past them, for the launcher's report.
                    cut: stepName.truncated
                    spare: stepName.width - stepName.implicitWidth

                    // Where its words' ink starts past their advance, and
                    // where the step before's ends short of its, so the
                    // mark stands its rung from the ink either side: by
                    // the first and last letters, since a face's measure
                    // of whole words leaves out the title's axes, which
                    // widen them, while a letter's bearing barely moves.
                    TitleInk {
                        id: stepInk
                        text: Array.from(step.modelData)[0] ?? ""
                    }
                    TitleInk {
                        id: beforeInk
                        text: step.index > 0
                            ? Array.from(prompt.crumbsShown.said[step.index - 1] ?? "").pop() ?? ""
                            : ""
                    }
                    readonly property real leads: stepInk.tightBoundingRect.x / prompt.titleInkFine
                    readonly property real trailsBefore: step.index === 0 ? 0
                        : (beforeInk.advanceWidth - beforeInk.tightBoundingRect.x
                            - beforeInk.tightBoundingRect.width) / prompt.titleInkFine

                    Item {
                        visible: step.index > 0
                        width: visible ? prompt.betweenWide - step.trailsBefore - step.leads : 0
                        height: stepName.height

                        Text {
                            x: Theme.space.rung.apart - step.trailsBefore
                                - betweenInk.tightBoundingRect.x / prompt.titleInkFine
                            text: prompt.between
                            color: Theme.muted
                            font: titleFace.font
                        }
                    }

                    // Its words as they're given way, whole, so nothing
                    // is cut and the next mark follows them at its own
                    // spacing.
                    Text {
                        id: stepName
                        text: step.modelData
                        // The Title's own ink, strong, so the heading
                        // outranks the cursor row's name beneath it.
                        color: Theme.level.title.colour
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
            const step = stepsShown.itemAt(at) as PathStep;
            if (step)
                drawn.push({ said: step.modelData, cut: step.cut, spare: step.spare });
        }
        return drawn;
    }

    // Whether the question asked is drawn whole, for the launcher's report.
    readonly property bool askedWhole: askedText.implicitWidth <= askedText.width + 0.5
    // And the words drawn, its clause given way where it had to.
    readonly property string askedShown: askedText.visible ? askedText.text : ""

    // A question asked once, where the field's words were, in the field's
    // face and ink.
    Text {
        id: askedText
        x: prompt.fieldX
        width: input.width
        anchors.baseline: shown.baseline
        visible: prompt.asking !== ""
        text: prompt.askingParts === null ? prompt.asking
            : Confirm.fitted(prompt.askingParts,
                words => Math.ceil(figureFace.advanceWidth(words)) + 1 <= width)
        textFormat: Text.PlainText
        elide: Text.ElideLeft
        color: Theme.strong
        font: input.font
    }

    // Scrolled, a muted mark at the field's start says letters went off
    // it, as Vim's precedes mark does a line scrolled sideways, its room
    // taken from what's typed rather than the path (VKCF3Z, 2026-11-06).
    Text {
        id: scrolledMark
        objectName: "precedes"
        x: prompt.fieldX + prompt.markFrom
        anchors.baseline: shown.baseline
        visible: prompt.scrolled
        textFormat: Text.PlainText
        text: "\u2026"
        color: Theme.muted
        font: input.font
    }

    // The field as shown, cut where its first whole letter starts once
    // it's scrolled. The field keeps its place in the line however much
    // is cut, so the cut never moves what's typed.
    Item {
        id: shown
        x: prompt.fieldX + prompt.shownFrom
        width: Math.max(0, input.width - prompt.shownFrom)
        height: prompt.height
        baselineOffset: input.y + input.baselineOffset
        clip: prompt.scrolled
    }

    TextInput {
        id: input
        parent: shown
        x: -prompt.shownFrom
        width: prompt.end - prompt.fieldX
        // Room for the caret after the last letter, so what's typed,
        // scrolled, ends short of the field's end by it rather than the
        // caret standing past the end.
        rightPadding: prompt.caretWidth
        // Scrolled, the mark's room, kept clear of the caret.
        leftPadding: prompt.scrolled ? scrolledMark.implicitWidth : 0
        onCursorRectangleChanged: prompt.seeScrolled()
        onContentWidthChanged: prompt.seeScrolled()
        onWidthChanged: prompt.seeScrolled()
        // Qt scrolls again only as the caret or the words move, so the
        // mark's room, once taken, would lie over the first letter shown.
        onLeftPaddingChanged: {
            ensureVisible(cursorPosition);
            prompt.seeScrolled();
        }
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
        // insert, where it only leaves for normal, and Tab would complete
        // rather than move.
        Keys.onReturnPressed: event => prompt.dropped(event) ? undefined
            : prompt.modal ? prompt.tabled(event) : prompt.returned(event)
        Keys.onEnterPressed: event => prompt.dropped(event) ? undefined
            : prompt.modal ? prompt.tabled(event) : prompt.returned(event)
        Keys.onTabPressed: event => prompt.dropped(event) ? undefined
            : prompt.modal ? prompt.tabled(event) : prompt.completed()
        Keys.onEscapePressed: event => prompt.dropped(event) ? undefined
            : prompt.modal ? prompt.tabled(event) : prompt.cancelled()
        onCursorPositionChanged: prompt.rest()
        Keys.onReleased: event => {
            if (!event.isAutoRepeat && prompt.keyHeld) {
                prompt.keyHeld = false;
                prompt.letGo();
            }
        }
        Keys.onPressed: event => {
            prompt.keyHeld = event.isAutoRepeat;
            if (prompt.dropped(event))
                return;
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
            // Every other key is the field's, so the arrows only move the
            // caret: no key of a plain line's steps anywhere.
            if (prompt.secret)
                prompt.secretKey(event);
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
        // The hint is the shell's voice, in every field, sized by capitals
        // and sat on the typed letters' baseline, since the two faces' line
        // boxes differ; the ghost is letters typed,
        // in the field's own face (look 3's piece 1).
        // y over an answer copies what the ghost says, so the ghost flashes
        // as a row y copies does (Choice.qml's flashed): the accent's ground
        // under its own ink, fill and colour only. Like a find's wash
        // (Wash.qml) it lies under the letters alone, from the first to the
        // last, so the space that sets "= 84" off what's typed stays bare.
        // The letters are measured as a Text set exactly as the ghost's is,
        // its axes too, which a FontMetrics given the ghost's font left out,
        // measuring them narrow; the ghost ends in its letters, so the
        // ground ends where it does. Only the ghost flashes, since only it
        // says the answer y copies.
        Rectangle {
            readonly property real before: Math.max(0, ghostText.implicitWidth - inked.implicitWidth)
            visible: prompt.ghostFlashed && ghostText.visible && inked.text !== ""
            x: ghostText.x + before
            y: ghostText.y
            width: Math.max(0, Math.min(ghostText.width, ghostText.implicitWidth) - before)
            height: ghostText.height
            color: Theme.accent

            Text {
                id: inked
                visible: false
                textFormat: Text.PlainText
                text: ghostText.text.trim()
                font.family: Theme.level.figure.family
                font.italic: Theme.level.figure.italic
                font.pointSize: Theme.level.figure.pointSize
                font.variableAxes: Theme.level.figure.variableAxes
            }
        }

        // The hint and the ghost are a Text each, each in its own face for
        // good, rather than one Text whose face turns as the field empties
        // or fills: turning it, under load, was 7 to 18 ms of a step into a
        // place from a find, and with a Text each that time was gone
        // (keeps-pace's step probe, 2026-11-10).
        Text {
            id: hintText
            // With nothing typed, past the caret where one is drawn, a mark
            // after it, so the caret never covers the hint's first letter
            // (VKCF3Z's space audit of the picker, fault 11).
            x: prompt.caretShown ? prompt.caretWidth + Theme.space.rung.mark : 0
            width: Math.max(0, input.width - x)
            elide: Text.ElideRight
            anchors.baseline: parent.baseline
            textFormat: Text.PlainText
            text: prompt.hint
            // A place that can't answer says so in its question's stead, so
            // the head keeps to two levels.
            visible: input.text === "" && text !== "" && prompt.placeStands === ""
            color: Theme.muted
            font.family: Theme.level.voice.family
            font.italic: Theme.level.voice.italic
            font.pointSize: Theme.level.voice.pointSize
            font.variableAxes: Theme.level.voice.variableAxes
        }

        Text {
            id: ghostText
            x: input.contentWidth
            width: Math.max(0, input.width - x)
            elide: Text.ElideRight
            anchors.baseline: parent.baseline
            textFormat: Text.PlainText
            text: prompt.ghost
            visible: input.text !== "" && text !== "" && !prompt.secret && prompt.noteSaid === ""
            color: prompt.ghostFlashed ? Theme.on_accent : Theme.muted
            font.family: Theme.level.figure.family
            font.italic: Theme.level.figure.italic
            font.pointSize: Theme.level.figure.pointSize
            font.variableAxes: Theme.level.figure.variableAxes
        }

        // The narrowed find's count, or how the place stands, apart from
        // what's typed, muted at the typed letters' own Figure, so
        // the line holds two levels at most, a place's name and what's typed
        // (VKCF3Z, 2026-11-06: one calm head).
        Text {
            x: (input.text !== "" ? input.contentWidth : hintText.x) + Theme.space.rung.apart
            width: Math.max(0, input.width - x)
            elide: Text.ElideRight
            anchors.baseline: parent.baseline
            visible: prompt.noteSaid !== ""
            text: `\u00b7 ${prompt.noteSaid}`
            color: Theme.muted
            font.family: Theme.level.figure.family
            font.pointSize: Theme.level.figure.pointSize
            font.weight: Theme.level.figure.weight
            font.features: Theme.level.figure.features
        }
    }
}
