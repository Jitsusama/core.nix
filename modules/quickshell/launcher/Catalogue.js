// The launcher's catalogue: every place and every thing's actions, by the
// same rows the launcher runs, so whatever it gains, what asks it from
// outside gains too (pi-ask's one tool, over `qs ipc call launcher list`
// and `run`). This holds the rules; Launcher.qml walks its rows by them.
// An id is the row's path from home, each step's steady id joined by a
// "\u203a", which no key holds, so it stays the same from one run to the
// next.
.pragma library

// How deep below home a row is listed: a place (0), what it holds (1), a
// thing's actions or parts (2), a setting's choices (3), and one more.
const DEEPEST = 4;

// Collections whose things are a list of items, each listed whole only as
// a place: what was copied, the emoji, the notices past and the shortcuts.
// A model asked to paste or find one opens the place instead.
const LISTS = new Set(["clipboard", "emoji", "notices", "shortcuts", "characters"]);

// Collections whose things are listed, each one opened by its own run, but
// not what each holds: every program's Quit and New window would bury the
// rest a few hundred times over.
const WHOLE = new Set(["applications"]);

// What a row's verb says it does that can't be taken back, or that ends
// something in use: a window or a program closed, a call left, a session
// locked or ended, a radio or a device turned off or forgotten, the
// screen shared. Those are offered, never done, from outside.
const DISRUPTIVE = new Set(["close", "quit", "leave", "lock", "sleep", "reboot", "stop", "off",
    "turn off", "forget", "disconnect", "share"]);

// The verb a row says now: a word, or a getter's.
function verbOf(row) {
    const verb = row.verb;
    return typeof verb === "string" ? verb : "";
}

// Whether it's done only with Joel's own Return: one the launcher asks to
// be sure of, or one whose verb ends something.
function disruptive(row) {
    return !!row.confirm || DISRUPTIVE.has(verbOf(row));
}

// Whether it's a place, which is opened, or an action, which is done: a
// part holding more is a place, though Return steps into it by running.
function kindOf(row) {
    if (row.category || row.opens || verbOf(row) === "more") {
        return "place";
    }
    const holds = !!row.verbs || !!row.targets || !!row.nested;
    return holds && !row.run && !row.given ? "place" : "action";
}

// Whether the catalogue lists it: a row with a name that does something
// or holds more, never a figure, a copy of what's shown, a command's
// start to finish in the field, or a thing that's gone.
function listed(row) {
    if (!row || row.gone || (row.name ?? "") === "" || row.start !== undefined) {
        return false;
    }
    const holds = !!row.verbs || !!row.targets || !!row.nested || !!row.category || !!row.opens;
    if (holds) {
        return true;
    }
    if (row.figure === true) {
        return false;
    }
    const verb = verbOf(row);
    return verb !== "copy" && verb !== "close" && (!!row.run || !!row.given);
}

// Whether what a row holds is listed too, at `depth` below home.
function descends(row, depth) {
    if (depth >= DEEPEST) {
        return false;
    }
    if (row.opens && LISTS.has(row.opens)) {
        return false;
    }
    if (!row.opens && row.from && WHOLE.has(row.from.settings?.name)) {
        return false;
    }
    return !String(row.key ?? "").startsWith("shortcut-level:");
}

// One entry: its id, what it is, its words and where it sits as Joel reads
// it, what it says now, whether only Joel's Return does it, and the words
// it asks for, for one that takes them.
function entry(row, id, within) {
    const out = { id, kind: kindOf(row), words: row.name, in: within.join(" \u203a "),
        disruptive: disruptive(row) };
    if (typeof row.side === "string" && row.side !== "") {
        out.says = row.side;
    }
    if (row.given) {
        out.args = [{ name: "words", words: row.ask ?? "" }];
    }
    return out;
}
