// The keys of the launcher's query line, in its two modes, as one table of
// key and act (docs/design.md, Vi's Keys), read apart from the field
// (Prompt.qml) so tests/query-keys.qml can hold it in Qt's own engine, and
// so the desktop's one language of keys can take it in later. The line is
// Neovim's command line, the rows a buffer below it: it opens in insert,
// where letters type, and Escape gives normal, where nothing types.
// Letters go by what they type, wherever they sit on the layout.
.pragma library

// Each act, the mode it's done in and the keys that do it. A key is named as
// the field names it (Prompt.qml, `named`): a modifier, then the letter it
// typed, or else its name, with Shift unnamed beside a letter, which says it
// by its case. A list of keys is a sequence typed one after another, as vi's
// cc. A key a mode doesn't name is the field's own in insert, where it types,
// and does nothing in normal. Where one differs from Neovim, the reason is
// beside it.
const table = [
    // Insert, as Vim's: the caret's moves and deleting before it.
    { act: "caretLeft", mode: "insert", keys: ["Left"] },
    { act: "caretRight", mode: "insert", keys: ["Right"] },
    { act: "caretStart", mode: "insert", keys: ["Home"] },
    { act: "caretEnd", mode: "insert", keys: ["End"] },
    { act: "deleteBack", mode: "insert", keys: ["Backspace", "Ctrl+h"] },
    { act: "deleteWord", mode: "insert", keys: ["Ctrl+w"] },
    { act: "deleteToStart", mode: "insert", keys: ["Ctrl+u"] },
    // The rows below are his completion menu's (modules/neovim/keys.nix):
    // the next and the one before, and Ctrl+Y takes the selected one into
    // the line, as the grey completion after what's typed shows it.
    { act: "next", mode: "insert", keys: ["Down", "Tab", "Ctrl+n"] },
    { act: "previous", mode: "insert", keys: ["Up", "Shift+Tab", "Ctrl+p"] },
    { act: "complete", mode: "insert", keys: ["Ctrl+y"] },
    { act: "normal", mode: "insert", keys: ["Escape"] },

    // Normal, over the rows as a buffer: Vim's moves, each taking a count
    // typed before it (3j). gg and {n}G go to the first row and the nth, G
    // to the last; H M L to the top, middle and bottom row in view; Ctrl+D
    // and Ctrl+U half a screen, Ctrl+F and Ctrl+B, as Page Down and Page
    // Up, a screen (moved, below).
    { act: "next", mode: "normal", keys: ["Down", "j"] },
    { act: "previous", mode: "normal", keys: ["Up", "k"] },
    { act: "first", mode: "normal", keys: [["g", "g"]] },
    { act: "last", mode: "normal", keys: ["G"] },
    { act: "viewTop", mode: "normal", keys: ["H"] },
    { act: "viewMiddle", mode: "normal", keys: ["M"] },
    { act: "viewBottom", mode: "normal", keys: ["L"] },
    { act: "halfDown", mode: "normal", keys: ["Ctrl+d"] },
    { act: "halfUp", mode: "normal", keys: ["Ctrl+u"] },
    { act: "pageDown", mode: "normal", keys: ["Ctrl+f"] },
    { act: "pageUp", mode: "normal", keys: ["Ctrl+b"] },
    // The left and right arrows step out and in, with h and l beside them,
    // and - and Backspace step out as his oil's do; a query typed is
    // cleared first, and u brings it back, as vi's undo does.
    { act: "into", mode: "normal", keys: ["Right", "l"] },
    { act: "out", mode: "normal", keys: ["Left", "h", "-", "Backspace"] },
    // q closes, as his Neovim closes help and its plugin windows, ZZ as it
    // closes a buffer and Space w as his leader closes a split; :q is the
    // command line's (CommandLine.qml), which : opens (read, below).
    { act: "close", mode: "normal", keys: ["q", ["Z", "Z"], ["Space", "w"]] },
    // Tab goes back to the place used last, and again to the one before
    // that, as his Space Tab goes back to the file used last (docs/
    // design.md, Vi's Keys: Tab stays ours, and Space Tab does it too).
    { act: "lastPlace", mode: "normal", keys: ["Tab", ["Space", "Tab"]] },
    // Every key here, as his leader's Space ? lists them, and F1 as help.
    // Space alone begins these and does nothing, waiting as gg's g does.
    { act: "keys", mode: "normal", keys: [["Space", "?"], "F1"] },
    { act: "restore", mode: "normal", keys: ["u"] },
    // Vim's operators on the rows as lines: d does each row's ending
    // action (Forget It, Close It) and y its copy, over the row again as
    // dd and yy, or as far as a motion goes (operated, below), so these
    // rows name only the doubled keys, for saying them.
    { act: "ending", mode: "normal", keys: [["d", "d"]] },
    { act: "copy", mode: "normal", keys: [["y", "y"]] },
    // Into insert: at the caret, after it, at the line's start and its end,
    // or with the line cleared first.
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
    // Space and the right arrow go into the drawer beside the rows, as his
    // Space and the arrows go to the window that way (the language's
    // window-right): its keys are then a page's, below.
    { act: "drawer", mode: "normal", keys: [["Space", "Right"]] },
    // Escape steps back one level, and closes the launcher at its home.
    { act: "back", mode: "normal", keys: ["Escape"] },

    // A prompt's words in normal, as Vim edits a line: w, b and e to the
    // next word's start, back to a word's start and on to its end, 0 and $
    // to the line's ends, x the letter under the caret, dw and cw the word
    // it's on, cw going into insert, and u back to before the last change.
    // They're read in normal only while the line holds a prompt's words
    // rather than a find (Prompt.qml, `words`), and before normal's own,
    // so u there undoes rather than bringing a find back.
    // h and l move the caret a letter, as Vim's do, rather than stepping
    // out of the place and into the row, and so do \u2190 and \u2192, each its
    // letter in normal; - and Backspace still step out.
    { act: "letterBack", mode: "words", keys: ["h", "Left"] },
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

    // Both modes: Return does the row or steps into it, as everywhere of
    // ours, with Shift below the window in focus and with Ctrl closing
    // whatever it opens; and Page Up and Page Down page the rows. Shift
    // and the arrows do nothing: the drawer is gone into with Space and
    // the right arrow and moved through there, as an arrow is its letter
    // and has no act of its own (VKCF3Z's spec v2, The Arrows).
    { act: "run", mode: "both", keys: ["Return", "Enter", "Shift+Return", "Shift+Enter",
        "Ctrl+Return", "Ctrl+Enter"] },
    { act: "pageUp", mode: "both", keys: ["PageUp"] },
    { act: "pageDown", mode: "both", keys: ["PageDown"] },

    // The drawer, gone into: the preview window, a page in normal with a
    // cursor on its lines (DrawerPage.js), as the reader's page and his
    // Neovim's windows have them. j k and the arrows move it a line and h l along one, the
    // view kept about it; Ctrl+E and Ctrl+Y scroll the view a line, Ctrl+D
    // and Ctrl+U half the view, Ctrl+F and Ctrl+B a view, with Page Down
    // and Page Up (both modes', above); gg G and H M L as on the rows, each
    // taking a count. Space and the left arrow go back to the rows, as his
    // window-left does, and so do ZZ, Space w and :q, as his Neovim
    // closes a window. Escape only lets a count or a waiting key go, as
    // it does nothing at rest in a window, and q alone does nothing, as
    // Joel asked of every modal surface (2026-11-04). Return does the row
    // the drawer's about.
    { act: "next", mode: "page", keys: ["Down", "j"] },
    { act: "previous", mode: "page", keys: ["Up", "k"] },
    { act: "left", mode: "page", keys: ["Left", "h"] },
    { act: "right", mode: "page", keys: ["Right", "l"] },
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
    { act: "rows", mode: "page", keys: [["Space", "Left"], ["Z", "Z"], ["Space", "w"]] }
];

