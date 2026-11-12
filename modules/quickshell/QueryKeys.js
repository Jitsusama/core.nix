// The keys of the launcher's line, in its modes, as one table of key and
// act (docs/design.md, Vi's Keys), read apart from the field (Prompt.qml) so
// tests/query-keys.qml can hold it in Qt's own engine, and so the desktop's
// one language of keys can take it in later. The line is Vim's / line, the
// rows a buffer below it: it opens on the find, where letters type and the
// rows narrow, and Escape gives normal, where nothing types. Words a row
// asks for are typed in insert, edited with the find's keys. Letters go by
// what they type, wherever they sit on the layout.
.pragma library
.import "Finding.js" as Finding
.import "Operators.js" as Operators

// Each act, the mode it's done in and the keys that do it. A key is named as
// the field names it (Prompt.qml, `named`): a modifier, then the letter it
// typed, or else its name, with Shift unnamed beside a letter, which says it
// by its case. A list of keys is a sequence typed one after another, as vi's
// cc. A row's mode "line" is the find's and insert's, "both" every mode's. A
// key a mode doesn't name is the field's own where it types, and does
// nothing in normal. Where one differs from Neovim, the reason is beside it.
const table = [
    // The line, as Vim's command line: the caret's moves and deleting
    // before it, Ctrl+H as Backspace.
    { act: "caretLeft", mode: "line", keys: ["Left"] },
    { act: "caretRight", mode: "line", keys: ["Right"] },
    { act: "caretStart", mode: "line", keys: ["Home"] },
    { act: "caretEnd", mode: "line", keys: ["End"] },
    { act: "deleteBack", mode: "line", keys: ["Backspace", "Ctrl+h"] },
    { act: "deleteWord", mode: "line", keys: ["Ctrl+w"] },
    { act: "deleteToStart", mode: "line", keys: ["Ctrl+u"] },
    // On the find, Tab goes into a row holding parts, as a program's page,
    // and is the next row on any other (QueryKeys reads the first row a key
    // is in, so this one comes before the next row's), since next only
    // repeats Down and Ctrl+N there (Joel, 2026-11-07: "Tab opens its
    // page"; VKCF3Z, 7f53e7b3).
    { act: "into", mode: "find", keys: ["Tab"] },
    // The rows below are his completion menu's (modules/neovim/keys.nix):
    // the next and the one before, and Ctrl+Y takes the selected one into
    // the line, as the grey completion after what's typed shows it.
    { act: "next", mode: "line", keys: ["Down", "Tab", "Ctrl+n"] },
    { act: "previous", mode: "line", keys: ["Up", "Shift+Tab", "Ctrl+p"] },
    { act: "complete", mode: "line", keys: ["Ctrl+y"] },
    // Escape and Ctrl+C leave the line for normal, the rows as found: on a
    // list the find is what the list is (the vi-paradigm spec's departure 5).
    { act: "normal", mode: "line", keys: ["Escape", "Ctrl+c"] },

    // Normal, over the rows as a buffer: Vim's moves, each taking a count
    // typed before it (3j). gg and {n}G go to the first row and the nth, G
    // to the last; H M L to the top, middle and bottom row in view; Ctrl+D
    // and Ctrl+U half a screen, Ctrl+F and Ctrl+B, as Page Down and Page
    // Up, a screen (moved, below).
    // In normal, in a buffer, the arrows move the cursor as their letters
    // do (Joel, 2026-11-05 12:1x), listed after them so no hint names one.
    { act: "next", mode: "normal", keys: ["j", "Down"] },
    { act: "previous", mode: "normal", keys: ["k", "Up"] },
    { act: "first", mode: "normal", keys: [["g", "g"]] },
    { act: "last", mode: "normal", keys: ["G"] },
    { act: "viewTop", mode: "normal", keys: ["H"] },
    { act: "viewMiddle", mode: "normal", keys: ["M"] },
    { act: "viewBottom", mode: "normal", keys: ["L"] },
    { act: "halfDown", mode: "normal", keys: ["Ctrl+d"] },
    { act: "halfUp", mode: "normal", keys: ["Ctrl+u"] },
    { act: "pageDown", mode: "normal", keys: ["Ctrl+f"] },
    { act: "pageUp", mode: "normal", keys: ["Ctrl+b"] },
    // Return goes in ("run", below) and - up, as his oil's do; h and l, and
    // Backspace as h, only say so, as q says how to close, and no arrow
    // does either (Joel, 2026-11-05: "where in vi does that step into shit?").
    { act: "out", mode: "normal", keys: ["-"] },
    { act: "intoSaid", mode: "normal", keys: ["l"] },
    { act: "outSaid", mode: "normal", keys: ["h", "Backspace"] },
    // ZZ closes, as his Neovim closes a buffer, Space w as his leader closes a
    // split, and :q (CommandLine.qml); q closes nothing, there being no
    // macros, and says so until the next key (the vi-paradigm spec, Normal).
    { act: "close", mode: "normal", keys: [["Z", "Z"], ["Space", "w"]] },
    { act: "quitSaid", mode: "normal", keys: ["q"] },
    // Ctrl+O and Ctrl+I go back and forward through the places stepped into,
    // as Vim's jump list; Space Tab goes to the place used last, as his goes
    // to the file used last. Tab is apart from Ctrl+I, as Neovim under
    // kitty's keyboard protocol tells them apart, and goes into a row
    // holding parts, a program's page or anything else Ctrl+] goes into,
    // saying "not here" on any other (Joel, 2026-11-07: "keep", Return
    // still switching; VKCF3Z, 7fd0aa50).
    { act: "jumpBack", mode: "normal", keys: ["Ctrl+o"] },
    { act: "jumpForward", mode: "normal", keys: ["Ctrl+i"] },
    { act: "into", mode: "normal", keys: ["Tab"] },
    { act: "lastPlace", mode: "normal", keys: [["Space", "Tab"]] },
    // Ctrl+] goes to the row's own place, as Vim follows a tag, from the find too; Ctrl+T back.
    { act: "follow", mode: "normal", keys: ["Ctrl+]"] },
    { act: "follow", mode: "find", keys: ["Ctrl+]"] },
    { act: "tagBack", mode: "normal", keys: ["Ctrl+t"] },
    // Every key here, as Space ? lists them, and F1; Space alone waits, as gg's g does.
    { act: "keys", mode: "normal", keys: [["Space", "?"], "F1"] },
    // Vim's operators on the rows as lines: d does each row's ending
    // action (Forget It, Close It) and y its copy, over the row again as
    // dd and yy, or as far as a motion goes (operated, below), so these
    // rows name only the doubled keys, for saying them. u takes back what
    // d did, Ctrl+R does it again, and . does the last d again.
    { act: "ending", mode: "normal", keys: [["d", "d"]] },
    { act: "copy", mode: "normal", keys: [["y", "y"]] },
    { act: "undoChange", mode: "normal", keys: ["u"] },
    { act: "redoChange", mode: "normal", keys: ["Ctrl+r"] },
    { act: "again", mode: "normal", keys: ["."] },
    // Visual line, Vim's V: the rows from where it began to the cursor,
    // extended by normal's moves (inMode) and taken at once by d, y and
    // Return; a colon hands them to the command line. On rows v is V, a row
    // having no letters, and x is d, o the other end and gv the last, as Vim's.
    { act: "selectLines", mode: "normal", keys: ["V", "v"] },
    { act: "selectLines", mode: "visual", keys: ["V", "v"] },
    { act: "ending", mode: "visual", keys: ["d", "x"] },
    { act: "copy", mode: "visual", keys: ["y"] },
    { act: "normal", mode: "visual", keys: ["Escape", "Ctrl+c"] },
    { act: "otherEnd", mode: "visual", keys: ["o"] },
    // Space leads in visual too, so the key list stands over a selection.
    { act: "keys", mode: "visual", keys: [["Space", "?"], "F1"] },
    { act: "reselect", mode: "normal", keys: [["g", "v"]] },
    // A paragraph of rows is a run of one kind or sender that a gap ends
    // (Operators.runAt): } and { go by them, ip and ap take one, as in Vim.
    { act: "paragraphOn", mode: "normal", keys: ["}"] },
    { act: "paragraphBack", mode: "normal", keys: ["{"] },
    { act: "inParagraph", mode: "object", keys: [["i", "p"]] },
    { act: "aParagraph", mode: "object", keys: [["a", "p"]] },
    // The gutter's numbers, relative or absolute, as his u l switches them.
    { act: "numbers", mode: "normal", keys: [["Space", "u", "l"]] },
    // Back to the find: / afresh going down and ? going up, the way n goes
    // on; i a I A holding the find, at the caret, after it, at the line's
    // start and its end; or cleared first, as cc clears a line.
    { act: "freshFind", mode: "normal", keys: ["/"] },
    { act: "freshFindBack", mode: "normal", keys: ["?"] },
    // n and N go on through the rows the find found, its way and back, round
    // past an end; f and F to the next row and the one before whose name
    // begins with the letter typed after, and ; and , again (Finding.js).
    { act: "matchNext", mode: "normal", keys: ["n"] },
    { act: "matchBack", mode: "normal", keys: ["N"] },
    { act: "toLetter", mode: "normal", keys: ["f"] },
    { act: "toLetterBack", mode: "normal", keys: ["F"] },
    { act: "letterAgain", mode: "normal", keys: [";"] },
    { act: "letterAgainBack", mode: "normal", keys: [","] },
    { act: "insert", mode: "normal", keys: ["i"] },
    { act: "append", mode: "normal", keys: ["a"] },
    { act: "insertStart", mode: "normal", keys: ["I"] },
    { act: "appendEnd", mode: "normal", keys: ["A"] },
    { act: "change", mode: "normal", keys: [["c", "c"], "S"] },
    // The place beside this one, the next and the one before, as vi's ]b
    // and [b go to the next buffer: the language's next-file and
    // previous-file (modules/keys/language.toml). A place says what's
    // beside it, as the file picker's folders do; elsewhere they do nothing.
    { act: "nextFile", mode: "normal", keys: [["]", "b"]] },
    { act: "previousFile", mode: "normal", keys: [["[", "b"]] },
    // The row's file opened in its own program, as his oil's gx opens the
    // file under the cursor; a row that isn't a file does nothing.
    { act: "openOwn", mode: "normal", keys: [["g", "x"]] },
    // @: runs the last command line again, as Vim's does (the language's
    // again-command); @ alone waits for it.
    { act: "commandAgain", mode: "normal", keys: [["@", ":"]] },
    // Vim's Ctrl+W l and w go into the drawer, and his Neovim's Space and the
    // right arrow over them (Joel, 2026-11-05: "leader stuff can overlay").
    { act: "drawer", mode: "normal", keys: [["Ctrl+w", "l"], ["Ctrl+w", "w"], ["Space", "Right"]] },

    // A prompt's words in normal, as Vim edits a line: w, b and e to the
    // next word's start, back to a word's start and on to its end, 0 and $
    // to the line's ends, x the letter under the caret, dw and cw the word
    // it's on, cw going into insert, and u back to before the last change.
    // They're read in normal only while the line holds a prompt's words
    // rather than a find (Prompt.qml, `words`), and before normal's own,
    // so u there undoes rather than bringing a find back.
    // h and l move the caret a letter, as Vim's do, as do \u2190, \u2192 and Backspace.
    { act: "letterBack", mode: "words", keys: ["h", "Left", "Backspace"] },
    { act: "letterNext", mode: "words", keys: ["l", "Right"] },
    { act: "wordNext", mode: "words", keys: ["w"] },
    { act: "wordBack", mode: "words", keys: ["b"] },
    { act: "wordEnd", mode: "words", keys: ["e"] },
    { act: "lineStart", mode: "words", keys: ["0"] },
    { act: "lineEnd", mode: "words", keys: ["$"] },
    { act: "deleteLetter", mode: "words", keys: ["x"] },
    { act: "deleteWordOn", mode: "words", keys: [["d", "w"]] },
    { act: "changeWord", mode: "words", keys: [["c", "w"]] },
    { act: "undo", mode: "words", keys: ["u"] },

    // Every mode: Return opens the row or steps into it, and with Shift,
    // Ctrl or Alt does nothing (the vi-paradigm spec's Return); Page Up and
    // Page Down page the rows. Shift and the arrows do nothing: an arrow is
    // its letter (VKCF3Z's spec v2, The Arrows).
    { act: "run", mode: "both", keys: ["Return", "Enter"] },
    { act: "pageUp", mode: "both", keys: ["PageUp"] },
    { act: "pageDown", mode: "both", keys: ["PageDown"] },

    // The drawer, gone into: the preview window, a page in normal with a
    // cursor on its lines (DrawerPage.js), as the reader's page and his
    // Neovim's windows have them. j k move it a line and h l along one, the
    // view kept about it; Ctrl+E and Ctrl+Y scroll the view a line, Ctrl+D
    // and Ctrl+U half the view, Ctrl+F and Ctrl+B a view, with Page Down
    // and Page Up (both modes', above); gg G and H M L as on the rows, each
    // taking a count. Ctrl+W h and w go back, the foot naming h, Space and
    // the left arrow too, and ZZ, Space w and :q, as his Neovim closes a
    // window; a bare arrow is its letter's move, as in a buffer (Joel,
    // 2026-11-05). Escape only lets a count or a waiting key go, as it does
    // nothing at rest in a window, and q alone only says how to close, as
    // Joel asked of every modal surface (2026-11-04). Return does the row.
    { act: "next", mode: "page", keys: ["j", "Down"] },
    { act: "previous", mode: "page", keys: ["k", "Up"] },
    { act: "left", mode: "page", keys: ["h", "Left"] },
    { act: "right", mode: "page", keys: ["l", "Right"] },
    { act: "scrollDown", mode: "page", keys: ["Ctrl+e"] },
    { act: "scrollUp", mode: "page", keys: ["Ctrl+y"] },
    { act: "halfDown", mode: "page", keys: ["Ctrl+d"] },
    { act: "halfUp", mode: "page", keys: ["Ctrl+u"] },
    { act: "pageDown", mode: "page", keys: ["Ctrl+f"] },
    { act: "pageUp", mode: "page", keys: ["Ctrl+b"] },
    { act: "first", mode: "page", keys: [["g", "g"]] },
    { act: "last", mode: "page", keys: ["G"] },
    { act: "viewTop", mode: "page", keys: ["H"] },
    { act: "viewMiddle", mode: "page", keys: ["M"] },
    { act: "viewBottom", mode: "page", keys: ["L"] },
    { act: "rows", mode: "page", keys: [["Ctrl+w", "h"], ["Ctrl+w", "w"], ["Space", "Left"],
        ["Z", "Z"], ["Space", "w"]] },
    // The leader's list of keys is his everywhere (VKCF3Z), so the drawer
    // lists its own; not while choosing, where Space leads nothing and ?
    // searches back.
    { act: "keys", mode: "page", keys: [["Space", "?"], "F1"] },
    // n and N go on to the search's next match, the way it went and the
    // other, each a move, so a held n repeats as j does; / and ? open the
    // search itself (seekRead).
    { act: "seekNext", mode: "page", keys: ["n"] },
    { act: "seekPrevious", mode: "page", keys: ["N"] },
    // za opens or folds its nerdy bits, as Vim's za a fold (the drawer
    // spec's part 7), and they stay as he left them from row to row.
    { act: "fold", mode: "page", keys: [["z", "a"]] },
    // A page can't be changed, so each key Vim's nv_edit refuses says E21
    // at once, i a o entering no insert, d c and r once their motion or
    // letter comes (Operators.refused), and visual's over what's chosen;
    // q says how to close, as on the rows.
    { act: "cannotChange", mode: "page", keys: ["x", "p", "S", "D", "C", "X", "P", "s", "J",
        "~", "i", "a", "I", "A", "o", "O"] },
    { act: "cannotChange", mode: "pageVisual", keys: ["d", "x", "c", "p", "D", "C", "X",
        "P", "s", "S", "J", "~"] },
    { act: "quitSaid", mode: "page", keys: ["q"] },
    { act: "quitSaid", mode: "pageVisual", keys: ["q"] },
    // Visual on the page, the language's select and select-lines: v from
    // the cursor's letter, V by whole lines, and gv the last chosen again.
    // While choosing, its rows are pageVisual's, whose two kinds share them
    // (inMode): the same key again ends it and the other changes its kind,
    // as Vim's do; the page's moves extend it; o goes to its other end; y
    // copies it and Escape lets it go, each back on the page. No way back
    // to the rows is in it, as Vim's visual has no ZZ.
    { act: "select", mode: "page", keys: ["v"] },
    { act: "selectLines", mode: "page", keys: ["V"] },
    { act: "reselect", mode: "page", keys: [["g", "v"]] },
    { act: "select", mode: "pageVisual", keys: ["v"] },
    { act: "selectLines", mode: "pageVisual", keys: ["V"] },
    { act: "otherEnd", mode: "pageVisual", keys: ["o"] },
    { act: "copy", mode: "pageVisual", keys: ["y"] },
    // Y copies the whole lines it's on, however it chooses, as Vim's v_Y.
    { act: "copyLines", mode: "pageVisual", keys: ["Y"] },
    { act: "back", mode: "pageVisual", keys: ["Escape", "Ctrl+c"] }
];

