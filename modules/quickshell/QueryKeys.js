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

    // Normal, until its own moves come: the rows' keys the line had before
    // it had modes. Tab goes on as in insert until the place used last
    // takes it. The left and right arrows step out and in, with h and l
    // beside them, and Backspace steps out as his oil's does; a query typed
    // is cleared first, and u brings it back, as vi's undo does.
    { act: "next", mode: "normal", keys: ["Down", "Tab"] },
    { act: "previous", mode: "normal", keys: ["Up", "Shift+Tab"] },
    { act: "into", mode: "normal", keys: ["Right", "l"] },
    { act: "out", mode: "normal", keys: ["Left", "h", "Backspace"] },
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

// Where the keys stand between presses: the keys of a sequence begun.
function rest() {
    return { typed: [] };
}

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

// What a key does in a mode from where the keys stand: the act, if one's
// done, how the keys stand after it, and whether the key was the line's to
// act on at all, rather than the field's own to type. Escape always cancels
// what's begun before it steps back out of anything.
function read(standing, key, mode) {
    // A key with no name, as Shift pressed on its own for a capital,
    // changes nothing, so a sequence begun carries on past it.
    if (key === "") {
        return { act: "", standing, ours: false };
    }
    const typed = standing.typed.concat([key]);
    if (key === "Escape" && standing.typed.length > 0) {
        return { act: "", standing: rest(), ours: true };
    }
    const { act, begun } = matching(typed, mode);
    if (act !== "") {
        return { act, standing: rest(), ours: true };
    }
    if (begun) {
        return { act: "", standing: { typed }, ours: true };
    }
    // In normal nothing types, so a key that ends nothing is the line's
    // and does nothing, letting what was begun go, as vi does.
    return { act: "", standing: rest(), ours: mode === "normal" };
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
