// The way the margin cat picks through a workspace's windows
// (MarginCatPaths.js), as a cat picks its way through branches: up or down
// the side of a window, onto the ledge where two windows stacked in a
// column leave a gap between them or a column's top, along it, across the
// gap between two columns with a leap, and on, until it's on the ledge it
// sits on (Joel, 2026-11-13: "i also want it to climb up, which will
// require it pathfinding through the spaces between windows"; "i might be
// on a window in the second row and it needs to travel to it. or I am in a
// column on the left and change workspaces where the last focused window
// is on the right"; WCJQSG b36c3fb6: a route across more than one window,
// leaping a gap mid-climb). Each place it can be is a stop: hanging on a
// column's side, or standing on a ledge, a column's top being its first;
// the search finds the quickest run of them to its seat, keeping near the
// straight line there, as a cat makes for where it's going.
.pragma library

// Its pace, in its cells a second, walking a ledge or a top or climbing
// a side, which it does at the pace it walks (Joel, 2026-11-13 17:22, by
// WCJQSG fc3f716a: at 320 a climb "seems accelerated").
const WALKS = 120;
// What a leap costs, crouch and landing and all, in ms: across a gap
// between two of a column's windows or two columns, and over an edge onto
// a ledge or a side.
const LEAPS = 330;
const MANTLES = 300;
// How far across it leaps between two columns, in its own widths.
const LEAPS_ACROSS = 6;
// What hopping across the marks costs, in ms, batting and perching and
// all.
const CROSSES = 2500;
// How much each ms of a stop's way costs more for each screen's height it
// strays off the straight line from where it sets off to its seat.
const STRAYS = 1.5;
// How much further than the nearest side to where it rode it may come
// up, in its widths.
const COMES_UP = 3;
// How near its seat's height a place must be to stand there, in cells.
const STANDS_WITHIN = 1;

// Where it hangs on a column's side (-1 left, 1 right), the `x` of its box.
function sideX(column, side, at) {
    return side < 0 ? column.left - at.wide : column.right;
}

// Where it stands on a ledge or a column's top whose window's top is `top`.
function stands(top, at) {
    return top - at.tall - at.air;
}

// The ledge over each window of a column it fits on: its top, the margin
// above it, and each gap between two windows stacked in it it fits in, as
// { row, y } by its window's index.
function ledges(column, at) {
    const found = [{ row: 0, y: stands(column.rows[0].top, at) }];
    for (let r = 1; r < column.rows.length; r++) {
        const room = column.rows[r].top - column.rows[r - 1].bottom;
        if (room >= at.tall + at.air) {
            found.push({ row: r, y: stands(column.rows[r].top, at) });
        }
    }
    return found;
}

// Which ledge of `cols` it stands on at `place` ({ x, y }), any of its
// box over it: { c, row }, or null for none.
function standing(cols, place, at) {
    for (const [c, column] of cols.entries()) {
        if (place.x < column.right && place.x + at.wide > column.left) {
            const ledge = ledges(column, at)
                .find(one => Math.abs(one.y - place.y) <= STANDS_WITHIN * at.cell);
            return ledge ? { c, row: ledge.row } : null;
        }
    }
    return null;
}

// The column beside `c` on its `side`, near enough to leap to: its index
// `c`, and `near`, where it stands at its nearer end.
function facing(cols, c, side, at) {
    const column = cols[c + side];
    if (!column) {
        return null;
    }
    const gap = side > 0 ? column.left - cols[c].right : cols[c].left - column.right;
    return gap >= 0 && gap <= LEAPS_ACROSS * at.wide
        ? { column, c: c + side, near: Math.min(Math.max(side > 0 ? column.left
            : column.right - at.wide, 0), at.across - at.wide) } : null;
}

function onScreen(x, at) {
    return x >= 0 && x + at.wide <= at.across;
}

// Where it stands on the screen's bottom, below every window.
function floor(at) {
    return at.height - at.tall;
}

// Every stop it can reach from the screen's bottom: along it, and up
// onto the bottom of any window's side on the screen.
function fromFloor(stop, cols, at, add) {
    for (const [c, column] of cols.entries()) {
        for (const side of [-1, 1]) {
            const x = sideX(column, side, at);
            if (onScreen(x, at)) {
                add("side", c, { side, x, y: column.bottom - at.tall,
                    row: column.rows.length - 1, how: "springs" },
                    (Math.abs(x - stop.x) / at.cell / WALKS) * 1000 + LEAPS);
            }
        }
    }
}

// Whether its box at `x` and `y` would lie in the ink of the margin's
// marks (MarginCatPaths.js's marks()), the middle half of each mark's
// cell, which are as solid as a window: it crosses them only by hopping
// from one to the next along the tops (MarginCatPaths.js's perches()).
function inMarks(x, y, at) {
    const lit = at.marks;
    return !!lit && x < lit.to - lit.each / 4 && x + at.wide > lit.from + lit.each / 4
        && y < (lit.each * 3) / 4 && y + at.tall > lit.each / 4;
}