// The acts that move through the rows, which repeat while their key is
// held, as Vim's motions do; every other act is done once a press, so a
// key let go late by a busy shell never does it twice (Prompt.qml).
const moves = ["next", "previous", "first", "last", "viewTop", "viewMiddle", "viewBottom",
    "halfDown", "halfUp", "pageDown", "pageUp", "left", "right", "scrollDown", "scrollUp",
    "wordNext", "wordBack", "wordEnd", "letterBack", "letterNext", "seekNext", "seekPrevious",
    "paragraphOn", "paragraphBack", "matchNext", "matchBack", "letterAgain", "letterAgainBack"];

function repeats(act) {
    return moves.includes(act);
}

// Whether a held key's repeat does its act again, and so is a key taken:
// where nothing types, only a move; where the line types, any act but
// Return's, which is once a press (Prompt.qml's returned). One that does
// nothing takes no key, so what its press said stands through the hold,
// as a Return held on Reboot leaves it armed (Y2YBTS, 2026-11-12).
function heldDoes(act, mode) {
    return still(mode) ? repeats(act) : act !== "run";
}

// Where the keys stand between presses: the count typed so far, as
// digits, and the keys of a sequence begun. An operator waiting for its
// motion stands as `operator`, { key, count }, its key and the count
// typed before it, the count after it being the standing's own.
function rest() {
    return { count: "", typed: [] };
}

