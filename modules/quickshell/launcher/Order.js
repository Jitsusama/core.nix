// The one order each kind of list keeps with nothing typed, so every list
// of a kind moves the same way (docs/design.md's Lists, "Each kind of list
// keeps one order"; RSCH-20261030-GB56HX). A list moves only by what Joel
// does, never by a guess, and a choice never moves when it's chosen; only
// typing ranks (Rank.js), and a list shown keeps its rows still while it's
// open (Steady.js). Each order here is a comparator, and `sorted` sorts by
// one keeping rows alike in it in the order they came, which Qt's engine's
// own Array.sort doesn't: it leaves four rows alike as 2 3 4 1. A fixed
// set and a search have none: a fixed set keeps the order it was designed
// in, and a search is Rank.js's. tests/every-list-keeps-its-kinds-order.py holds each place
// to its kind's order here. Apart from the launcher, so tests/order.qml can
// hold it in Qt's own engine.
.pragma library

// The first of the orders that tells two rows apart. Looped by index,
// since a sort calls it for every pair it weighs, and for...of makes an
// iterator and a result for each step of each call: on the laptop that
// was most of an opening's seconds (2026-11-10).
function then(...orders) {
    return (one, other) => {
        // biome-ignore lint/style/useForOf: Qt's engine makes an iterator result per step.
        for (let at = 0; at < orders.length; at += 1) {
            const said = orders[at](one, other);
            if (said !== 0) {
                return said;
            }
        }
        return 0;
    };
}

// Whether a row has a time or an amount: a number, or a time written as
// ISO 8601, which reads in order as it's written.
function has(value) {
    return value !== undefined && value !== null && value !== "" && !Number.isNaN(value);
}

// By a time or amount, the larger first or the smaller, a row without one
// after every row with one.
function by(of, largerFirst) {
    return (one, other) => {
        const a = of(one);
        const b = of(other);
        if (!(has(a) && has(b))) {
            return Number(has(b)) - Number(has(a));
        }
        if (a === b) {
            return 0;
        }
        return (a < b) === largerFirst ? 1 : -1;
    };
}

function larger(of) {
    return by(of, true);
}

function smaller(of) {
    return by(of, false);
}

// A name's runs of digits and of the rest, the rest in lower case.
const RUNS = /\d+|\D+/g;
const DIGITS = /^\d/;

function pieces(name) {
    return String(name ?? "").toLowerCase().match(RUNS) ?? [];
}

// A to Z, as names are read: numbers by their value, so Tab 9 comes before
// Tab 10, and case alike. Qt's engine leaves out localeCompare's numeric
// option, so the digits are read here. It's only ever the last tie, or the
// order of many peers alike, as a large folder's files.
function byName(name) {
    return byPieces(one => pieces(name(one)));
}

// The same, from each one's name already broken into pieces (`pieces`),
// for a long list sorted often, which would otherwise break both names
// apart at each of its comparisons, as People's 500 did at every opening
// (2026-11-10).
function byPieces(of) {
    return (one, other) => {
        const a = of(one);
        const b = of(other);
        for (let at = 0; at < Math.min(a.length, b.length); at += 1) {
            const numbers = DIGITS.test(a[at]) && DIGITS.test(b[at]);
            const said = numbers ? Number(a[at]) - Number(b[at]) : a[at].localeCompare(b[at]);
            if (said !== 0) {
                return said < 0 ? -1 : 1;
            }
        }
        return a.length - b.length;
    };
}

// Things returned to, as Programs, People and Quests: open or a favourite
// first, those in the order Joel put them in where he did (`placed`, a
// quest's priority), then by the last thing he did with each, the newest
// first, then A to Z. `first` says whether one is open or a favourite,
// `last` when he last did something with it (or nothing, where that isn't
// known), `name` what it's called, or `pieces` its name as `pieces` breaks
// it, worked out once.
function returned(how) {
    const first = one => Boolean(how.first(one));
    return then((one, other) => Number(first(other)) - Number(first(one)),
        (one, other) => first(one) && first(other) && how.placed
            ? how.placed(one, other) : 0,
        larger(how.last ?? (() => null)),
        how.pieces ? byPieces(how.pieces) : byName(how.name ?? (() => "")));
}

// A stream, as Recent, Notices and mail: the newest first, by `at`, a
// number or an ISO 8601 time, and those alike in time in the order they
// came. `first` puts those still going ahead, as a run still running.
function stream(at, first) {
    const going = first ?? (() => false);
    return then((one, other) => Number(Boolean(going(other))) - Number(Boolean(going(one))),
        larger(at));
}

// Live states, as Sessions, devices and Wi-Fi: by how each stands, its
// state's place in the order given (`stands`, the most pressing first), then
// when it started (`since`), the earliest first so one that starts comes
// after those there, then by `then` where a kind has one more measure (a
// network's strength), then A to Z.
function live(how) {
    return then(smaller(how.stands), smaller(how.since ?? (() => null)),
        how.then ?? (() => 0), byName(how.name));
}

// Amounts, as Processor, Memory, Traffic and disks: the worst first, the
// largest by `amount`, or by `worse` where worse isn't only larger (a disk
// nearly full before a larger one with room), then A to Z.
function amounts(how) {
    return then(how.worse ?? larger(how.amount), byName(how.name));
}

// Choices, as Scale, Refresh and sound's devices: their natural order, a
// scale by its size, a device by its name, given by `natural` where it isn't
// the name, then A to Z. Which one is chosen is never part of it: it's said
// where it stands, so choosing one never moves it.
function choices(how) {
    return then(how.natural ?? (() => 0), byName(how.name));
}

// The rows in an order, those alike in it in the order they came, as
// Array.sort alone doesn't keep them in Qt's engine.
function sorted(rows, order) {
    return Array.from(rows ?? [], (row, at) => ({ row, at }))
        .sort((one, other) => order(one.row, other.row) || one.at - other.at)
        .map(each => each.row);
}

// Where rows listed in this order first break it: the place of the first
// row that should come after the next one, or -1 where none does.
function broken(rows, order) {
    for (let at = 0; at + 1 < rows.length; at += 1) {
        if (order(rows[at], rows[at + 1]) > 0) {
            return at;
        }
    }
    return -1;
}
