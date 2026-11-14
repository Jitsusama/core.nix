// The margin cat's ways through the world (MarginCatScreen.qml): each
// screen's workspaces stacked one above another at niri's gap, as niri lays
// them out, and the cat getting about them as a cat would. It walks along
// the tops of their columns, leaps each gap between two, climbs their
// sides, finds its way up through them (MarginCatRoutes.js), and hops
// across the margin's marks one at a time (Joel, 2026-11-13: "it should
// stay on top of the previous workspaces window ... i also want it to
// climb up, which will require it pathfinding through the spaces between
// windows"; "when crossing across gaps between windows, the kitty should
// jump, is also should crawl above icons at the top middle, or do
// something convincing to show it is having to engage with them"; VKCF3Z
// 3d799798 for the hops). A way is a run of beats (MarginCatWays.js), each
// a pose of the theme's own, and frame() says where it is at each moment
// of one.
.pragma library
.import "MarginCat.js" as MarginCat
.import "MarginCatRoutes.js" as Routes
.import "MarginCatWays.js" as Ways
.import "MarginCatFalls.js" as Falls
.import "MarginCatFloats.js" as Floats

// How high it hops over a gap between columns, in its cells, where the
// margin above has room; and how far above the higher end any other leap
// rises.
const HOPS = 1.5;
const RISES = 1;
// At the marks: how long it bats the first before it hops on, and how
// long it perches crouched on each, about PERCHES and never the same
// twice running, by PERCHED_BY either side, so across them it goes no
// quicker than it walks and never reads as a dash (VKCF3Z 3d799798).
const BATS = 400;
const PERCHES = 300;
const PERCHED_BY = 0.25;

const holds = Ways.holds;

// Each column of a workspace, from its tiles ({ left, top, width, height,
// column, row }, niri's tile_pos_in_workspace_view, tile_size and place in
// its scrolling layout, and its window's `id`) in its own view, moved `dy`
// down: across from `left` to `right`, from its `top` to its `bottom`, and
// its `rows` stacked from the top, each with its window's `id` and `row`,
// 1 at the column's top. A tile with no column, floating, is no ground to
// stand on.
function columns(tiles, dy) {
    const by = {};
    for (const tile of tiles ?? []) {
        if (tile.column === undefined || tile.column === null) {
            continue;
        }
        const row = { top: tile.top + dy, bottom: tile.top + tile.height + dy, id: tile.id,
            row: tile.row };
        const was = by[tile.column];
        by[tile.column] = was ? {
            left: Math.min(was.left, tile.left),
            right: Math.max(was.right, tile.left + tile.width),
            top: Math.min(was.top, row.top),
            bottom: Math.max(was.bottom, row.bottom),
            rows: was.rows.concat([row])
        } : { left: tile.left, right: tile.left + tile.width, rows: [row],
            top: row.top, bottom: row.bottom };
    }
    return Object.values(by).map(column => Object.assign(column, {
        rows: column.rows.sort((one, other) => one.top - other.top)
    })).sort((one, other) => one.left - other.left);
}

// Where the margin's `count` marks are across a screen `across` wide, each
// in a square cell `each` wide from the screen's top, touching and centred
// on it, its ink the middle half of its cell: `from` and `to`, or null for
// none.
function marks(count, across, each) {
    const reach = (count * each) / 2;
    return count > 0 ? { from: across / 2 - reach, to: across / 2 + reach, each, count } : null;
}

// `x` kept on the screen, where its width is given; a walk worked out
// alone, with no screen, is held only at the left.
function onScreen(x, at) {
    return Math.min(Math.max(x, 0), (at.across ?? Infinity) - at.wide);
}