// Whether anything's begun: a count, a sequence's first keys, an operator,
// a search being typed or f waiting for its letter.
function pending(standing) {
    return standing.count !== "" || standing.typed.length > 0
        || standing.operator !== undefined || standing.seek !== undefined
        || standing.letter !== undefined;
}

const digit = /^[0-9]$/;

function same(a, b) {
    return a.length === b.length && a.every((key, at) => key === b[at]);
}

function begins(sequence, typed) {
    return typed.length < sequence.length && typed.every((key, at) => key === sequence[at]);
}

// Whether a mode is one where nothing types, where counts and the command
// line are taken: normal and visual on the rows, and the drawer's page.
function still(mode) {
    return mode === "normal" || mode === "visual" || paged(mode);
}

// Whether the keys are in the drawer: on its page, or choosing on it by
// letters or by lines.
function paged(mode) {
    return mode === "page" || chosen(mode);
}
function chosen(mode) {
    return mode === "pageVisual" || mode === "pageVisualLine";
}

// Whether a row's keys are a mode's: "line" is the find's and insert's,
// where letters type, "both" every mode's. Choosing on the page, by letters or lines, has
// pageVisual's rows and the page's moves; visual on the rows has normal's
// moves besides its own, since every move extends it.
function inMode(row, mode) {
    if (chosen(mode)) {
        return row.mode === "pageVisual"
            || ((row.mode === "page" || row.mode === "both") && moves.includes(row.act));
    }
    if (mode === "visual" && row.mode === "normal") {
        return moves.includes(row.act);
    }
    if (mode === "visual" && row.mode === "object") {
        return true;
    }
    // An operator's text objects are theirs alone, never the find's keys.
    if (mode === "object") {
        return row.mode === "object";
    }
    return row.mode === mode || (row.mode === "line" && !still(mode)) || row.mode === "both";
}

