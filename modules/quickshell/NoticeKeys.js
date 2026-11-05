// The keys of the notices' stack while it holds them, as one table of key
// and act (docs/design.md, Vi's Keys), read apart from the stack
// (Notifications.qml) so tests/notice-keys.qml can hold it in Qt's own
// engine, and so the desktop's one control system can retune it later;
// beside it, how a search is typed with them. What it finds in the cards,
// and how a match is washed, is the shell's one way of searching, Search.js.
// Letters go by what they type, wherever they sit on the layout.
.pragma library

// Each act and the keys that do it, in the modes it's done in: normal,
// unless it says, or visual, while V selects cards. A key is named as the
// stack names it (Notifications.qml, `named`): a modifier, then the letter
// it typed, or else its name, with Shift unnamed beside a letter, which
// says it by its case. A list of keys is a sequence typed one after
// another, as vi's dd. Where one differs from Neovim, the reason is beside
// it.
const both = ["normal", "visual"];
const table = [
    // Through the cards, or, stepped into the front one's acts, through
    // them; selecting, the selection runs on with them.
    { act: "next", keys: ["j", "Down"], modes: both },
    { act: "previous", keys: ["k", "Up"], modes: both },
    // Into the front card's acts and back out, as vi's l and h move
    // right and left.
    { act: "into", keys: ["l", "Right"] },
    { act: "out", keys: ["h", "Left"] },
    // Return acts rather than moving down a line, as everywhere of ours,
    // and nothing with Ctrl closes after it: q does, as in Neovim (Joel,
    // 2026-11-02: "all of those ctrl keys do not feel very vim to me").
    { act: "run", keys: ["Return", "Enter"] },
    // Forgetting is a card's ending act, as vi's dd deletes a line.
    // Backspace too, since the cards' footers have named it from the start.
    { act: "forget", keys: [["d", "d"], "Delete", "Backspace"] },
    // Selected, d forgets them at once, as vi's d does a selection.
    { act: "forget", keys: ["d", "x", "Delete", "Backspace"], modes: ["visual"] },
    // All of them, as vi's dG deletes from here to the last line: the
    // front card is the first and the stack runs on to the oldest. Shift
    // with Delete or Backspace does the same, and neither asks.
    { act: "forgetAll", keys: [["d", "G"], "Shift+Delete", "Shift+Backspace"] },
    // Back what was forgotten last, as vi's u undoes, while the stack's
    // held.
    { act: "undo", keys: ["u"] },
    // Copying what the front card said, as vi's yy yanks a line, leaving
    // the stack where it was; selected, y copies them all.
    { act: "copy", keys: [["y", "y"]] },
    { act: "copy", keys: ["y"], modes: ["visual"] },
    // The newest and the oldest, as vi's first and last lines.
    { act: "newest", keys: [["g", "g"]], modes: both },
    { act: "oldest", keys: ["G"], modes: both },
    // Searching what the cards say, forward and back, and again either
    // way, as vi's / ? n N; Return ends the typing.
    { act: "searchForward", keys: ["/"] },
    { act: "searchBack", keys: ["?"] },
    { act: "again", keys: ["n"] },
    { act: "againBack", keys: ["N"] },
    // Selecting whole cards, as vi's V selects whole lines, and V again
    // ends it.
    { act: "select", keys: ["V"], modes: both },
    // Every key here, as Joel's leader lists them.
    { act: "keys", keys: [["Space", "?"]] },
    // q closes, as Joel's Neovim closes help and its plugin windows, so
    // nothing here records macros.
    { act: "close", keys: ["q"] },
    { act: "back", keys: ["Escape"], modes: both }
];

// The keys as Space ? lists them: each line's name and the acts whose
// first keys it shows, in the table's order of use. Escape isn't listed,
// since it always steps back. Each says what its keys do as a sentence,
// as the keys' own lists and the reader's do, since these name keys rather
// than acts to choose, which keep their Title Case.
const listed = [
    { name: "Turn the cards", acts: ["next", "previous"] },
    { name: "Into its acts and out", acts: ["into", "out"] },
    { name: "Do it", acts: ["run"] },
    { name: "Forget it", acts: ["forget"] },
    { name: "Forget them all", acts: ["forgetAll"] },
    { name: "Bring it back", acts: ["undo"] },
    { name: "Copy what it said", acts: ["copy"] },
    { name: "The newest and oldest", acts: ["newest", "oldest"] },
    { name: "Search", acts: ["searchForward", "searchBack"] },
    { name: "The next match", acts: ["again", "againBack"] },
    { name: "Select cards", acts: ["select"] },
    { name: "These keys", acts: ["keys"] },
    { name: "Close", acts: ["close"] }
];

