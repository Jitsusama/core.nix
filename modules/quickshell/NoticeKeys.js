// The keys of the notices' stack while it holds them, as one table of key
// and act (docs/design.md, Vi's Keys), read apart from the stack
// (Notifications.qml) so tests/notice-keys.qml can hold it in Qt's own
// engine, and so the desktop's one control system can retune it later.
// Letters go by what they type, wherever they sit on the layout.
.pragma library

// Each act and the keys that do it. A key is named as the stack names it
// (Notifications.qml, `named`): a modifier, then the letter it typed, or
// else its name, with Shift unnamed beside a letter, which says it by its
// case. A list of keys is a sequence typed one after another, as vi's dd.
// Where one differs from Neovim, the reason is beside it.
const table = [
    // Through the cards, or, stepped into the front one's acts, through
    // them.
    { act: "next", keys: ["j", "Down"] },
    { act: "previous", keys: ["k", "Up"] },
    // Into the front card's acts and back out, as vi's l and h move
    // right and left.
    { act: "into", keys: ["l", "Right"] },
    { act: "out", keys: ["h", "Left"] },
    // Return acts rather than moving down a line, as everywhere of ours.
    { act: "run", keys: ["Return", "Enter"] },
    { act: "runAndClose", keys: ["Ctrl+Return", "Ctrl+Enter"] },
    // Forgetting is a card's ending act, as vi's dd deletes a line.
    // Backspace too, since the cards' footers have named it from the start.
    { act: "forget", keys: [["d", "d"], "Delete", "Backspace"] },
    // With Shift, all of them. DD is ours: D deletes to the line's end in
    // vi, which a stack has no use for.
    { act: "forgetAll", keys: [["D", "D"], "Shift+Delete", "Shift+Backspace"] },
    // The newest and the oldest, as vi's first and last lines.
    { act: "newest", keys: [["g", "g"]] },
    { act: "oldest", keys: ["G"] },
    // q closes, as Joel's Neovim closes help and its plugin windows, so
    // nothing here records macros.
    { act: "close", keys: ["q"] },
    { act: "back", keys: ["Escape"] }
];

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

// What a key does from where the keys stand: the act, if one's done, with
// its count (0 when none was typed), how the keys stand after it, and
// whether the key was the stack's at all. Escape always cancels what's in
// progress before it steps out of anything.
function read(standing, key) {
    // A key with no name, as Shift pressed on its own for the second D of
    // DD, changes nothing, so a sequence or count begun carries on past it.
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
    for (const row of table) {
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