// The acts that move through the rows, which repeat while their key is
// held, as Vim's motions do; every other act is done once a press, so a
// key let go late by a busy shell never does it twice (Prompt.qml).
const moves = ["next", "previous", "first", "last", "viewTop", "viewMiddle", "viewBottom",
    "halfDown", "halfUp", "pageDown", "pageUp", "left", "right", "scrollDown", "scrollUp",
    "wordNext", "wordBack", "wordEnd", "letterBack", "letterNext"];

function repeats(act) {
    return moves.includes(act);
}

// Where the keys stand between presses: the count typed so far, as
// digits, and the keys of a sequence begun. An operator waiting for its
// motion stands as `operator`, { key, count }, its key and the count
// typed before it, the count after it being the standing's own.
function rest() {
    return { count: "", typed: [] };
}

// Whether anything's begun: a count, a sequence's first keys or an
// operator.
function pending(standing) {
    return standing.count !== "" || standing.typed.length > 0
        || standing.operator !== undefined;
}

// The operators, by their key, and the act each does over the rows it
// takes.
const operators = { d: "ending", y: "copy" };

// The moves an operator takes as its motion, each linewise on the rows,
// as Vim's j k gg G H M L are on lines. Ctrl+D, Ctrl+U and the screens
// scroll rather than move, so Vim takes none of them as a motion.
const motions = ["next", "previous", "first", "last", "viewTop", "viewMiddle", "viewBottom"];

