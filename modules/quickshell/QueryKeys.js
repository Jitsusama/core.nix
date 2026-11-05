// The keys of the launcher's query line, in its two modes, as one table of
// key and act (docs/design.md, Vi's Keys), read apart from the field
// (Prompt.qml) so tests/query-keys.qml can hold it in Qt's own engine, and
// so the desktop's one language of keys can take it in later. The line is
// Neovim's command line, the rows a buffer below it: it opens in insert,
// where letters type, and Escape gives normal, where nothing types, and
// where / and ? take a search typed on the line, as his command line does.
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
    { act: "deleteBack", mode: "insert", keys: ["Backspace"] },
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
    // typed before it (3j). Tab goes on as in insert until the place used
    // last takes it. gg and {n}G go to the first row and the nth, G to the
    // last; H M L to the top, middle and bottom row in view; Ctrl+D and
    // Ctrl+U half a screen, Ctrl+F and Ctrl+B, as Page Down and Page Up, a
    // screen (moved, below).
    { act: "next", mode: "normal", keys: ["Down", "j", "Tab"] },
    { act: "previous", mode: "normal", keys: ["Up", "k", "Shift+Tab"] },
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
    // q closes, as his Neovim closes help and its plugin windows, and ZZ
    // as it closes a buffer; :q is the ex line's (exRead, below).
    { act: "close", mode: "normal", keys: ["q", ["Z", "Z"]] },
    { act: "restore", mode: "normal", keys: ["u"] },
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
    // A search of the rows by their words, as Vim's / and ? search a
    // buffer, typed on the line as his command line takes one (search,
    // below); n and N go on to the next match, the way it went and the
    // other, each a move, so a held n repeats as j does.
    { act: "searchForward", mode: "normal", keys: ["/"] },
    { act: "searchBack", mode: "normal", keys: ["?"] },
    { act: "searchNext", mode: "normal", keys: ["n"] },
    { act: "searchPrevious", mode: "normal", keys: ["N"] },
    // Escape steps back one level, and closes the launcher at its home.
    { act: "back", mode: "normal", keys: ["Escape"] },

    // Both modes: Return does the row or steps into it, as everywhere of
    // ours, with Shift below the window in focus and with Ctrl closing
    // whatever it opens; Page Up and Page Down page the rows; and with
    // Shift the arrows scroll the drawer, as his picker's preview keys do.
    { act: "run", mode: "both", keys: ["Return", "Enter", "Shift+Return", "Shift+Enter",
        "Ctrl+Return", "Ctrl+Enter"] },
    { act: "pageUp", mode: "both", keys: ["PageUp"] },
    { act: "pageDown", mode: "both", keys: ["PageDown"] },
    { act: "drawerUp", mode: "both", keys: ["Shift+Up"] },
    { act: "drawerDown", mode: "both", keys: ["Shift+Down"] },

    // A search being typed, as Neovim's command line takes one: letters
    // and Space type, and its keys edit as insert's do. Return runs it, and
    // Escape or Ctrl+C cancels it, as Backspace does with nothing left to
    // take (typedInSearch).
    { act: "caretLeft", mode: "search", keys: ["Left"] },
    { act: "caretRight", mode: "search", keys: ["Right"] },
    { act: "caretStart", mode: "search", keys: ["Home"] },
    { act: "caretEnd", mode: "search", keys: ["End"] },
    { act: "deleteBack", mode: "search", keys: ["Backspace"] },
    { act: "deleteWord", mode: "search", keys: ["Ctrl+w"] },
    { act: "deleteToStart", mode: "search", keys: ["Ctrl+u"] },
    { act: "find", mode: "search", keys: ["Return", "Enter"] },
    { act: "cancel", mode: "search", keys: ["Escape", "Ctrl+c"] }
];

// The acts that move through the rows, which repeat while their key is
// held, as Vim's motions do; every other act is done once a press, so a
// key let go late by a busy shell never does it twice (Prompt.qml).
const moves = ["next", "previous", "first", "last", "viewTop", "viewMiddle", "viewBottom",
    "halfDown", "halfUp", "pageDown", "pageUp", "drawerUp", "drawerDown", "searchNext",
    "searchPrevious"];