// How a key is drawn where Chord.qml has no glyph for it by name.
const faces = { Return: "\u21b5" };

// The keys listed, each a line's name and its chord as Chord.qml draws
// one: each act's first keys, a sequence's keys a space apart, and each
// act's set a wider space from the next.
function keyList() {
    return listed.map(line => {
        const chord = [];
        line.acts.forEach((act, at) => {
            const first = table.find(one => one.act === act).keys[0];
            const sequence = Array.isArray(first) ? first : [first];
            sequence.forEach((key, place) => {
                chord.push({ text: faces[key] ?? key, after: at > 0 || place > 0 });
            });
        });
        return { name: line.name, chord };
    });
}

// Where a list scrolled in a room is, as Neovim's ruler says it: All when
// the room shows it all, Top and Bot at its ends, and otherwise how far
// through it is, as what's above against all that doesn't show.
function ruler(top, room, total) {
    const below = Math.max(total - top - room, 0);
    if (top === 0) {
        return below === 0 ? "All" : "Top";
    }
    if (below === 0) {
        return "Bot";
    }
    return `${Math.floor((top * 100) / (top + below))}%`;
}

const digit = /^[0-9]$/;

// Where the keys stand between presses: the count typed so far, and the
// keys of a sequence begun.
function rest() {
    return { count: "", typed: [] };
}

function same(a, b) {
    return a.length === b.length && a.every((key, at) => key === b[at]);
}

function begins(sequence, typed) {
    return typed.length < sequence.length && typed.every((key, at) => key === sequence[at]);
}

// What a key does from where the keys stand, in a mode, normal unless
// said: the act, if one's done, with its count (0 when none was typed),
// how the keys stand after it, and whether the key was the stack's at
// all. Escape always cancels what's in progress before it steps out of
// anything.
function read(standing, key, mode) {
    const now = mode ?? "normal";
    // A key with no name, as Shift pressed on its own for the G of dG,
    // changes nothing, so a sequence or count begun carries on past it.
    if (key === "") {
        return { act: "", count: 0, standing, ours: false };
    }
    const typed = standing.typed.concat([key]);
    const counted = standing.count !== "";
    if (key === "Escape" && (counted || standing.typed.length > 0)) {
        return { act: "", count: 0, standing: rest(), ours: true };
    }
    // A count before a motion, as 5j; a lone 0 counts nothing here.
    if (standing.typed.length === 0 && digit.test(key) && (counted || key !== "0")) {
        return { act: "", count: 0, standing: { count: standing.count + key, typed: [] }, ours: true };
    }
    const count = counted ? Number(standing.count) : 0;
    let begun = false;
    for (const row of table.filter(one => (one.modes ?? ["normal"]).includes(now))) {
        for (const keys of row.keys) {
            const sequence = Array.isArray(keys) ? keys : [keys];
            if (same(sequence, typed)) {
                return { act: row.act, count, standing: rest(), ours: true };
            }
            begun = begun || begins(sequence, typed);
        }
    }
    if (begun) {
        return { act: "", count: 0, standing: { count: standing.count, typed }, ours: true };
    }
    // A key that ends nothing lets what was begun go, as vi does.
    const begunBefore = counted || standing.typed.length > 0;
    return { act: "", count: 0, standing: rest(), ours: begunBefore };
}

// A key typed into a search: the pattern after it, and whether it ends
// the typing, found to keep or cancelled. Return keeps it; Escape, or
// Backspace with nothing left to take, cancels, as vi's / does.
function searching(pattern, key) {
    if (key === "Return" || key === "Enter") {
        return { pattern, ends: "found" };
    }
    if (key === "Escape" || (key === "Backspace" && pattern === "")) {
        return { pattern, ends: "cancelled" };
    }
    if (key === "Backspace") {
        return { pattern: pattern.slice(0, -1), ends: "" };
    }
    if (key === "Space") {
        return { pattern: `${pattern} `, ends: "" };
    }
    // A letter types itself; any other key changes nothing.
    return { pattern: key.length === 1 ? pattern + key : pattern, ends: "" };
}

// Which card a search turns to, from a stack's cards in order, the front
// first, and whether each holds the pattern: the first after the front
// that does, going on round to the front last, as vi wraps; backward, the
// last before it. -1 when none does.
function next(holds, forward) {
    const count = holds.length;
    for (let step = 1; step <= count; step++) {
        const at = forward ? step % count : (count - step) % count;
        if (holds[at]) {
            return at;
        }
    }
    return -1;
}