// Across the margin's marks (marks()) the way `dir` goes, from half a
// mark's cell before them: it bats the first, hops onto each in turn and
// perches there crouched, its paws on the mark's ink and only its head
// above the screen, and hops down past the last where `onto` has it
// from the `x` just past them: { x, y }, and the `side` of a window it
// hangs on there, if any. Given the mark it's under, `from`, it hops
// straight up onto that one instead, and on from there.
function perches(way, dir, onto, from) {
    const at = way.at;
    const lit = at.marks;
    const perched = lit.each / 4 + 2 * at.cell - at.tall;
    let first = 0;
    if (from === undefined) {
        holds(way, "bats", BATS);
        holds(way, "crouches", MarginCat.CROUCHES);
    } else {
        first = dir > 0 ? from : lit.count - 1 - from;
        // Up there already, as the windows changed on its way across,
        // it goes on with no crouch.
        if (way.y > perched + at.cell) {
            holds(way, "crouches", MarginCat.CROUCHES);
        }
        Ways.leap(way, { x: way.x, y: perched }, RISES, "hops");
    }
    for (let i = first; i < lit.count; i++) {
        const mark = dir > 0 ? i : lit.count - 1 - i;
        const middle = lit.from + (mark + 0.5) * lit.each;
        Ways.leap(way, { x: middle - at.wide / 2, y: perched }, RISES, "hops");
        holds(way, "perches", PERCHES * (1 + PERCHED_BY * Math.sin(2.4 * (i + 1))));
    }
    const off = onto(dir > 0 ? lit.to + lit.each / 2 : lit.from - at.wide - lit.each / 2);
    if (off.side) {
        // Over a window's side under them, and down onto it, clear of
        // the window's corner.
        Ways.leap(way, { x: off.x, y: perched }, RISES, "hops");
        Ways.clings(way, off.side, 1);
        Ways.leap(way, off, 0, "hops");
    } else if (off.y - perched > 2 * at.tall) {
        // Nothing near under it past them: off level, and a fall.
        Ways.leap(way, { x: off.x, y: perched }, RISES, "hops");
        Ways.falls(way, off);
        holds(way, "lands", MarginCat.LANDS);
        holds(way, "shakes", Falls.SHAKES);
    } else {
        Ways.leap(way, off, RISES, "hops");
        holds(way, "lands", MarginCat.LANDS);
    }
}

// Along the tops of `cols` from where it is to `x`: each gap between two
// columns it passes is leapt, a crouch before and after, and the `lit`
// marks (marks()) are hopped across (perches()).
function along(way, x, cols, lit) {
    const at = way.at;
    const dir = Math.sign(x - way.x);
    if (dir === 0) {
        return;
    }
    way.facing = dir;
    const ground = to => {
        const column = Routes.columnAt(cols, to, at.wide);
        return column ? Routes.stands(column.top, at) : way.y;
    };
    const stops = [];
    const ordered = dir > 0 ? cols : cols.slice().reverse();
    const passes = (begins, ends) => (begins - (way.x + at.wide / 2)) * dir > 0
        && (x + at.wide / 2 - ends) * dir > 0;
    for (let i = 1; i < ordered.length; i++) {
        const edge = dir > 0 ? ordered[i - 1].right : ordered[i - 1].left;
        const next = dir > 0 ? ordered[i].left : ordered[i].right;
        if ((next - edge) * dir > 0 && passes(edge, next)) {
            stops.push({ at: dir > 0 ? edge - at.wide : edge, kind: "leaps",
                to: { x: dir > 0 ? next : next - at.wide, y: Routes.stands(ordered[i].top, at) } });
        }
    }
    if (lit) {
        const left = lit.from - at.wide - lit.each / 2;
        const right = lit.to + lit.each / 2;
        const before = dir > 0 ? left : right;
        const past = dir > 0 ? right : left;
        // Passing their ink at all, it hops across them, from half a
        // mark's cell before them or from where it is if nearer.
        const sweeps = Math.min(way.x, x) < lit.to - lit.each / 4
            && Math.max(way.x, x) + at.wide > lit.from + lit.each / 4;
        if (sweeps) {
            const start = (before - way.x) * dir >= 0 ? before : way.x;
            // A gap under the marks it hops across on them.
            const low = Math.min(start, past);
            const high = Math.max(start, past) + at.wide;
            stops.splice(0, stops.length, ...stops.filter(stop =>
                Math.max(stop.at, stop.to.x) + at.wide <= low
                || Math.min(stop.at, stop.to.x) >= high));
            stops.push({ at: start, kind: "perches" });
        }
    }
    stops.sort((one, other) => (one.at - other.at) * dir);
    for (const stop of stops) {
        // Past the marks already, hopping down beyond them.
        if ((stop.at - way.x) * dir < 0) {
            continue;
        }
        Ways.goes(way, stop.at, ground(stop.at));
        if (stop.kind === "leaps") {
            holds(way, "crouches", MarginCat.CROUCHES);
            Ways.leap(way, stop.to, HOPS);
            holds(way, "lands", MarginCat.LANDS);
        } else {
            perches(way, dir, past => ({ x: past, y: ground(past) }));
        }
    }
    Ways.goes(way, x, ground(x));
}

// Its last steps onto its seat `to`: a cell and a half past it, if it was
// walking, then back onto it, and a crouch a moment as it stops.
function arrives(way, to) {
    const at = way.at;
    const last = way.legs[way.legs.length - 1];
    const dir = Math.sign(to.x - (last?.from.x ?? to.x));
    if (last?.kind === "goes" && dir !== 0) {
        // Never past the screen's edge.
        const past = to.x + dir * MarginCat.OVERSHOOTS * at.cell;
        last.to = { x: onScreen(past, at), y: to.y };
        way.x = last.to.x;
        holds(way, "settles", MarginCat.SETTLES, to);
    } else {
        Ways.goes(way, to.x, to.y);
    }
    holds(way, "lands", MarginCat.LANDS);
}