function repeats(act) {
    return moves.includes(act);
}

// Where the keys stand between presses: the count typed so far, as
// digits, and the keys of a sequence begun.
function rest() {
    return { count: "", typed: [] };
}

// Whether anything's begun: a count, a sequence's first keys or an ex
// line.
function pending(standing) {
    return standing.count !== "" || standing.typed.length > 0 || standing.ex !== undefined;
}

// The ex line, Vim's command line after a colon, which waits for its
// Return as long as it takes: what's typed after the colon, held in how
// the keys stand as `ex` while it's open. A line of digits goes to that
// row, as :12 goes to line 12; Vim's ways of quitting close, since the
// launcher writes nothing, so :wq and :x close as :q does; anything else
// does nothing, as an ex line the launcher doesn't know.
const quits = ["q", "quit", "q!", "quit!", "qa", "qall", "qa!", "wq", "x", "xit"];
const printable = /^[!-~]$/;
const digits = /^[0-9]+$/;

function exRan(line) {
    const said = line.trim();
    if (digits.test(said)) {
        return { act: "first", count: Math.max(1, Number(said)) };
    }
    return { act: quits.includes(said) ? "close" : "", count: 0 };
}

// A key typed on the ex line: Return runs it, Escape lets it go, as
// Backspace does on an empty line, and anything printable is typed.
function exRead(standing, key) {
    const line = standing.ex;
    const open = ex => ({ act: "", count: 0, standing: { count: "", typed: [], ex },
        ours: true });
    if (key === "Return" || key === "Enter") {
        return Object.assign(exRan(line), { standing: rest(), ours: true });
    }
    if (key === "Escape" || (key === "Backspace" && line === "")) {
        return { act: "", count: 0, standing: rest(), ours: true };
    }
    if (key === "Backspace") {
        return open(line.slice(0, -1));
    }
    return open(printable.test(key) ? line + key : line);
}

const digit = /^[0-9]$/;

function same(a, b) {
    return a.length === b.length && a.every((key, at) => key === b[at]);
}

function begins(sequence, typed) {
    return typed.length < sequence.length && typed.every((key, at) => key === sequence[at]);
}

// Whether a row's keys are a mode's: "both" is the line's two modes,
// insert and normal, and a search being typed has only its own.
function inMode(row, mode) {
    return row.mode === mode || (row.mode === "both" && mode !== "search");
}

