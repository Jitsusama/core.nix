// Where the margin cat can be on a workspace's columns (MarginCatPaths.js's
// columns()), and the paces its routes are timed at: on a ledge, a
// column's top or a gap between two of its windows it fits in; hanging on
// a column's side; or on the screen's bottom. MarginCatRoutes.js picks its
// way through them, and MarginCatFloats.js reads them for the floating
// windows' tops, so both take them from here.
.pragma library
.import "MarginCat.js" as MarginCat

// Its pace, in its cells a second, walking a ledge or a top or climbing
// a side, which it does at the pace it walks (Joel, 2026-11-13 17:22, by
// WCJQSG fc3f716a: at 320 a climb "seems accelerated"): the walker's own
// cap on a long way, so a route planned here is timed as it's walked, and
// one way down or up is weighed against another as the walker would take
// it.
const WALKS = MarginCat.HURRIES;
// What a leap costs, crouch and landing and all, in ms: across a gap
// between two of a column's windows or two columns, and over an edge onto
// a ledge or a side.
const LEAPS = 330;
const MANTLES = 300;
// How far across it leaps between two columns, in its own widths.
const LEAPS_ACROSS = 6;
// The highest it leaps up, in its heights: a cat jumps about five times
// its height, and a longer leap reads as flying.
const LEAPS_UP = 4;
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

// The column under the cat's middle at `x`, or the nearest one to it.
function columnAt(cols, x, wide) {
    const middle = x + wide / 2;
    let nearest = null;
    let far = Infinity;
    for (const column of cols) {
        const away = Math.max(column.left - middle, middle - column.right, 0);
        if (away < far) {
            far = away;
            nearest = column;
        }
    }
    return nearest;
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

function near(one, other) {
    return Math.abs(one - other) < 1e-6;
}

// Whether `place` is on the screen's bottom rather than any ledge: where
// it climbs a floating window's side from with nothing under it
// (MarginCatFloats.js's onto()).
function floored(cols, place, at) {
    return at.height !== undefined && near(place.y, floor(at))
        && standing(cols, place, at) === null;
}

// The bottom of the lowest window of `cols` over a leap from `from` to
// `to`, `wide` across, which its arc mustn't rise into; or none.
function ceiling(cols, from, to, wide) {
    const low = Math.min(from.x, to.x);
    const high = Math.max(from.x, to.x) + wide;
    const under = Math.min(from.y, to.y);
    let bottom = -Infinity;
    for (const column of cols ?? []) {
        if (column.right <= low || column.left >= high) {
            continue;
        }
        for (const row of column.rows) {
            if (row.bottom <= under + 1e-6) {
                bottom = Math.max(bottom, row.bottom);
            }
        }
    }
    return bottom;
}