// Along a route's `stops` (MarginCatRoutes.js) over `now`'s columns from
// the first, where it is, past the `lit` marks: up and down sides, along
// ledges and tops, over edges and across gaps with a leap.
function follows(way, stops, now, lit) {
    let was = stops[0];
    for (const [i, stop] of stops.entries()) {
        const onto = { x: stop.x, y: stop.y };
        const dir = stop.y > way.y + 1e-6 ? 1 : -1;
        if (i === 0 || (stop.how === "climbs" && stops[i + 1]?.how === "climbs")) {
            // Where it sets off, or on its way up or down a side to
            // where it climbs on to, in one climb, its gaps leapt.
            continue;
        }
        if (stop.how === "climbs") {
            Ways.clings(way, stop.side, dir);
            Ways.climb(way, now[was.c], stop.y, RISES / 2);
        } else if (stop.how === "goes") {
            // A ledge has no gaps to leap and no marks over it, and no
            // columns given, it walks at the height it's at.
            along(way, stop.x, stop.row === 0 ? now : [], stop.row === 0 ? lit : null);
        } else if (stop.perches && was.kind === "ledge") {
            // Along its top to the marks over its end, across them, and
            // down onto its window's side under them.
            const before = stop.perches > 0 ? lit.from - way.at.wide - lit.each / 2
                : lit.to + lit.each / 2;
            way.facing = stop.perches;
            if ((before - way.x) * stop.perches > 0) {
                Ways.goes(way, before, way.y);
            }
            perches(way, stop.perches, () => stop);
        } else if (stop.perches) {
            // Up from under the marks onto the one it's under, and across.
            const beneath = Math.floor((way.x + way.at.wide / 2 - lit.from) / lit.each);
            way.turn = 0;
            way.facing = stop.perches;
            perches(way, stop.perches, past => ({ x: past, y: stop.y }),
                Math.min(Math.max(beneath, 0), lit.count - 1));
        } else if (stop.how === "over") {
            Ways.clings(way, stop.side, 1);
            Ways.leap(way, onto, RISES / 2, "over");
        } else if (stop.kind === "side") {
            // Along the screen's bottom to under it, from there.
            if (stop.how === "springs") {
                Ways.goes(way, stop.x, way.y);
            }
            holds(way, "crouches", MarginCat.CROUCHES);
            Ways.clings(way, stop.side, dir);
            Ways.leap(way, onto, RISES / 2);
        } else {
            way.facing = stop.how === "leaps" ? Math.sign(stop.x - way.x) || way.facing
                : -was.side;
            if (stop.how === "leaps") {
                holds(way, "crouches", MarginCat.CROUCHES);
            }
            way.turn = 0;
            Ways.leap(way, onto, stop.how === "leaps" ? HOPS : RISES / (stop.how === "hops" ? 2 : 1),
                stop.how);
            holds(way, "lands", MarginCat.LANDS);
        }
        was = stop;
    }
}

// Its last steps to its seat `to` from the ledge a route left it on: along
// the tops past the `lit` marks, its seat on one, or along its ledge; or,
// `to` the way onto a floating window (Floats.onto), to where it gets on
// (`via`) and on.
function seats(way, to, now, lit) {
    const plan = to.via ? to : null;
    const goes = plan?.via ?? to;
    const tops = (Routes.standing(now, goes, way.at)?.row ?? 0) === 0;
    along(way, goes.x, tops ? now : [], tops ? lit : null);
    if (plan) {
        Floats.lands(way, plan);
    } else {
        arrives(way, to);
    }
}

// From under the screen up onto the side a route up begins on (`first`):
// out of sight to just under the screen's bottom, and from there up onto
// the column's side, so all of its leap that's seen is one.
function comesUp(way, first) {
    const below = way.at.height + way.at.cell;
    Ways.clings(way, first.side, -1);
    if (way.y < below) {
        Ways.leap(way, { x: way.x, y: below }, 0);
    }
    Ways.leap(way, { x: first.x, y: below }, 0);
    Ways.leap(way, { x: first.x, y: first.y }, RISES);
}

// A walk along the margin from `from` to its seat `to`, each { x, y }, over
// `at`'s `cols` (columns()) and past its `marks`: it turns its head the
// way it goes, thinks `pause` ms, draws back a cell crouched, and walks,
// as MarginCat.js's walk says, leaping each gap and hopping across the
// marks on the way. `hurried`, its seat moved under it, it neither thinks
// nor crouches, and turns only to go back the way it came from `facing`.
// Where the windows changed under it, it's first shoved clear, or grabs
// an edge or falls (MarginCatFalls.js), and sets off from there. Null
// where it's already there, to the screen's pixel.
function walk(from, to, at) {
    if (there(from, to, at)) {
        return null;
    }
    return Ways.timed(onward(Ways.begun(at, from), to, at));
}

