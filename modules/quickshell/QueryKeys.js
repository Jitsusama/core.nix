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
    // ex line's (exRead, below).
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
    { act: "drawerDown", mode: "both", keys: ["Shift+Down"] }
];

// The acts that move through the rows, which repeat while their key is
// held, as Vim's motions do; every other act is done once a press, so a
// key let go late by a busy shell never does it twice (Prompt.qml).
const moves = ["next", "previous", "first", "last", "viewTop", "viewMiddle", "viewBottom",
    "halfDown", "halfUp", "pageDown", "pageUp", "drawerUp", "drawerDown"];

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

// Whether anything's begun: a count, a sequence's first keys, an
// operator or an ex line.
function pending(standing) {
    return standing.count !== "" || standing.typed.length > 0 || standing.ex !== undefined
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
// Backspace does on an empty line, and anything printable is typed, as a
// space is.
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
    if (key === "Space") {
        return open(`${line} `);
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

function inMode(row, mode) {
    return row.mode === mode || row.mode === "both";
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

// What normal reads before its table, or null for a key it doesn't. A
// colon at rest opens the ex line, a count before it let go, since the
// rows take no ranges. A count is typed where nothing types: 0 alone is
// Vim's first column, which rows haven't, so it counts only after another
// digit, and one after an operator is its motion's. An operator's key at
// rest begins it, with the count before it.
function beforeTable(standing, key) {
    const atRest = standing.typed.length === 0;
    if (key === ":" && atRest && standing.operator === undefined) {
        return { act: "", count: 0, standing: { count: "", typed: [], ex: "" }, ours: true };
    }
    if (atRest && digit.test(key) && (standing.count !== "" || key !== "0")) {
        return { act: "", count: 0, standing: Object.assign({}, standing,
            { count: standing.count + key, typed: [] }), ours: true };
    }
    if (standing.operator !== undefined) {
        return operated(standing, key);
    }
    if (atRest && key in operators) {
        const operator = { key, count: standing.count };
        return { act: "", count: 0, standing: { count: "", typed: [], operator }, ours: true };
    }
    return null;
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
    if (key === "Escape" && pending(standing)) {
        return { act: "", count: 0, standing: rest(), ours: true };
    }
    const before = mode === "normal" ? beforeTable(standing, key) : null;
    if (before !== null) {
        return before;
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
    // and does nothing, letting what was begun go, as vi does.
    return { act: "", count: 0, standing: rest(), ours: mode === "normal" };
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

// The keys an act is typed by in a mode as Vim's own letters, j rather
// than the down arrow, for the foot to say what an operator took; its
// first key where it has none.
function lettersOf(act, mode) {
    const row = table.find(one => one.act === act && inMode(one, mode));
    const letters = (row?.keys ?? []).find(key => [].concat(key).every(one => one.length === 1));
    return letters === undefined ? keyOf(act, mode) : [].concat(letters).join("");
}

// Whether a key does an act in a mode, so a hint names a key only where
// pressing it does what the hint says.
function binds(act, key, mode) {
    return table.some(one => one.act === act && inMode(one, mode) && one.keys.includes(key));
}

// The mode an act leaves the line in.
function modeAfter(act, mode) {
    if (act === "normal") {
        return "normal";
    }
    if (["insert", "append", "insertStart", "appendEnd", "change"].includes(act)) {
        return "insert";
    }
    return mode;
}
