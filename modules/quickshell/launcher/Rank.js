.pragma library
// How the launcher orders what it found, so the same letters always land
// on the same row (docs/design.md, The Launcher). Joel has little memory to
// spare and types two to four letters without looking, so the order may
// change only when he changes it, by picking something else more.
//
// - A thing is one row: rows that say they're the same thing (their `is`,
//   or their key) are one, in the first one's place, as the one with the
//   most to say (its `rich`).
// - Tiers: a name that is what's typed, then a name that starts with it,
//   then a name with a later word that starts with it, then only
//   something else it's known by, typed whole before only started. An act's name starts with its verb, so
//   "pow" is the act that starts Power before one that uses power. Nothing lifts
//   a row over a better tier but what was latched for what's typed.
// - The latch: the row picked most for what's typed and for the words it
//   starts, counted together, once it's been picked at least two more
//   times than any other there, so one slip teaches nothing and a stray
//   pick for "light" never holds "l". The row it was left on holds until
//   another's been picked two more times than it. It's worked out from
//   every pick, never fading, so a latch seeded from a mis-pick heals by
//   the same rule. A row that isn't there now (a notice gone, a window
//   closed) counts for nothing, and a row named what's typed is first
//   over another latched, unless that other was picked for exactly this
//   at least twice. It only orders: every match still shows.
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

// The tier of a row found only by starting another word it's known by,
// after one whose other word is typed whole: "quiet" is Turn on do not
// disturb, known as quiet, before Turn the volume down, known as quieter.
const WORD_START = 2.5;

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

// The tier of a row found by its other words (`known`, as one string):
// 2 when each word typed is one of them whole, and WORD_START when one
// only starts.
function wordTier(known, typed) {
    return says(known, typed) ? 2 : WORD_START;
}

// Whether each word typed is one of a string's words whole.
function says(text, typed) {
    const words = plainly(text).split(" ");
    const query = plainly(typed);
    return query !== "" && query.split(" ").every(word => words.includes(word));
}

// How many more picks than any other a row needs to hold what's typed, so
// one stray pick, or one more than another's, moves nothing.
const LEAD = 2;

// Whether a pick for `asked` counts for `query`: one for exactly it, and
// when both are one word, one for a word it starts, as "slack" for "sl".
// Words typed together count only whole.
function countsFor(asked, query) {
    return asked === query || (!(query.includes(" ") || asked.includes(" "))
        && asked.startsWith(query));
}

// How often each row was picked for what's typed: `all` counting the words
// it starts, `exact` only it. `there(key)` is the fewest letters a row
// counts at, 0 for one that isn't there now, so a gone row counts for
// nothing; one that can't be taken back counts only once its first word's
// typed whole. picks: { query -> { key -> { n, c, t } } }, `c` every pick.
function tally(picks, typed, there) {
    const query = plainly(typed);
    const out = {};
    const counted = query === "" ? []
        : Object.keys(picks).filter(asked => countsFor(asked, query));
    for (const asked of counted) {
        const here = Object.keys(picks[asked])
            .filter(key => there(key) > 0 && query.length >= there(key));
        for (const key of here) {
            const one = out[key] ?? { all: 0, exact: 0 };
            const times = picks[asked][key].c ?? 0;
            one.all += times;
            one.exact += asked === query ? times : 0;
            out[key] = one;
        }
    }
    return out;
}

// What's latched for what's typed, or "". `held` is the row it was left on
// (Picks.qml keeps it), which holds while it's been picked LEAD times and
// nothing's been picked LEAD more; when it doesn't, the row picked most
// takes it, by LEAD more than the next, so one slip neither takes a latch
// nor loosens one. When a row there is named what's typed (`whole`, their
// keys), another holds only if picked for exactly this LEAD times.
// ask: { there, whole, held }, `there` as tally takes it.
function latched(picks, typed, ask) {
    const tallied = tally(picks, typed, ask.there);
    const whole = ask.whole;
    const held = ask.held ?? "";
    const times = key => (tallied[key] ?? { all: 0 }).all;
    const ranked = Object.keys(tallied).sort((a, b) => times(b) - times(a)
        || alphabetically(a, b));
    const named = whole ?? [];
    const may = key => named.length === 0 || named.includes(key) || tallied[key].exact >= LEAD;
    const holds = key => tallied[key] !== undefined && times(key) >= LEAD && may(key)
        && ranked.every(other => other === key || times(other) < times(key) + LEAD);
    if (held && holds(held)) {
        return held;
    }
    const [leader, second] = ranked;
    const next = second === undefined ? 0 : times(second);
    return leader !== undefined && times(leader) - next >= LEAD && may(leader) ? leader : "";
}

