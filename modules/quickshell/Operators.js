// What the launcher's operators do to its rows, as Vim's d and y do to a
// buffer's lines (QueryKeys.js reads their keys): which of a row's actions
// d does and which y does, what a copy copies, and when d asks before it
// does what can't be put back. Read apart from the launcher so
// tests/query-keys.qml can hold it in Qt's own engine.
.pragma library
.import "QueryKeys.js" as Keys

// The rows an operator takes, from where the rows stand (as Keys.moved
// takes them): over the row and count - 1 below it with no motion, as dd
// and 3dd, to the last at most, as Neovim's 9dd near the end; or from the
// row to wherever the motion goes, whichever way, every row between
// included, as a linewise motion takes lines. No rows span nothing.
function spanned(count, motion, place) {
    if (place.last < place.highest) {
        return { from: 0, to: -1 };
    }
    if (motion === null) {
        return { from: place.at, to: Math.min(place.last, place.at + Math.max(1, count) - 1) };
    }
    const to = Keys.moved(motion.act, motion.count, place);
    return { from: Math.min(place.at, to), to: Math.max(place.at, to) };
}

// A row's ending action, the one that ends what it is, as a window's Close
// or a notice's Forget It: the key language's act ending (modules/keys/
// language.toml), which words.json names on each such action as "act".
// The row itself, where it's one, as an action listed in a step into a
// window is; else the first of its actions that is; else none. A verb of
// close alone isn't one, since an answer's Return that only closes the
// launcher says close too.
function endingOf(row, actions) {
    const ends = one => one.act === "ending" && !!one.run;
    if (ends(row)) {
        return row;
    }
    return actions.find(ends) ?? null;
}

// A row's copy: the row itself, where Return copies it, as an answer's
// rows and a figure do; else the first of its actions that copies, as a
// file's Copy the Path or a window's Copy Title; else none. `copies` says
// whether a row copies, as the launcher reads its verb.
function copyOf(row, actions, copies) {
    if (copies(row)) {
        return row;
    }
    return actions.find(copies) ?? null;
}

// What a copy copies, which its row says as `copies`, a text or a function
// giving one as it's asked, so a live figure copies what it shows: "" for
// one that says nothing.
function textOf(action) {
    const copies = action.copies;
    const text = typeof copies === "function" ? copies() : copies;
    return typeof text === "string" ? text : "";
}

// What several copies put on the clipboard: each a line, in the rows'
// order, as a linewise yank holds lines, those copying nothing left out.
function copied(actions) {
    return actions.map(textOf).filter(text => text !== "").join("\n");
}

// The rows a span takes, each with its place, the row at -1 being an
// answer's headline above them, where there's one.
function taken(rows, span, headline) {
    const these = [];
    for (let at = span.from; at <= span.to; at++) {
        const row = at === -1 ? headline : rows[at];
        if (row) {
            these.push({ at, row });
        }
    }
    return these;
}

// Each row's ending action over the rows taken, with its place, rows with
// none left out: what d does there.
function endings(rows, actionsOf) {
    return rows
        .map(({ at, row }) => ({ at, action: endingOf(row, actionsOf(row)) }))
        .filter(ending => ending.action !== null);
}

// Each row's copy over the rows taken, with its place, rows with none
// left out: what y does there.
function copiesIn(rows, actionsOf, copies) {
    return rows
        .map(({ at, row }) => ({ at, action: copyOf(row, actionsOf(row), copies) }))
        .filter(copy => copy.action !== null);
}

// Whether d asks once before it does these: where any can't be put back,
// which an action says by having no `undo`, as a window's Close, since
// the window and what it held go with it.
function asks(actions) {
    return actions.some(action => typeof action.undo !== "function");
}

// Whether an operator answers the ask standing, { act, from, to }: the
// same act again over the same rows, as a second dd over the row it asked
// about does; anything else lets the ask go.
function answers(ask, act, span) {
    return ask !== null && ask.act === act && ask.from === span.from && ask.to === span.to;
}
