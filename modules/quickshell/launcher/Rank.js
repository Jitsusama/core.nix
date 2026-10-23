.pragma library
// How the launcher orders what it found, so the same letters always land
// on the same row (docs/design.md, The Launcher). Joel has little memory to
// spare and types two to four letters without looking, so the order may
// change only when he changes it, by picking something else twice.
//
// - A thing is one row: rows that say they're the same thing (their `is`,
//   or their key) are one, in the first one's place, as the one with the
//   most to say (its `rich`).
// - Tiers: a name that is what's typed, then a name that starts with it,
//   then a name with a later word that starts with it, then only
//   something else it's known by. An act's name starts with its verb, so
//   "pow" is the act that starts Power before one that uses power. Nothing lifts
//   a row over a better tier but what was latched for what's typed.
// - The latch: what was picked for what's typed, and for each start of a
//   word picked for, so a pick for "slack" holds at "sl" too. It moves to
//   another row only once that row's been picked twice running, so one
//   slip teaches nothing, and it never fades.
// - The rest, within a tier: what was picked for what's typed, most
//   first; then what's picked most of all, as counted when the launcher
//   opened, so nothing moves while it's open; then the better match, the
//   shorter name and the alphabet.
//
// It's plain functions over plain values, so tests/ranking.qml holds them
// to a table in Qt's own JavaScript engine.

// What a row is, for telling two rows apart.
function identity(row) {
    return row.is ?? row.key ?? "";
}

// Rows that are one thing, as one row: the first one's place, the richest
// one's words.
function once(rows) {
    const at = {};
    const out = [];
    for (const row of rows) {
        const id = identity(row);
        const seen = id === "" ? undefined : at[id];
        if (id === "") {
            out.push(row);
        } else if (seen === undefined) {
            at[id] = out.length;
            out.push(row);
        } else if ((row.rich ?? 0) > (out[seen].rich ?? 0)) {
            out[seen] = row;
        }
    }
    return out;
}

// What's typed as the latch keeps it: in lower case, its spaces single.
function plainly(typed) {
    return String(typed).trim().toLowerCase().replace(/\s+/g, " ");
}

// The tier of a name starting with what's typed, between one typed whole
// and one whose later word starts with it.
const NAME_START = 0.5;

// Which tier a match is in: 0 for a name that is what's typed, 0.5 for a
// name starting with it, 1 for a later word of its name starting with it,
// 2 for something else it's known by. A row added late, at 1 unless it
// says, goes among the word starts.
function tier(name, typed, byWord) {
    if (byWord) {
        return 2;
    }
    const said = plainly(name);
    const query = plainly(typed);
    if (said === query) {
        return 0;
    }
    return said.startsWith(firstWord(query)) ? NAME_START : 1;
}

// The strings a pick for `typed` is latched at: it, and when it's one word,
// every start of it, but never one shorter than `least` letters, which a
// row that can't be taken back sets to its first word's length.
function latchedAt(typed, least) {
    const query = plainly(typed);
    if (query === "") {
        return [];
    }
    if (query.includes(" ")) {
        return [query];
    }
    const out = [];
    for (let n = Math.max(1, least ?? 1); n <= query.length; n += 1) {
        out.push(query.slice(0, n));
    }
    return out;
}

// The latches after a pick of `key` for `typed`, as a new table: each
// string the pick counts at holds what it held, unless `key` has now been
// picked twice running there, or nothing was held.
// latches: { string -> { on: key, rival: key, times: n } }
function latch(latches, typed, key, least) {
    const out = Object.assign({}, latches);
    for (const query of latchedAt(typed, least)) {
        const held = out[query];
        if (!held || held.on === key) {
            out[query] = { on: key, rival: "", times: 0 };
        } else if (held.rival === key && held.times + 1 >= 2) {
            out[query] = { on: key, rival: "", times: 0 };
        } else {
            out[query] = { on: held.on, rival: key, times: held.rival === key ? held.times + 1 : 1 };
        }
    }
    return out;
}

// What's latched for what's typed, or "".
function latched(latches, typed) {
    const held = latches[plainly(typed)];
    return held ? held.on : "";
}

// The first word of a name, in lower case, which a row that can't be
// taken back has to be typed past before it's latched.
function firstWord(name) {
    return plainly(name).split(" ")[0] ?? "";
}

// Two names in the alphabet's order, for a sort.
function alphabetically(a, b) {
    if (a === b) {
        return 0;
    }
    return a < b ? -1 : 1;
}

// Found rows in order. Each is { choice, score, tier } and the choice has
// its `key` and `name`. `ask` gives what orders them: the key latched for
// what's typed (latched), how much each key was picked for it (picked, key
// -> weight), and how often each key's been picked at all (often, key ->
// count), taken when the launcher opened.
function order(found, ask) {
    const latchedKey = ask.latched ?? "";
    const picked = ask.picked ?? {};
    const often = ask.often ?? {};
    const weight = f => (picked[f.choice.key] ?? { weight: 0 }).weight ?? 0;
    const count = f => weight(f) > 0 ? 0 : often[f.choice.key] ?? 0;
    const first = f => latchedKey !== "" && f.choice.key === latchedKey ? 0 : 1;
    return found.slice().sort((a, b) => first(a) - first(b)
        || a.tier - b.tier
        || weight(b) - weight(a)
        || count(b) - count(a)
        || b.score - a.score
        || a.choice.name.length - b.choice.name.length
        || alphabetically(a.choice.name, b.choice.name));
}

// Where rows found later, such as a collection's best, go among the rows
// already ordered: after the rows of their tier and the better ones, never
// above them, but the one latched for what's typed goes first.
function placed(rows, later, latchedKey) {
    const all = rows.slice();
    for (const row of later) {
        if (latchedKey !== "" && row.key === latchedKey) {
            all.unshift(row);
        } else {
            const at = all.findIndex(f => (f.tier ?? 1) > (row.tier ?? 1));
            all.splice(at < 0 ? all.length : at, 0, row);
        }
    }
    return all;
}

// Where the selection goes when the rows change under it: onto the row it
// was on, by what that row is, or else where it was.
function pinned(rows, id, at) {
    if (id !== "") {
        const found = rows.findIndex(row => identity(row) === id);
        if (found >= 0) {
            return found;
        }
    }
    return Math.max(0, Math.min(at, rows.length - 1));
}

// The keys picked most, most first, ties by name, so home's likely picks
// change order only when the counts do. `counts` is key -> picks, and
// `names` key -> its name.
function likeliest(counts, names) {
    return Object.keys(counts).filter(key => names[key] !== undefined).sort((a, b) =>
        counts[b] - counts[a] || alphabetically(names[a], names[b]));
}
