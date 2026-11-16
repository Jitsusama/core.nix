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
.import "MarginCatLedges.js" as Ledges

// Where it can be and its paces (MarginCatLedges.js), by the names its
// routes and the files that route through it have always used.
const WALKS = Ledges.WALKS;
const LEAPS = Ledges.LEAPS;
const MANTLES = Ledges.MANTLES;
const LEAPS_ACROSS = Ledges.LEAPS_ACROSS;
const LEAPS_UP = Ledges.LEAPS_UP;
const sideX = Ledges.sideX;
const stands = Ledges.stands;
const columnAt = Ledges.columnAt;
const ledges = Ledges.ledges;
const standing = Ledges.standing;
const facing = Ledges.facing;
const onScreen = Ledges.onScreen;
const floor = Ledges.floor;
const near = Ledges.near;
const floored = Ledges.floored;
// What hopping across the marks costs, in ms, batting and perching and
// all.
const CROSSES = 2500;
// How much each ms of a stop's way costs more for each screen's height it
// strays off the straight line from where it sets off to its seat.
const STRAYS = 1.5;
// How much further than the nearest side to where it rode it may come
// up, in its widths.
const COMES_UP = 3;

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
    // Off its side's foot down onto the screen's bottom, only where its
    // seat is there (quickest()'s `downTo`): elsewhere the floor is never
    // a short cut, and under a workspace sliding away it's another's.
    const bottom = column.rows[column.rows.length - 1].bottom - at.tall;
    if (at.downTo !== undefined && near(stop.y, bottom) && at.downTo > bottom + 1e-6) {
        add("floor", -1, { x, y: at.downTo, row: 0, how: "falls" }, LEAPS);
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
    ontoFloats(stop, [lowest, highest], at, add);
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

// Whether a move between `one` and `other` would pass through any of
// `at.floats`' faces, which are as solid as a tiled window's top: nothing
// goes through a floating window (VKCF3Z 26038726). Without it the cat
// climbed the gap between two columns behind one (the GPU rig,
// 2026-11-14).
function throughFloats(one, other, at) {
    return (at.floats ?? []).some(it => Math.max(one.x, other.x) + at.wide > it.left + 0.5
        && Math.min(one.x, other.x) < it.right - 0.5
        && Math.max(one.y, other.y) + at.tall > it.top + 0.5
        && Math.min(one.y, other.y) < it.bottom - 0.5);
}

// Where it stands at either end of floating window `it`'s top, as far as
// the screen's edges.
function floatEnds(it, at) {
    return [Math.max(it.left, 0), Math.min(it.right, at.across) - at.wide];
}

// What it lands on, dropping from `y` at `x`: the nearest ledge of `cols`
// or floating window's top under it, all of its box on it, or the screen's
// bottom: { kind, c, y, row }, or null where the screen has no bottom.
function landsOn(x, y, cols, at) {
    let found = null;
    const take = (kind, c, under, row) => {
        if (under > y + 1e-6 && (!found || under < found.y)) {
            found = { kind, c, y: under, row };
        }
    };
    for (const [c, column] of cols.entries()) {
        if (x >= column.left - 0.5 && x + at.wide <= column.right + 0.5) {
            for (const ledge of ledges(column, at)) {
                take("ledge", c, ledge.y, ledge.row);
            }
        }
    }
    for (const [f, it] of (at.floats ?? []).entries()) {
        if (x >= it.left - 0.5 && x + at.wide <= it.right + 0.5) {
            take("float", f, stands(it.top, at), 0);
        }
    }
    return found ?? (at.height === undefined ? null
        : { kind: "floor", c: -1, y: floor(at), row: 0 });
}

// From a ledge, along it `reach` ([lowest, highest]), onto each floating
// window as onto a ledge of a column (WCJQSG ade5ce02: "it climbs onto,
// jumps off and walks along floats as it does tiled ones"): along to
// beside its edges, which its face may hide the rest of the ledge past;
// and from beside an edge, a leap up it within LEAPS_UP of its heights
// and over, never across its face as it rises. Down onto one from over
// it on the way is owed.
function ontoFloats(stop, reach, at, add) {
    const [lowest, highest] = reach;
    const walk = x => (Math.abs(x - stop.x) / at.cell / WALKS) * 1000;
    const along = x => x >= lowest - 1e-6 && x <= highest + 1e-6 && !near(x, stop.x);
    for (const [f, it] of (at.floats ?? []).entries()) {
        const y = stands(it.top, at);
        const [left, right] = floatEnds(it, at);
        for (const x of [it.left - at.wide, it.right].filter(along)) {
            add("ledge", stop.c, { x, y: stop.y, row: stop.row, how: "goes" }, walk(x));
        }
        const side = near(stop.x, it.left - at.wide) ? -1 : near(stop.x, it.right) ? 1 : 0;
        if (side !== 0 && y <= stop.y + 1e-6 && stop.y - y <= LEAPS_UP * at.tall
            && right >= left) {
            add("float", f, { x: side < 0 ? left : right, y, row: 0, side, how: "leapsOn",
                via: { x: stop.x, y: it.top - at.tall } }, LEAPS + MANTLES);
        }
    }
}

// Every stop it can reach from a floating window's top with one move, as
// from a ledge: along it to either end, and from an end, a hop clear and
// down onto what's under, or across onto a ledge of the column it faces
// within a leap up or down. Straight up off it onto a ledge over it is
// owed, as is a leap from one floating window to another.
function fromFloat(stop, cols, at, add) {
    const it = at.floats[stop.c];
    const [lowest, highest] = floatEnds(it, at);
    if (highest < lowest) {
        return;
    }
    for (const x of [lowest, highest].filter(one => !near(one, stop.x))) {
        add("float", stop.c, { x, y: stop.y, row: 0, how: "goes" },
            (Math.abs(x - stop.x) / at.cell / WALKS) * 1000);
    }
    const dir = near(stop.x, highest) ? 1 : near(stop.x, lowest) ? -1 : 0;
    if (dir === 0) {
        return;
    }
    const clear = dir > 0 ? it.right : it.left - at.wide;
    const under = onScreen(clear, at) ? landsOn(clear, stop.y, cols, at) : null;
    if (under) {
        add(under.kind, under.c, { x: clear, y: under.y, row: under.row, how: "hopsOff",
            via: { x: clear, y: stop.y } }, LEAPS);
    }
    for (const [c, column] of cols.entries()) {
        const gap = dir > 0 ? column.left - it.right : it.left - column.right;
        if (gap < 0 || gap > LEAPS_ACROSS * at.wide) {
            continue;
        }
        const x = dir > 0 ? Math.max(column.left, 0) : Math.min(column.right, at.across) - at.wide;
        for (const ledge of ledges(column, at)
            .filter(one => Math.abs(one.y - stop.y) <= LEAPS_UP * at.tall)) {
            add("ledge", c, { x, y: ledge.y, row: ledge.row, how: "leaps" }, LEAPS);
        }
    }
}

// Whether a walk along the tops of `cols` between `one` and `other`, each
// an `x`, would pass through a floating window's face, where it takes the
// way over it instead (fromLedge()'s ontoFloats()).
function blocked(one, other, cols, at) {
    const low = Math.min(one, other);
    const high = Math.max(one, other) + at.wide;
    return cols.some(column => {
        const from = Math.max(low, column.left);
        const to = Math.min(high, column.right);
        const y = stands(column.rows[0].top, at);
        return to > from && (at.floats ?? []).some(it => it.left < to - 0.5
            && it.right > from + 0.5 && y + at.tall > it.top + 0.5 && y < it.bottom - 0.5);
    });
}

// Whether from `start` it walks along the tops to `goal` with no route:
// both on them, and no floating window's face across them between.
function alongTops(start, goal, cols, at) {
    return start.kind === "ledge" && start.row === 0
        && (standing(cols, goal, at)?.row ?? 0) === 0 && !blocked(start.x, goal.x, cols, at);
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
        // A move by way of a point, up beside an edge or clear of one, is
        // through no face where neither of its two legs is.
        const through = more.via ? throughFloats(stop, more.via, at)
            || throughFloats(more.via, more, at) : throughFloats(stop, more, at);
        if (!inMarks(more.x, more.y, at) && (!past || along) && !through) {
            found.push({ stop: Object.assign({ kind, c }, more), ms: ms + (past ? CROSSES : 0) });
        }
    };
    const from = { side: fromSide, ledge: fromLedge, floor: fromFloor, float: fromFloat }[
        stop.kind];
    from(stop, cols, at, add);
    return found;
}

