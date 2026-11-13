// What the margin cat does when the windows change under it
// (MarginCatPaths.js's walk): a window grown into it shoves it clear; a
// window gone from under it, or shrunk or moved away, leaves it in the
// air, where it grabs an edge in reach, or falls with weight onto
// whatever is below, lands on its feet and shakes it off (Joel,
// 2026-11-13, by WCJQSG 2bd7c7c6: "the kitten also needs to handle
// resizing windows, even if those leave it suspended in air").
.pragma library
.import "MarginCat.js" as MarginCat
.import "MarginCatRoutes.js" as Routes
.import "MarginCatWays.js" as Ways

// How long a window takes at least to shove it clear, in ms, and how fast
// at most, in its heights a second: half its body between two frames at
// 120 Hz, near enough, so a long shove is never a jump.
const SHOVES = 140;
const SHOVED_AT_MOST = 54;
// How far off an edge it can grab as it starts to fall, in its widths
// across and its heights down.
const GRABS_ACROSS = 1;
const GRABS_DOWN = 1.5;
// How long it shakes itself off after a fall, in ms.
const SHAKES = 320;
// How far above it, in its cells, a ledge may be that it scrambles onto
// rather than falls past.
const SCRAMBLES = 2;

// How far its box at `place` lies in a window of `cols`, beyond half a
// pixel each way: the row's { column, row }, or null.
function within(place, cols, at) {
    for (const column of cols) {
        if (place.x < column.right - 0.5 && place.x + at.wide > column.left + 0.5) {
            const row = column.rows.find(one => place.y < one.bottom - 0.5
                && place.y + at.tall > one.top + 0.5);
            if (row) {
                return { column, row };
            }
        }
    }
    return null;
}

// The nearest place to `place` clear of every window of `cols` and on
// the screen: above the window it's in, or beside it, or under it.
function clear(place, cols, at, inWindow) {
    const { column, row } = inWindow;
    const tries = [
        { x: place.x, y: row.top - at.tall - at.air },
        { x: column.left - at.wide, y: place.y },
        { x: column.right, y: place.y },
        { x: place.x, y: row.bottom + at.air }
    ].filter(one => Routes.onScreen(one.x, at) && one.y >= 0
        && one.y + at.tall <= (at.height ?? Infinity) && within(one, cols, at) === null);
    const far = one => Math.hypot(one.x - place.x, one.y - place.y);
    return tries.reduce((most, one) => most === null || far(one) < far(most) ? one : most, null);
}

// Where it can stand or hang at `place` on `cols`: a stop as
// MarginCatRoutes.js's are, or null where it's in the air.
function holding(place, cols, at) {
    if (cols.length === 0 || at.height === undefined) {
        // With no windows, or no screen's height, it walks the margin
        // at the height it's at.
        return { kind: "ledge", c: -1, x: place.x, y: place.y, row: 0, how: "stands" };
    }
    const on = Routes.standing(cols, place, at);
    if (on !== null) {
        const y = Routes.stands(cols[on.c].rows[on.row].top, at);
        return { kind: "ledge", c: on.c, x: place.x, y, row: on.row, how: "stands" };
    }
    if (at.height !== undefined && Math.abs(place.y - Routes.floor(at)) <= at.cell) {
        return { kind: "floor", c: -1, x: place.x, y: Routes.floor(at), row: 0, how: "stands" };
    }
    for (const [c, column] of cols.entries()) {
        for (const side of [-1, 1]) {
            const hangs = Math.abs(place.x - Routes.sideX(column, side, at)) <= 0.5
                && place.y >= column.top - at.tall - 1e-6
                && place.y <= column.bottom - at.tall + 1e-6;
            if (hangs) {
                const row = column.rows.findIndex(one => place.y <= one.bottom - at.tall + 1e-6);
                return { kind: "side", c, side, x: place.x, y: place.y, row, how: "hangs" };
            }
        }
    }
    return null;
}

// A corner it can grab from `place`: the top of a window's side, a
// little below it and near across, as a stop, or null.
function grabbed(place, cols, at) {
    let found = null;
    let far = Infinity;
    for (const [c, column] of cols.entries()) {
        for (const side of [-1, 1]) {
            const x = Routes.sideX(column, side, at);
            for (const [r, row] of column.rows.entries()) {
                const y = row.top - at.tall;
                const off = Math.hypot(x - place.x, y - place.y);
                const reaches = Math.abs(x - place.x) <= GRABS_ACROSS * at.wide
                    && y >= place.y && y - place.y <= GRABS_DOWN * at.tall;
                const free = within({ x, y }, cols, at) === null && Routes.onScreen(x, at)
                    && !Routes.inMarks(x, y, at);
                if (reaches && free && off < far) {
                    far = off;
                    found = { kind: "side", c, side, x, y, row: r, how: "grabs" };
                }
            }
        }
    }
    return found;
}

