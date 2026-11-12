// How a list the launcher shows takes what's changed under it while Joel
// looks at it: a session starting to work, a window renamed. Its rows
// stay in the order they're shown, each with what it says now, those new
// come at the end and those gone stay where they were, dimmed, until a key
// makes the list afresh, and the selection stays on its row, so nothing
// moves under the cursor (Joel, 2026-10-25: "it needs to be fucking stable
// and always make sense"). Only a search Joel types orders by rank
// (Rank.js). Apart from the launcher, so tests/steady.qml can hold it in
// Qt's own engine.
.pragma library

// The rows as they read now, in the order of those shown: each row shown
// as it is now, or as it was, `gone` and `went`, where it's no longer
// there, then each new one in its own order. A new one comes at the end,
// out of any place a tree would give it, so one that says how it reads
// ranked flat (`ranked`, as a session under its orchestrator does) reads so;
// a list shown afresh has nothing new, and keeps every row's own. `went`
// tells a row that left from one its source draws dimmed itself, gone but
// there, as Write in Slack stands dimmed until Slack answers. A row is
// known by `id`; rows alike in it pair off in their order, so two windows
// titled alike keep their places.
// The rows shown that may stand where they are as the list is said again:
// what ends a search comes after it all and is made again with it, so it's
// no row to hold in place (held, it stood dimmed above the one made
// again). An answer there only while nothing else was, as Ask's for two
// plain words, goes once it gives way (`gaveWay`) to what came after it,
// as the notices daemon's finds come a moment after the key, rather than
// standing dimmed above them with the selection on it.
function standing(shown, gaveWay) {
    return shown.filter(row => !row.endedBy && !(gaveWay && row.answered));
}

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
        } else {
            out.push(row.went ? row : Object.assign({}, row, { gone: true, went: true }));
        }
    }
    now.forEach((row, at) => {
        if (!taken[at]) {
            out.push(shown.length > 0 && typeof row.ranked === "function" ? row.ranked() : row);
        }
    });
    return out;
}

// The rows shown, held still as kept holds them, with no new one among
// them: while visual, a pending operator or the command line holds the
// rows, what arrives waits, counted, and comes in at the end once it ends
// (VKCF3Z's spec). { rows, waiting }.
function held(shown, now, id) {
    const out = kept(shown, now, id);
    return { rows: out.slice(0, shown.length), waiting: out.length - shown.length };
}

// The rows without those that left since they were shown, as a key makes
// the list afresh; a row its source draws dimmed stays where it stands.
function forgotten(rows) {
    return rows.filter(row => !row.went);
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