// The rows a mode reads, in the order it reads them: in normal on a line
// holding a prompt's words, the words' own first.
function readIn(mode, words) {
    const own = table.filter(one => inMode(one, mode));
    return words && mode === "normal" ? table.filter(one => one.mode === "words").concat(own)
        : own;
}

// Whether a key typed so far is a whole sequence of a mode's, or begins
// one: the act it ends, if any, and whether it's begun another.
function matching(typed, mode, words) {
    let begun = false;
    for (const row of readIn(mode, words)) {
        for (const keys of row.keys) {
            const sequence = Array.isArray(keys) ? keys : [keys];
            if (same(sequence, typed)) {
                return { act: row.act, begun: false };
            }
            begun = begun || begins(sequence, typed);
        }
    }
    return { act: "", begun };
}

// What a key does in a mode from where the keys stand: the act, if one's
// done, with the count typed before it (0 for none), how the keys stand
// after it, and whether the key was the line's to act on at all, rather
// than the field's own to type. Escape only cancels what's begun, as Vim's
// does, and at rest in normal does nothing, saying nothing, read as
// `rested`; one meaning nothing else in normal is `idle`, said "not
// here". Words says the line holds a prompt's words rather than a find,
// which normal edits.
function read(standing, key, mode, words = false) {
    // A key with no name, as Shift pressed on its own for a capital,
    // changes nothing, so a sequence or a count begun carries on past it.
    if (key === "") {
        return { act: "", count: 0, standing, ours: false };
    }
    if (standing.seek !== undefined) {
        return Finding.seekRead(standing, key, rest());
    }
    // f and F's wait for a letter, and the letter (Finding.js).
    const letter = Finding.letterRead(standing, key, { mode, words }, rest());
    if (letter !== null) {
        return letter;
    }
    // In the drawer, / and ? at rest open its search, the way it goes,
    // keeping a count typed before them for the match it lands on.
    if (paged(mode) && (key === "/" || key === "?") && standing.typed.length === 0) {
        return { act: "", count: 0, standing: { count: standing.count, typed: [],
            seek: { pattern: "", forward: key === "/" } }, ours: true };
    }
    // A colon at rest where nothing types opens the command line, which
    // takes the keys from here (CommandLine.qml): in normal with a count
    // typed before it as its range of rows, in visual over the selection,
    // and in the drawer with none, since a count there counts lines, not
    // rows. One while an operator waits lets the operator go, as no motion.
    if (still(mode) && key === ":" && standing.typed.length === 0
        && standing.operator === undefined) {
        const count = mode === "normal" && standing.count !== "" ? Number(standing.count) : 0;
        return { act: "command", count, standing: rest(), ours: true };
    }
    // At rest in normal too, closing nothing (Joel, 2026-11-05: "fuck escape
    // quitting"), and never told "not here": his Neovim's bell is off, and
    // he presses it out of habit to be sure of normal.
    if (key === "Escape" && (pending(standing) || mode === "normal")) {
        const rested = mode === "normal" && !pending(standing);
        return { act: "", count: 0, standing: rest(), ours: true, rested };
    }
    // Normal's and visual's counts and operators, which keep an operator's
    // count apart from its motion's.
    const before = mode === "normal" || mode === "visual"
        ? Operators.beforeTable(standing, key, { mode, words }, { matching, rested: rest() })
        : null;
    const refused = mode === "page"
        ? Operators.refused(standing, key, { matching, rested: rest() }) : null;
    if (before !== null || refused !== null) {
        return before ?? refused;
    }
    // A count before a move in the drawer, choosing or not, where nothing
    // types: 0 alone is Vim's first column, which rows haven't, so it
    // counts only after another digit.
    if (paged(mode) && standing.typed.length === 0 && digit.test(key)
        && (standing.count !== "" || key !== "0")) {
        return { act: "", count: 0, standing: { count: standing.count + key, typed: [] },
            ours: true };
    }
    const typed = standing.typed.concat([key]);
    const count = standing.count === "" ? 0 : Number(standing.count);
    const { act, begun } = matching(typed, mode, words);
    if (act !== "") {
        return { act, count, standing: rest(), ours: true };
    }
    if (begun) {
        return { act: "", count: 0, standing: { count: standing.count, typed }, ours: true };
    }
    // Where nothing types, a key that ends nothing is the line's and does
    // nothing, letting what was begun go, as vi does.
    return { act: "", count: 0, standing: rest(), ours: still(mode), idle: still(mode) };
}