// Whether a key typed so far is a whole sequence of a mode's, or begins
// one: the act it ends, if any, and whether it's begun another.
function matching(typed, mode) {
    let begun = false;
    for (const row of table.filter(one => inMode(one, mode))) {
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
// does, and steps back out of anything on the next press.
function read(standing, key, mode) {
    // A key with no name, as Shift pressed on its own for a capital,
    // changes nothing, so a sequence or a count begun carries on past it.
    if (key === "") {
        return { act: "", count: 0, standing, ours: false };
    }
    if (standing.ex !== undefined) {
        return exRead(standing, key);
    }
    // A colon at rest in normal opens the ex line; a count before it is
    // let go, since the rows take no ranges.
    if (mode === "normal" && key === ":" && standing.typed.length === 0) {
        return { act: "", count: 0, standing: { count: "", typed: [], ex: "" }, ours: true };
    }
    if (key === "Escape" && pending(standing)) {
        return { act: "", count: 0, standing: rest(), ours: true };
    }
    // A count before a move, in normal, where nothing types: 0 alone is
    // Vim's first column, which rows haven't, so it counts only after
    // another digit.
    if (mode === "normal" && standing.typed.length === 0 && digit.test(key)
        && (standing.count !== "" || key !== "0")) {
        return { act: "", count: 0, standing: { count: standing.count + key, typed: [] },
            ours: true };
    }
    const typed = standing.typed.concat([key]);
    const count = standing.count === "" ? 0 : Number(standing.count);
    const { act, begun } = matching(typed, mode);
    if (act !== "") {
        return { act, count, standing: rest(), ours: true };
    }
    if (begun) {
        return { act: "", count: 0, standing: { count: standing.count, typed }, ours: true };
    }
    // In normal nothing types, so a key that ends nothing is the line's
    // and does nothing, letting what was begun go, as vi does. A search
    // being typed is the line's too, typed by typedInSearch rather than
    // into the field, which holds the query meanwhile.
    return { act: "", count: 0, standing: rest(), ours: mode !== "insert" };
}

// A search being typed, as { pattern, at }, the caret's place in it, after
// an act of the search's, or the letters a key typed where it ends none:
// the line after it, and whether it ends the typing, "found" to run it or
// "cancelled" to put back what the field held, as Vim's / does with
// Return, and with Escape or a Backspace with nothing left to take.
function typedInSearch(line, act, letters) {
    const { pattern, at } = line;
    const put = (text, caret) => ({ pattern: text, at: caret, ends: "" });
    switch (act) {
    case "find":
        return { pattern, at, ends: "found" };
    case "cancel":
        return { pattern, at, ends: "cancelled" };
    case "deleteBack":
        if (pattern === "") {
            return { pattern, at, ends: "cancelled" };
        }
        return at === 0 ? put(pattern, at)
            : put(pattern.slice(0, at - 1) + pattern.slice(at), at - 1);
    case "deleteWord": {
        const start = wordStart(pattern, at);
        return put(pattern.slice(0, start) + pattern.slice(at), start);
    }
    case "deleteToStart":
        return put(pattern.slice(at), 0);
    case "caretLeft":
        return put(pattern, Math.max(0, at - 1));
    case "caretRight":
        return put(pattern, Math.min(pattern.length, at + 1));
    case "caretStart":
        return put(pattern, 0);
    case "caretEnd":
        return put(pattern, pattern.length);
    case "":
        // A key that types a character, as a letter or Space does; one
        // that types nothing, or a control character, changes nothing.
        if (letters === "" || letters < " " || letters === "\u007f") {
            return put(pattern, at);
        }
        return put(pattern.slice(0, at) + letters + pattern.slice(at), at + letters.length);
    default:
        return put(pattern, at);
    }
}

// How the keys stand once a sequence begun has waited its timeoutlen for
// the rest: its keys let go, and a count typed before them kept, since a
// count waits for its move as long as it takes, as Vim's does.
function waited(standing) {
    return { count: standing.count, typed: [] };
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

// What Vim's iskeyword counts as a word's: letters, digits, the underscore
// and the accented letters past them.
const wordLetter = /[0-9A-Za-z_\u00c0-\u024f]/;
const space = /\s/;
const notSpace = /\S/;

function wordly(letter) {
    return wordLetter.test(letter);
}

// Where the word before the caret starts, as Vim's Ctrl+W deletes back to:
// past the spaces before it, then a run of a word's letters, or else a run
// of anything else but spaces.
function wordStart(text, at) {
    let start = at;
    while (start > 0 && space.test(text[start - 1])) {
        start -= 1;
    }
    if (start === 0) {
        return 0;
    }
    const kind = wordly(text[start - 1]);
    while (start > 0 && !space.test(text[start - 1]) && wordly(text[start - 1]) === kind) {
        start -= 1;
    }
    return start;
}

// Where the caret goes as a mode is entered, as Vim moves it: one left
// leaving insert, since normal's block sits on a letter and insert's bar
// between two; at it, after it, at the first letter or at the end, going
// into insert.
function caretAfter(act, text, at) {
    switch (act) {
    case "normal":
        return Math.max(0, at - 1);
    case "append":
        return Math.min(text.length, at + 1);
    case "insertStart": {
        const first = text.search(notSpace);
        return first < 0 ? text.length : first;
    }
    case "appendEnd":
        return text.length;
    default:
        return at;
    }
}

// The mode an act leaves the line in.
function modeAfter(act, mode) {
    if (act === "searchForward" || act === "searchBack") {
        return "search";
    }
    if (act === "find" || act === "cancel") {
        return "normal";
    }
    if (act === "normal") {
        return "normal";
    }
    if (["insert", "append", "insertStart", "appendEnd", "change"].includes(act)) {
        return "insert";
    }
    return mode;
}

// Where the caret may rest in normal: on a letter, never past the last.
function resting(text, at) {
    return Math.max(0, Math.min(at, text.length - 1));
}