// A stop by where it is to a hundredth of a pixel, so one a fraction off
// a ledge's end is never taken for the end.
function key(stop) {
    const at = one => Math.round(one * 100);
    return `${stop.kind}:${stop.c}:${stop.side ?? 0}:${at(stop.x)}:${at(stop.y)}`;
}

function near(one, other) {
    return Math.abs(one - other) < 1e-6;
}

// Every stop it can reach from a side with one move, each with its cost
// in ms: up or down to the top or bottom of each of its windows, to the
// height of a gap between two of them it fits in, or to the height of a
// ledge across from it; over onto the ledge at a window's top, or in
// onto the ledge in a gap at its height; and across onto a ledge level
// with it.
function fromSide(stop, cols, at, add) {
    const column = cols[stop.c];
    const x = stop.x;
    const beside = facing(cols, stop.c, stop.side, at);
    const across = beside ? ledges(beside.column, at) : [];
    const own = ledges(column, at);
    const inGaps = own.filter(one => one.row > 0);
    // Its column's top corner under the marks, it climbs only as high as
    // their ink's bottom, and hops up onto them from there.
    const lit = at.marks;
    const topped = column.rows[0].top - at.tall;
    const marked = !!lit && inMarks(x, topped, at);
    const highest = marked ? (lit.each * 3) / 4 : topped;
    const heights = column.rows.reduce((all, one) =>
        all.concat([one.top - at.tall, one.bottom - at.tall]), [])
        .map(y => near(y, topped) ? highest : y)
        .concat(across.map(ledge => ledge.y), inGaps.map(ledge => ledge.y));
    for (const y of heights.filter(one => !near(one, stop.y))) {
        const gap = inGaps.find(one => near(one.y, y));
        const r = gap ? gap.row : column.rows.findIndex(one => y >= one.top - at.tall - 1e-6
            && y <= one.bottom - at.tall + 1e-6);
        if (r >= 0) {
            const gaps = column.rows.filter(one => one.top > Math.min(y, stop.y) + at.tall
                && one.top <= Math.max(y, stop.y) + at.tall).length;
            add("side", stop.c, { side: stop.side, x, y, row: r, how: "climbs" },
                (Math.abs(stop.y - y) / at.cell / WALKS) * 1000 + gaps * LEAPS);
        }
    }
    for (const ledge of across.filter(one => Math.abs(one.y - stop.y) <= at.cell + 1e-6)) {
        add("ledge", beside.c, { x: beside.near, y: ledge.y, row: ledge.row, how: "leaps" },
            LEAPS);
    }
    // Its column's end on its side, as far as the screen's edge.
    const end = Math.min(Math.max(stop.side < 0 ? column.left : column.right - at.wide, 0),
        at.across - at.wide);
    const row = column.rows[stop.row];
    const over = own.find(one => one.row === stop.row);
    if (over && near(stop.y, row.top - at.tall)) {
        add("ledge", stop.c, { x: end, y: over.y, row: stop.row, how: "mantles" }, MANTLES);
    }
    const underMarks = marked ? near(stop.y, highest)
        : !!lit && near(stop.y, topped) && inMarks(end, own[0].y, at);
    if (stop.row === 0 && underMarks) {
        // Onto the marks, and off them either way onto its top.
        for (const dir of [-1, 1]) {
            const past = dir < 0 ? lit.from - at.wide - lit.each / 2 : lit.to + lit.each / 2;
            if (past >= Math.max(column.left, 0)
                && past <= Math.min(column.right, at.across) - at.wide) {
                add("ledge", stop.c, { x: past, y: own[0].y, row: 0, how: "mantles",
                    perches: dir }, MANTLES + CROSSES);
            }
        }
    }
    // In from beside a gap, level, since from higher or lower it would
    // go through the window above or below the gap.
    for (const gap of inGaps.filter(one => near(one.y, stop.y))) {
        add("ledge", stop.c, { x: end, y: gap.y, row: gap.row, how: "hops" }, LEAPS);
    }
}