// Whether `from` is `to`, to the screen's pixel.
function there(from, to, at) {
    const pixel = one => Math.round(one * at.ratio);
    return pixel(from.x) === pixel(to.x) && pixel(from.y) === pixel(to.y);
}

// A walk's legs onto `way`, from wherever it has got to, over `at`'s
// `cols`, as walk() says.
function onward(way, to, at) {
    const cols = at.cols ?? [];
    // On a floating window, it gets off first, or walks its top to a
    // seat on it; a seat on another, it makes for where it gets on.
    const from = { x: way.x, y: way.y };
    const aboard = Floats.on(at.floats, from, at);
    const stays = aboard !== null && aboard === Floats.on(at.floats, to, at);
    const plan = stays ? null : Floats.onto(from, to, cols, at);
    const goal = plan?.via ?? to;
    const level = place => (Routes.standing(cols, place, at)?.row ?? 0) === 0;
    const legs = way.legs.length;
    // Off the tops, to or from a window stacked under another, it finds
    // its way there, setting off the way its route first goes.
    if (!stays) {
        Floats.off(way, cols, goal, at);
    }
    let start = stays ? { kind: "floats" } : Falls.settle(way, cols, at);
    if (start.kind === "perched") {
        way.turn = 0;
        way.facing = Math.sign(to.x - way.x) || way.facing || 1;
        perches(way, way.facing, past => Falls.under({ x: past, y: way.y }, cols, at),
            start.mark);
        start = Falls.settle(way, cols, at);
    }
    const tops = stays || (start.kind === "ledge" && start.row === 0 && level(goal));
    let stops = tops || start.kind === "below" ? null : Routes.route(start, goal, cols, at);
    if (start.kind === "below") {
        stops = Routes.up({ x: way.x, y: at.height }, goal, cols, at);
        if (stops !== null) {
            comesUp(way, stops[0]);
        }
    }
    const fell = way.legs.length > legs;
    const dir = Math.sign((stops?.[1] ?? goal).x - way.x) || at.facing || 1;
    if (!at.hurried || at.facing !== dir) {
        way.facing = dir;
        holds(way, "turns", MarginCat.TURNS);
    }
    if (!at.hurried && !fell) {
        holds(way, "thinks", at.pause ?? 0);
        const back = start.kind === "side" ? 0 : dir * MarginCat.DRAWS_BACK * at.cell;
        holds(way, "crouches", MarginCat.CROUCHES, { x: onScreen(way.x - back, at), y: way.y });
    }
    if (stops !== null) {
        follows(way, stops, cols, at.marks);
    }
    seats(way, plan ?? to, stays ? [] : cols, at.marks);
    return way;
}

// A move between workspaces, `way` 1 to the one below and -1 above. Through
// niri's slide it holds its ground, seated on the glass as the bar is while
// the world slides under it, rather than riding its window off the screen
// and coming back in somewhere else, which read as a teleport, and which
// the shell, hearing of a slide when niri is half done, could never keep in
// step with (VKCF3Z 4138e938, 4d18ade4). Once the slide settles it walks to
// its seat `to` on the new workspace's columns (`now`, tiles as columns()
// takes them) from where it held, as any walk does: on the ledge under it,
// as on a tiled workspace, whose tops land where the old ones were; out of
// a window's face the nearest way, never down through it; or a fall onto
// what's under it from the air. With nowhere to go, `to` null, it only
// holds. Its frames are `ahead` ms in, as late as it hears
// (MarginCat.behind), so the walk sets off as niri's slide stops.
function moves(from, to, at) {
    const spring = Object.assign({}, at.spring, { slowdown: at.slowdown ?? 1 });
    const ahead = (at.ahead ?? 0) + (at.way > 0 ? at.frame ?? 0 : 0);
    const way = Ways.begun(at, from);
    Ways.leg(way, "rides", from, { ms: MarginCat.slides(spring), slide: () => 0,
        posed: () => null });
    const now = columns(at.now, 0);
    if (to !== null && now.length > 0 && !there(from, to, at)) {
        onward(way, to, Object.assign({}, at, { cols: now, hurried: false }));
    }
    return Object.assign(Ways.timed(way), { way: at.way, ahead });
}

// Where a way has it `t` ms in (MarginCatWays.js's frame), and a move as
// far ahead as it was told of late.
function frame(route, t) {
    return Ways.frame(route, t + (route.ahead ?? 0));
}