// What it lands on falling from `place`: the nearest ledge of `cols`
// below it, a column's top or the gap between two of its windows, or
// the screen's bottom; or one a little above it, which it scrambles up
// onto.
function under(place, cols, at) {
    let found = { kind: "floor", c: -1, x: place.x, y: Routes.floor(at), row: 0, how: "falls" };
    for (const [c, column] of cols.entries()) {
        if (place.x < column.right && place.x + at.wide > column.left) {
            for (const ledge of Routes.ledges(column, at)) {
                if (ledge.y >= place.y - SCRAMBLES * at.cell && ledge.y < found.y) {
                    found = { kind: "ledge", c, x: place.x, y: ledge.y, row: ledge.row,
                        how: "falls" };
                }
            }
        }
    }
    return found;
}

// Whether at `place` it's on the margin's marks, or on its way across
// them, which hold still whatever the windows do.
function onMarks(place, at) {
    const lit = at.marks;
    return !!lit && place.x < lit.to - lit.each / 4 && place.x + at.wide > lit.from + lit.each / 4
        && place.y < (lit.each * 3) / 4;
}

// Carried as niri moves the top or side it held before the windows
// changed (`at.before`, columns() of the tiles as they were, each row
// its window's `id`), a window grown or shrunk against it: on niri's own
// spring, so it goes exactly as the window's edge does, never through a
// window's face (the GPU rig, 2026-11-13). Its top gone, or no longer
// under it, it's left where it is, in the air; on the marks, which
// hold still, it stays. Only before it sets off: the change came first.
function rides(way, cols, at) {
    const held = at.before?.length && at.spring && way.legs.length === 0 && !onMarks(way, at)
        ? holding(way, at.before, at) : null;
    if (held === null || held.c < 0) {
        return;
    }
    const was = at.before[held.c];
    const ids = was.rows.map(one => one.id).filter(id => id !== undefined);
    const column = cols.find(one => one.rows.some(row => ids.includes(row.id)));
    if (!column) {
        return;
    }
    let to = null;
    if (held.kind === "side") {
        to = { x: Routes.sideX(column, held.side, at), y: Math.min(Math.max(way.y,
            column.top - at.tall), column.bottom - at.tall) };
    } else {
        const id = was.rows[held.row].id;
        const row = column.rows.find(one => id !== undefined && one.id === id);
        const middle = way.x + column.left - was.left + at.wide / 2;
        if (row && middle >= column.left && middle <= column.right) {
            to = { x: middle - at.wide / 2, y: Routes.stands(row.top, at) };
        }
    }
    if (to === null || Math.hypot(to.x - way.x, to.y - way.y) < 0.01
        || within(to, cols, at) !== null || !Routes.onScreen(to.x, at)
        || Routes.inMarks(Math.min(to.x, way.x), Math.min(to.y, way.y), Object.assign({}, at,
            { wide: Math.abs(to.x - way.x) + at.wide, tall: Math.abs(to.y - way.y) + at.tall }))) {
        return;
    }
    // Told of the resize `ahead` ms after niri set its edge going, it
    // starts as far in (MarginCat.behind).
    const spring = Object.assign({}, at.spring, { slowdown: at.slowdown ?? 1 });
    const ms = MarginCat.slides(spring);
    const ahead = Math.min(at.ahead ?? 0, ms);
    Ways.leg(way, "rides", to, { ms: ms - ahead,
        slide: t => MarginCat.slid(t + ahead, spring) });
}

// From wherever it is on a way, onto something that holds it on
// `cols`: shoved clear of a window grown into it, then, in the air, a
// grab at an edge in reach or a fall onto what's below, landing on its
// feet and shaking it off. The stop it ends on.
function settle(way, cols, at) {
    rides(way, cols, at);
    const inWindow = within(way, cols, at);
    if (inWindow !== null) {
        const out = clear(way, cols, at, inWindow);
        if (out !== null) {
            way.turn = 0;
            const far = Math.hypot(out.x - way.x, out.y - way.y);
            const ms = Math.max(SHOVES, (far / (SHOVED_AT_MOST * at.tall)) * MarginCat.MS);
            Ways.leg(way, "holds", out, { name: "shoved", pose: "pouncing", ms, ease: null });
        }
    }
    if (onMarks(way, at) && cols.length > 0) {
        const lit = at.marks;
        // On the marks, or on its way across them: it goes on across.
        const mark = Math.floor((way.x + at.wide / 2 - lit.from) / lit.each);
        return { kind: "perched", mark: Math.min(Math.max(mark, 0), lit.count - 1) };
    }
    if (at.height !== undefined && cols.length > 0 && way.y > Routes.floor(at) + 1e-6) {
        // Under the screen, as on its way up from the workspace below:
        // it comes up the way MarginCatPaths.js's walk finds.
        return { kind: "below" };
    }
    const held = holding(way, cols, at);
    if (held !== null) {
        if (Math.abs(held.y - way.y) > 1e-6) {
            // A little off the ledge it's on, as it lands from a hop.
            Ways.goes(way, way.x, held.y);
        }
        return held;
    }
    const grab = grabbed(way, cols, at);
    if (grab !== null) {
        Ways.clings(way, grab.side, -1);
        Ways.leap(way, grab, 0, "grabs");
        return grab;
    }
    const lands = under(way, cols, at);
    way.turn = 0;
    Ways.falls(way, lands);
    Ways.holds(way, "lands", MarginCat.LANDS);
    Ways.holds(way, "shakes", SHAKES);
    return lands;
}