// Every stop it can reach from a ledge with one move: along it to either
// end, and from an end out of a gap onto its column's side level with
// it, over the edge onto its own window's side, or across to the next
// column's ledges level with it or the side facing it. Every leap off a
// ledge, or onto one in a gap, is level: a leap up or down beside a
// window would go through its corner.
function fromLedge(stop, cols, at, add) {
    const column = cols[stop.c];
    // Its ends, as far as the screen's edges where it runs off them.
    const lowest = Math.max(column.left, 0);
    const highest = Math.min(column.right, at.across) - at.wide;
    for (const x of [lowest, highest].filter(one => highest >= lowest && !near(one, stop.x))) {
        add("ledge", stop.c, { x, y: stop.y, row: stop.row, how: "goes" },
            (Math.abs(x - stop.x) / at.cell / WALKS) * 1000);
    }
    // Along the tops across the marks, whose ends under them it can't
    // stand on, onto a top just past them.
    const lit = at.marks;
    if (stop.row === 0 && lit) {
        for (const x of [lit.from - at.wide - lit.each / 2, lit.to + lit.each / 2]) {
            const on = standing(cols, { x, y: stop.y }, at);
            const low = Math.min(stop.c, on?.c ?? -1);
            const high = Math.max(stop.c, on?.c ?? -1);
            const level = on?.row === 0 && cols.slice(low, high + 1).every((one, i, all) =>
                near(one.rows[0].top, column.rows[0].top)
                && (i === 0 || one.left - all[i - 1].right <= LEAPS_ACROSS * at.wide));
            if (level && on.c !== stop.c && x >= 0 && x + at.wide <= at.across) {
                add("ledge", on.c, { x, y: stop.y, row: 0, how: "goes" },
                    (Math.abs(x - stop.x) / at.cell / WALKS) * 1000);
            }
        }
    }
    // Its top's end under the marks, which it can't stand on: across them
    // and down onto its window's side under them.
    for (const way of [-1, 1].filter(one => stop.row === 0 && lit
        && inMarks(one < 0 ? lowest : highest, stop.y, at))) {
        const hangs = sideX(column, way, at);
        if (onScreen(hangs, at)) {
            const before = way > 0 ? lit.from - at.wide - lit.each / 2 : lit.to + lit.each / 2;
            add("side", stop.c, { side: way, x: hangs, y: (lit.each * 3) / 4, row: 0, how: "perches",
                perches: way }, (Math.abs(before - stop.x) / at.cell / WALKS) * 1000 + LEAPS);
        }
    }
    const dir = stop.x >= column.right - at.wide - 1e-6 ? 1
        : stop.x <= column.left + 1e-6 ? -1 : 0;
    if (dir === 0) {
        return;
    }
    const side = sideX(column, dir, at);
    if (stop.row > 0 && onScreen(side, at)) {
        add("side", stop.c, { side: dir, x: side, y: stop.y, row: stop.row, how: "leaps" },
            LEAPS);
    }
    if (onScreen(side, at)) {
        add("side", stop.c, { side: dir, x: side, y: column.rows[stop.row].top - at.tall,
            row: stop.row, how: "over" }, MANTLES);
    }
    const facingIt = facing(cols, stop.c, dir, at);
    if (!facingIt) {
        return;
    }
    const { c: d, column: beside } = facingIt;
    for (const ledge of ledges(beside, at).filter(one => Math.abs(one.y - stop.y) <= at.cell)) {
        add("ledge", d, { x: facingIt.near, y: ledge.y, row: ledge.row, how: "leaps" }, LEAPS);
    }
    const x = sideX(beside, -dir, at);
    const row = beside.rows.findIndex(one => stop.y >= one.top - at.tall
        && stop.y <= one.bottom - at.tall);
    if (row >= 0 && onScreen(x, at)) {
        add("side", d, { side: -dir, x, y: stop.y, row, how: "leaps" }, LEAPS);
    }
}

function next(stop, cols, at) {
    const found = [];
    // Never into the marks, and past them only along the tops, where it
    // hops from mark to mark (MarginCatPaths.js's perches()).
    const add = (kind, c, more, ms) => {
        const past = inMarks(Math.min(stop.x, more.x), Math.min(stop.y, more.y),
            Object.assign({}, at, { wide: Math.abs(more.x - stop.x) + at.wide,
                tall: Math.abs(more.y - stop.y) + at.tall }));
        const along = (more.how === "goes" && more.row === 0) || !!more.perches;
        if (!inMarks(more.x, more.y, at) && (!past || along)) {
            found.push({ stop: Object.assign({ kind, c }, more), ms: ms + (past ? CROSSES : 0) });
        }
    };
    const from = { side: fromSide, ledge: fromLedge, floor: fromFloor }[stop.kind];
    from(stop, cols, at, add);
    return found;
}