// Two counts as one, as Vim multiplies the count before an operator and
// the one after it: 2d3j goes six rows. Either alone is itself, and
// neither is no count.
function multiplied(before, after) {
    if (before === "" && after === "") {
        return 0;
    }
    return Number(before === "" ? 1 : before) * Number(after === "" ? 1 : after);
}

// A key while an operator waits: the operator's own key again does it
// over the row and those below, as dd does; a motion does it as far as
// that goes, as dj does, with the counts multiplied; a sequence's first
// key (gg's g) waits for the rest; anything else lets it go. It waits as
// long as it takes for its motion, as Vim's does.
function operated(standing, key) {
    const { operator } = standing;
    const act = operators[operator.key];
    const count = multiplied(operator.count, standing.count);
    if (standing.typed.length === 0 && key === operator.key) {
        return { act, count, motion: null, standing: rest(), ours: true };
    }
    // Another operator's key lets this one go, as dy does in Vim.
    const typed = standing.typed.concat([key]);
    const matched = key in operators ? { act: "", begun: false } : matching(typed, "normal");
    if (motions.includes(matched.act)) {
        return { act, count: 0, motion: { act: matched.act, count }, standing: rest(),
            ours: true };
    }
    if (matched.act === "" && matched.begun) {
        return { act: "", count: 0, standing: { count: standing.count, typed, operator },
            ours: true };
    }
    return { act: "", count: 0, standing: rest(), ours: true };
}

// The range a count before a colon gives the command line, as Vim's 3:
// gives :.,.+2 and 1: gives :., or none (Ex.js's counted).
function counted(count) {
    return count > 1 ? `.,.+${count - 1}` : count === 1 ? "." : "";
}

const digit = /^[0-9]$/;

function same(a, b) {
    return a.length === b.length && a.every((key, at) => key === b[at]);
}

function begins(sequence, typed) {
    return typed.length < sequence.length && typed.every((key, at) => key === sequence[at]);
}

// Whether a mode is one where nothing types, where counts and the command
// line are taken: normal on the rows, and the drawer's page.
function still(mode) {
    return mode === "normal" || mode === "page";
}