// The row a latch was left on, from a table kept as { query -> key }, or as
// { query -> { on } } by a launcher before.
function heldAt(latches, typed) {
    const held = latches[plainly(typed)];
    return typeof held === "string" ? held : held?.on ?? "";
}

// The latches after a pick for `typed`, as a new table: each query the pick
// counts at (it, and every start of it when it's one word) left on what's
// latched there now, and a latch on a row with no picks left forgotten.
function relatch(latches, picks, typed, there) {
    const picked = new Set();
    for (const asked of Object.keys(picks)) {
        for (const key of Object.keys(picks[asked])) {
            picked.add(key);
        }
    }
    const query = plainly(typed);
    const fresh = {};
    for (let n = query.includes(" ") ? query.length : 1; n <= query.length; n += 1) {
        const start = query.slice(0, n);
        fresh[start] = latched(picks, start, { there, held: heldAt(latches, start) });
    }
    const out = {};
    for (const asked of Object.keys(latches)) {
        const held = heldAt(latches, asked);
        if (fresh[asked] === undefined && picked.has(held)) {
            out[asked] = held;
        }
    }
    for (const start of Object.keys(fresh)) {
        if (fresh[start] !== "") {
            out[start] = fresh[start];
        }
    }
    return out;
}

// Who's there to count, from the rows found now (key -> the fewest letters
// each counts at): those, and none other, since a row not found can't be
// shown first.
function foundNow(rows) {
    return key => rows[key] ?? 0;
}

// Who's there to count when what's found isn't known, as at a shorter
// start than what's typed: the rows found now as they say, and any other
// as though there, so a start is said to be enough only when it's sure.
function maybeThere(rows) {
    return key => rows[key] ?? 1;
}

// Who's there to count from every row there is (key -> the fewest letters
// each counts at), as when a pick's written: those, none of a kind that's
// there but not them, and any of a kind not there at all, which may not be
// read yet, as though there.
function standing(known) {
    const kinds = new Set(Object.keys(known).map(kindOf));
    return key => known[key] ?? (kinds.has(kindOf(key)) ? 0 : 1);
}

// The fewest letters a row is learned at: its first word's, for one that
// can't be taken back, so a slip of two letters never powers off.
function leastOf(row) {
    return row.confirm ? Math.max(1, firstWord(row.name).length) : 1;
}

// The picks with each row that's gone left out, and a query left with none
// dropped. Gone is a row not in `known` (key -> anything) whose kind is
// (the key's start, "window" or "notice"), so a kind not read yet, or off,
// keeps its picks; and one not picked within `stale` milliseconds of `now`,
// so a row that comes and goes, as a call's Leave does, keeps its own.
function forget(picks, known, now, stale) {
    const kinds = new Set(Object.keys(known).map(kindOf));
    const gone = (key, entry) => known[key] === undefined && kinds.has(kindOf(key))
        && now - (entry.t ?? 0) > stale;
    const out = {};
    for (const asked of Object.keys(picks)) {
        const kept = {};
        let any = false;
        for (const key of Object.keys(picks[asked])) {
            if (!gone(key, picks[asked][key])) {
                kept[key] = picks[asked][key];
                any = true;
            }
        }
        if (any) {
            out[asked] = kept;
        }
    }
    return out;
}

// The kind of thing a key names, its first part: "notice" in
// "notice:Slack:Hi".
function kindOf(key) {
    return String(key).split(":")[0];
}

// The first word of a name, in lower case, which a row that can't be
// taken back has to be typed past before it's latched.
function firstWord(name) {
    return plainly(name).split(" ")[0] ?? "";
}

// The words that follow "turn" and say which way, as Return's verb says
// them: Turn the volume down is "turn down", not "turn".
const TURNS = ["on", "off", "up", "down"];

// The verb an act's name gives Return: its first word, and for "turn" the
// way it turns, wherever the name says it.
function verbIn(name) {
    const words = plainly(name).split(" ");
    const way = words[0] === "turn" ? words.slice(1).find(word => TURNS.includes(word)) : undefined;
    return way === undefined ? words[0] : `turn ${way}`;
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