// The quickest way from the stops it may set off from, `open`, to its seat
// `to` on `cols` (MarginCatPaths.js's columns()): each stop in turn, each
// with `how` it got there, the seat left out. On its seat's ledge it walks
// to it; on the tops, its seat on one, it walks along them to it, leaping
// each gap between. Given `ends`, it ends instead at the first stop that
// `ends` takes, `to` only where it makes for. Null where there's no way.
function quickest(open, trip, cols, at) {
    const { from, to, ends } = trip;
    const goal = standing(cols, to, at) ?? { c: -1, row: 0 };
    const length = Math.hypot(to.x - from.x, to.y - from.y);
    const line = stop => {
        const t = length === 0 ? 0 : Math.min(Math.max(((stop.x - from.x) * (to.x - from.x)
            + (stop.y - from.y) * (to.y - from.y)) / (length * length), 0), 1);
        return Math.hypot(stop.x - from.x - (to.x - from.x) * t,
            stop.y - from.y - (to.y - from.y) * t) / Math.max(at.height ?? at.across, 1);
    };
    const strays = (one, other) => 1 + (STRAYS * (line(one) + line(other))) / 2;
    const done = {};
    while (open.length > 0) {
        open.sort((one, other) => one.ms - other.ms);
        const here = open.shift();
        const id = key(here.stop);
        if (done[id]) {
            continue;
        }
        done[id] = true;
        if (here.stop.kind === "seat") {
            const stops = [];
            for (let link = here.back; link; link = link.back) {
                stops.unshift(link.stop);
            }
            return stops;
        }
        if (ends?.(here.stop)) {
            // Ending here, it still has the way across to `to` to go.
            const end = { kind: "seat", c: -1, x: here.stop.x, y: here.stop.y };
            const ms = (Math.abs(to.x - here.stop.x) / at.cell / WALKS) * 1000;
            open.push({ stop: end, ms: here.ms + ms, back: here });
        }
        const seat = { kind: "seat", c: -1, x: to.x, y: to.y };
        if (!ends && here.stop.kind === "ledge" && (goal.row === 0 ? here.stop.row === 0
            : here.stop.c === goal.c && here.stop.row === goal.row)) {
            const gaps = cols.filter(one => one.left > Math.min(here.stop.x, to.x) + at.wide
                && one.left < Math.max(here.stop.x, to.x) + at.wide).length;
            const ms = (Math.abs(to.x - here.stop.x) / at.cell / WALKS) * 1000 + gaps * LEAPS;
            open.push({ stop: seat, ms: here.ms + ms * strays(here.stop, seat), back: here });
        }
        for (const step of next(here.stop, cols, at)) {
            if (!done[key(step.stop)]) {
                open.push({ stop: step.stop, ms: here.ms + step.ms * strays(here.stop, step.stop),
                    back: here });
            }
        }
    }
    return null;
}

// The quickest way up from `from`, under the screen where it rode, to its
// seat `to` on `cols`: from the side of a column's bottom window it comes
// up onto, by where it rode, since what it did out of sight is never a
// short cut. Null where no column has a side on the screen.
function up(from, to, cols, at) {
    const sides = cols.reduce((all, column, c) => all.concat([-1, 1].map(side => ({
        kind: "side", c, side, x: sideX(column, side, at), row: column.rows.length - 1,
        y: column.rows[column.rows.length - 1].bottom - at.tall, how: "comes"
    }))), []).filter(stop => onScreen(stop.x, at));
    const nearest = Math.min(...sides.map(stop => Math.abs(stop.x - from.x)));
    const under = cols.findIndex(column => from.x + at.wide / 2 >= column.left
        && from.x + at.wide / 2 <= column.right);
    const open = sides.filter(stop => stop.c === under
        || Math.abs(stop.x - from.x) <= nearest + COMES_UP * at.wide)
        .map(stop => ({ stop, ms: LEAPS, back: null }));
    return open.length > 0 ? quickest(open, { from, to }, cols, at) : null;
}

// The quickest way from a ledge it stands on at `from` to its seat `to`,
// beginning with where it stands. Null where it stands on none, or there's
// no way.
function between(from, to, cols, at) {
    const on = standing(cols, from, at);
    if (on === null) {
        return null;
    }
    const stop = { kind: "ledge", c: on.c, x: from.x, y: from.y, row: on.row, how: "stands" };
    return quickest([{ stop, ms: 0, back: null }], { from, to }, cols, at);
}

// The quickest way from any `stop` to its seat `to`, beginning with it.
function route(stop, to, cols, at) {
    return quickest([{ stop, ms: 0, back: null }], { from: stop, to }, cols, at);
}

// The quickest way from a ledge it stands on at `from` off its window,
// `making` for `toward`: down onto a column's side at its bottom, `down`,
// to drop from there where `lands` takes its `x`; or else onto a
// column's top, to leap up from. Null where it stands on none, or
// there's no way.
function off(from, cols, at, making) {
    const { toward, down, lands } = making;
    const on = standing(cols, from, at);
    if (on === null) {
        return null;
    }
    const stop = { kind: "ledge", c: on.c, x: from.x, y: from.y, row: on.row, how: "stands" };
    const ends = down
        ? one => one.kind === "side" && near(one.y, cols[one.c].bottom - at.tall)
            && (!lands || lands(one.x))
        : one => one.kind === "ledge" && one.row === 0;
    return quickest([{ stop, ms: 0, back: null }], { from, to: toward, ends }, cols, at);
}
