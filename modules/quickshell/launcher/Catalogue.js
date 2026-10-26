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
// a place: what was copied, the emoji, the notices past, the shortcuts,
// and what comes and goes in Agents, the queue's jobs, AI spend's findings and
// the subagents' runs. A model asked to paste or find one opens the place
// instead.
const LISTS = new Set(["clipboard", "emoji", "notices", "shortcuts", "characters", "jobs",
    "spend", "runs"]);

// Of those, the ones long enough that a walk over every row leaves them
// out: their rows are each made one way, and proven offscreen.
const LONG = new Set(["clipboard", "emoji", "characters"]);

// Whether a collection is one of the long lists.
function long(source) {
    return LONG.has(source);
}

// Collections whose things are listed, each one opened by its own run, but
// not what each holds: every program's Quit and New window, every window's
// Size, Float and workspaces, and every quest's own, would bury the rest a
// few hundred times over. The window Joel came from keeps its own, as This
// window.
const WHOLE = new Set(["applications", "windows", "quests"]);

// What a keyboard's key does is a lookup, not something done to the
// machine, and the boards' keys are a few hundred rows.
const LOOKUPS = "keyboard:key:";

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

// What a step into a row holds, as the right arrow and the catalogue
// both show it: everything it can be told, Return's own act first, even
// where its provider left it out, so the right arrow always does more
// than Return and never less (Joel's concept map: Return is the default
// act and the right arrow does more; agreed with 15J7N3 for every row).
// A row that only steps in, asks first or asks for words has no act of
// its own to add, nor does one holding nothing.
function leading(row, inside) {
    if (inside.length === 0 || kindOf(row) !== "action" || row.confirm || row.given) {
        return inside;
    }
    const verb = verbOf(row);
    const said = verb.toLowerCase();
    if (said === "" || inside.some(one => verbOf(one).toLowerCase() === said)) {
        return inside;
    }
    const own = {
        key: `return:${said}`,
        name: verb.charAt(0).toUpperCase() + verb.slice(1),
        verb,
        glyph: row.glyph ?? "",
        hue: "action",
        stays: row.stays === true,
        // Asked through the row, so a run that depends on how the row
        // stands now, a program's switch or start, is read when it's done.
        run: () => row.run()
    };
    return [own].concat(inside);
}

// Whether the catalogue lists it: a row with a name that does something
// or holds more, never a figure, a copy of what's shown, a command's
// start to finish in the field, or a thing that's gone.
function listed(row) {
    if (!row || row.gone || (row.name ?? "") === "" || row.start !== undefined) {
        return false;
    }
    if (String(row.key ?? "").startsWith(LOOKUPS)) {
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

// Whether Return does something on a row (Joel, 2026-10-26: every row
// has a Return act; a row that holds more steps in, a row that only shows
// a value copies it, and a row that can do neither doesn't belong in the
// launcher). One still being read, whose value is an ellipsis, isn't held
// against it, since it copies once it's read.
function acts(row) {
    if (!row || row.gone || row.side === "\u2026") {
        return true;
    }
    const steps = !!row.verbs || !!row.targets || !!row.nested || !!row.category
        || !!row.opens || !!row.from;
    if (steps || !!row.given || row.start !== undefined) {
        return true;
    }
    return !!row.run && !(row.figure === true && verbOf(row) === "close");
}

// Whether what a row holds is listed too, at `depth` below home, inside
// `above`, the row holding it, or null at home.
function descends(row, depth, above) {
    if (depth >= DEEPEST) {
        return false;
    }
    if (row.opens && LISTS.has(row.opens)) {
        return false;
    }
    if (above && WHOLE.has(above.opens)) {
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