// Whether a row's keys are a mode's: "both" is the line's two modes,
// insert and normal, and the drawer's page.
function inMode(row, mode) {
    return row.mode === mode || row.mode === "both";
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

// What normal reads before its table, or null for a key it doesn't. A
// count is typed where nothing types: 0 alone is Vim's first column,
// which rows haven't, so it counts only after another digit, and one after
// an operator is its motion's. An operator's key at rest begins it, with
// the count before it, but on a prompt's words (`words`), where d and c
// are the words' own, as dw and cw.
function beforeTable(standing, key, words) {
    const atRest = standing.typed.length === 0;
    if (atRest && digit.test(key) && (standing.count !== "" || key !== "0")) {
        return { act: "", count: 0, standing: Object.assign({}, standing,
            { count: standing.count + key, typed: [] }), ours: true };
    }
    if (standing.operator !== undefined) {
        return operated(standing, key);
    }
    if (atRest && !words && key in operators) {
        const operator = { key, count: standing.count };
        return { act: "", count: 0, standing: { count: "", typed: [], operator }, ours: true };
    }
    return null;
}

// What a key does in a mode from where the keys stand: the act, if one's
// done, with the count typed before it (0 for none), how the keys stand
// after it, and whether the key was the line's to act on at all, rather
// than the field's own to type. Escape only cancels what's begun, as Vim's
// does, and steps back out of anything on the next press. Words says the
// line holds a prompt's words rather than a find, which normal edits.
function read(standing, key, mode, words = false) {
    // A key with no name, as Shift pressed on its own for a capital,
    // changes nothing, so a sequence or a count begun carries on past it.
    if (key === "") {
        return { act: "", count: 0, standing, ours: false };
    }
    // A colon at rest where nothing types opens the command line, which
    // takes the keys from here (CommandLine.qml): in normal with a count
    // typed before it as its range of rows, and in the drawer with none,
    // since a count there counts lines, not rows. One while an operator
    // waits lets the operator go, as no motion.
    if (still(mode) && key === ":" && standing.typed.length === 0
        && standing.operator === undefined) {
        const count = mode === "normal" && standing.count !== "" ? Number(standing.count) : 0;
        return { act: "command", count, standing: rest(), ours: true };
    }
    if (key === "Escape" && pending(standing)) {
        return { act: "", count: 0, standing: rest(), ours: true };
    }
    // Normal's counts and operators, which keep an operator's count apart
    // from its motion's.
    const before = mode === "normal" ? beforeTable(standing, key, words) : null;
    if (before !== null) {
        return before;
    }
    // A count before a move in the drawer, where nothing types: 0 alone is
    // Vim's first column, which rows haven't, so it counts only after
    // another digit.
    if (mode === "page" && standing.typed.length === 0 && digit.test(key)
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
    // In normal and the drawer nothing types, so a key that ends nothing
    // is the line's and does nothing, letting what was begun go, as vi
    // does.
    return { act: "", count: 0, standing: rest(), ours: still(mode) };
}

// How the keys stand once a sequence begun has waited its timeoutlen for
// the rest: its keys let go, and a count typed before them kept, since a
// count waits for its move as long as it takes, as Vim's does.
function waited(standing) {
    const kept = { count: standing.count, typed: [] };
    return standing.operator === undefined ? kept
        : Object.assign(kept, { operator: standing.operator });
}

// Where a move takes the selection, from where the rows stand: the row
// selected, the highest it may go (-1 where an answer's headline sits
// above the rows) and the last, the first and last rows wholly in view,
// and how many rows a screen holds. A count moves that many times, or,
// with gg and G, goes to that row, counted from 1, as a line's number is.
// H and L count from the view's top and bottom. Ctrl+D and Ctrl+U move
// half the rows in view, or a count's rows, as Vim's 'scroll' takes it.
// Every move stops at the first and last rows, never going round.
function moved(act, count, place) {
    const times = Math.max(1, count);
    const within = at => Math.max(place.highest, Math.min(at, place.last));
    const half = count > 0 ? count : Math.max(1, Math.floor((place.bottom - place.top + 1) / 2));
    switch (act) {
    case "next":
        return within(place.at + times);
    case "previous":
        return within(place.at - times);
    case "first":
        return count > 0 ? within(count - 1) : place.highest;
    case "last":
        return count > 0 ? within(count - 1) : within(place.last);
    case "viewTop":
        return within(Math.min(place.top + times - 1, place.bottom));
    case "viewMiddle":
        return within(place.top + Math.floor((place.bottom - place.top) / 2));
    case "viewBottom":
        return within(Math.max(place.bottom - times + 1, place.top));
    case "halfDown":
        return within(place.at + half);
    case "halfUp":
        return within(place.at - half);
    case "pageDown":
        return within(place.at + times * place.screen);
    case "pageUp":
        return within(place.at - times * place.screen);
    default:
        return place.at;
    }
}

// The first key of an act in a mode, as the table names it, for the field
// to say beside what it does.
function keyOf(act, mode) {
    const row = table.find(one => one.act === act && inMode(one, mode));
    const keys = row ? row.keys[0] : "";
    return Array.isArray(keys) ? keys.join("") : keys;
}

// Whether a key does an act in a mode, so a hint names a key only where
// pressing it does what the hint says.
function binds(act, key, mode) {
    return table.some(one => one.act === act && inMode(one, mode) && one.keys.includes(key));
}

// The mode an act leaves the line in.
function modeAfter(act, mode) {
    if (act === "normal" || act === "rows") {
        return "normal";
    }
    if (act === "drawer") {
        return "page";
    }
    if (["insert", "append", "insertStart", "appendEnd", "change", "changeWord"]
        .includes(act)) {
        return "insert";
    }
    return mode;
}