// Whether a key opens the command line: a colon as the find's first
// letter, as Vim's from anywhere; after letters it's the find's.
function opensCommands(key, mode, text) {
    return key === ":" && mode === "find" && text === "";
}

// The first key of an act in a mode, as the table names it, for the field
// to say beside what it does.
function keyOf(act, mode) {
    const row = table.find(one => one.act === act && inMode(one, mode));
    const keys = row ? row.keys[0] : "";
    return Array.isArray(keys) ? keys.join("") : keys;
}

// Whether an act in visual takes its selection: d, y and Return.
function takesSelection(act) {
    return act === "ending" || act === "copy" || act === "run";
}

// The mode an act leaves the line in, typing going back to `typing`'s.
function modeAfter(act, mode, typing = "find") {
    if (act === "normal" || act === "rows") {
        return "normal";
    }
    // V on the rows begins visual line there and ends it; in the drawer
    // it chooses the page's lines (below).
    if (act === "selectLines" && !paged(mode)) {
        return mode === "visual" ? "normal" : "visual";
    }
    if (act === "reselect" && !paged(mode)) {
        return "visual";
    }
    // Visual's d, y and Return take the selection and end it, as Vim's
    // visual operators do; its colon keeps it behind the command line.
    if (mode === "visual" && takesSelection(act)) {
        return "normal";
    }
    if (act === "drawer") {
        return "page";
    }
    // v and V choose by their kind, the same again ending it and the other
    // changing it; a copy or Escape ends it on the page.
    if (act === "select") {
        return mode === "pageVisual" ? "page" : "pageVisual";
    }
    if (act === "selectLines") {
        return mode === "pageVisualLine" ? "page" : "pageVisualLine";
    }
    if (["copy", "copyLines", "back", "cannotChange"].includes(act) && chosen(mode)) {
        return "page";
    }
    if (["insert", "append", "insertStart", "appendEnd", "change", "changeWord", "freshFind",
        "freshFindBack"].includes(act)) {
        return typing;
    }
    return mode;
}
