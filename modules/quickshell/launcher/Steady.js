// How a list the launcher shows takes what's changed under it while Joel
// looks at it: a session starting to work, a window renamed. Its rows
// stay in the order they're shown, each with what it says now, those new
// come at the end and those gone go, and the selection stays on its row,
// so nothing moves under the cursor. Only a search Joel types orders by
// rank (Rank.js). Apart from the launcher, so tests/steady.qml can hold
// it in Qt's own engine.
.pragma library

// The rows as they read now, in the order of those shown: each row shown
// that's still there, as it is now, then each new one in its own order.
// A row is known by `id`; rows alike in it pair off in their order, so
// two windows titled alike keep their places.
function kept(shown, now, id) {
    const waiting = {};
    now.forEach((row, at) => {
        const said = id(row);
        if (waiting[said] === undefined) {
            waiting[said] = [];
        }
        waiting[said].push(at);
    });
    const taken = now.map(() => false);
    const out = [];
    for (const row of shown) {
        const left = waiting[id(row)];
        if (left && left.length > 0) {
            const at = left.shift();
            taken[at] = true;
            out.push(now[at]);
        }
    }
    now.forEach((row, at) => {
        if (!taken[at]) {
            out.push(row);
        }
    });
    return out;
}

// Where the selection goes in them, from where it `was`: { row, at,
// lowest }, the id of the row it was on ("" for none) and its place. It
// goes onto that row, or, that one gone, stays where it was, no lower than
// `lowest` (-1 when an answer's headline stands above the rows) and no
// further than the last row.
function selection(rows, was, id) {
    const found = was.row === "" ? -1 : rows.findIndex(row => id(row) === was.row);
    if (found >= 0) {
        return found;
    }
    return Math.max(was.lowest, Math.min(was.at, rows.length - 1));
}