// The quickest way from the stops it may set off from, `open`, to its seat
// `to` on `cols` (MarginCatPaths.js's columns()): each stop in turn, each
// with `how` it got there, the seat left out. On its seat's ledge it walks
// to it; on the tops, its seat on one, it walks along them to it, leaping
// each gap between. Null where there's no way.
function quickest(open, trip, cols, at) {
    const { from, to } = trip;
    // A seat on the screen's bottom is walked to along it, never along
    // the tops.
    const low = floored(cols, to, at);
    const goal = standing(cols, to, at) ?? { c: -1, row: low ? -1 : 0 };
    const reach = low ? Object.assign({}, at, { downTo: floor(at) }) : at;
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
        const seat = { kind: "seat", c: -1, x: to.x, y: to.y };
        if (low && here.stop.kind === "floor") {
            const ms = (Math.abs(to.x - here.stop.x) / at.cell / WALKS) * 1000;
            open.push({ stop: seat, ms: here.ms + ms * strays(here.stop, seat), back: here });
        }
        if (here.stop.kind === "ledge" && (goal.row === 0 ? here.stop.row === 0
            && !blocked(here.stop.x, to.x, cols, at)
            : here.stop.c === goal.c && here.stop.row === goal.row)) {
            const gaps = cols.filter(one => one.left > Math.min(here.stop.x, to.x) + at.wide
                && one.left < Math.max(here.stop.x, to.x) + at.wide).length;
            const ms = (Math.abs(to.x - here.stop.x) / at.cell / WALKS) * 1000 + gaps * LEAPS;
            open.push({ stop: seat, ms: here.ms + ms * strays(here.stop, seat), back: here });
        }
        for (const step of next(here.stop, cols, reach)) {
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

// The quickest way from any `stop` to its seat `to`, beginning with it.
function route(stop, to, cols, at) {
    return quickest([{ stop, ms: 0, back: null }], { from: stop, to }, cols, at);
}

// The quickest way from `stop` to the foot of each side of `cols` on the
// screen, where it can drop off a workspace into the one below
// (MarginCatStacked.js), or with `heads` to the head of each, where it
// leaps up off one into the one above: each as { stops, ms }, its stops
// beginning with `stop` and ending at the foot or head, and what it takes
// in ms.
function feet(stop, cols, at, heads) {
    const found = [];
    const done = {};
    const open = [{ stop, ms: 0, back: null }];
    const foot = one => one.kind === "side" && (heads
        ? one.row === 0 && near(one.y, cols[one.c].rows[0].top - at.tall)
        : one.row === cols[one.c].rows.length - 1 && near(one.y, cols[one.c].bottom - at.tall));
    while (open.length > 0) {
        open.sort((one, other) => one.ms - other.ms);
        const here = open.shift();
        const id = key(here.stop);
        if (done[id]) {
            continue;
        }
        done[id] = true;
        if (foot(here.stop)) {
            const stops = [];
            for (let link = here; link; link = link.back) {
                stops.unshift(link.stop);
            }
            found.push({ stops, ms: here.ms });
        }
        for (const step of next(here.stop, cols, at)) {
            if (!done[key(step.stop)]) {
                open.push({ stop: step.stop, ms: here.ms + step.ms, back: here });
            }
        }
    }
    return found;
}
