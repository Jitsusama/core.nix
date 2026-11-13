// The margin cat's ways through the world (MarginCatScreen.qml): each
// screen's workspaces stacked one above another at niri's gap, as niri lays
// them out, and the cat getting about them as a cat would. It walks along
// the tops of their columns, leaps each gap between two, climbs a column's
// side, and creeps behind the margin's marks rather than through them
// (Joel, 2026-11-13: "it should stay on top of the previous workspaces
// window ... i also want it to climb up, which will require it
// pathfinding through the spaces between windows"; "when crossing across
// gaps between windows, the kitty should jump"; VKCF3Z e2068d64 for the
// creep). A way is a run of beats, each a pose of the theme's own, and
// frame() says where it is at each moment of one.
.pragma library
.import "MarginCat.js" as MarginCat

// How fast it falls, in its cells a second each second: a hop of a cell
// and a half over a gap lasts about a sixth of a second, a drop between
// workspaces about two thirds.
const GRAVITY = 400;
// The shortest a leap may last, so even a small one is seen.
const LEAPS_AT_LEAST = 160;
// How high it hops over a gap between columns, in its cells, where the
// margin above has room; and how far above the higher end any other leap
// rises.
const HOPS = 1.5;
const RISES = 1;
// How much slower than its walk it creeps behind the marks, crouched low,
// and how long it stops crouched before them and stands after.
const CREEPS = 0.6;
// How much faster than its walk it climbs, in bounds, and the longest a
// climb up a whole screen's height may take, in ms, so a tall screen
// takes three seconds rather than six, and never more than half its
// body between two frames at 120 Hz.
const CLIMBS = 2;
const CLIMBS_AT_MOST = 3000;
const STOPS = 300;
const STANDS = 180;
// The longest all the beats nobody can see may take together, in ms: off
// the screen, on a workspace above or below it, it hurries, since only
// where it comes back into sight is seen, and a cat gone for seconds
// reads as lost.
const UNSEEN = 360;
// The beats it walks, climbs or creeps, its strides stepped by the ground
// it covers, and the poses of each stride.
const STRIDING = { goes: ["walkingA", "walkingB"], climbs: ["walkingA", "walkingB"],
    creeps: ["creepingA", "creepingB"] };

// Each column of a workspace, from its tiles ({ left, top, width, height,
// column }, niri's tile_pos_in_workspace_view, tile_size and column) in
// its own view, moved `dy` down: across from `left` to `right`, from its
// `top` to its `bottom`, and its `rows` stacked from the top. A tile with
// no column, floating, is no ground to stand on.
function columns(tiles, dy) {
    const by = {};
    for (const tile of tiles ?? []) {
        if (tile.column === undefined || tile.column === null) {
            continue;
        }
        const row = { top: tile.top + dy, bottom: tile.top + tile.height + dy };
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
// in a cell `each` wide, touching and centred on it: `from` and `to`, or
// null for none.
function marks(count, across, each) {
    const reach = (count * each) / 2;
    return count > 0 ? { from: across / 2 - reach, to: across / 2 + reach, each, count } : null;
}

// Which of a drawing's `cells` columns at `x`, each `cell` wide, lie behind
// one of the `lit` marks (marks()), each mark drawn in the middle half of
// its cell: a string with "." for each hidden column and "#" for each
// shown, or "" when none is hidden. The margin draws over the cat, so a cat
// creeping behind them shows only in the gaps between them, never through
// a mark; the screen works this out only as the cat moves.
function veil(x, cells, cell, lit) {
    if (!lit || x + cells * cell <= lit.from || x >= lit.to) {
        return "";
    }
    let mask = "";
    for (let c = 0; c < cells; c++) {
        const middle = x + (c + 0.5) * cell - lit.from;
        const within = middle - Math.floor(middle / lit.each) * lit.each;
        mask += middle >= 0 && middle < lit.to - lit.from
            && within >= lit.each / 4 && within < (lit.each * 3) / 4 ? "." : "#";
    }
    return mask.includes(".") ? mask : "";
}

// A drawing with each column its veil (veil()) hides left out.
function behind(drawing, mask) {
    if (mask === "") {
        return drawing;
    }
    return drawing.map(row => row.split("").map((one, c) => mask[c] === "." ? "." : one).join(""));
}

// The column under the cat's middle at `x`, or the nearest one to it.
function under(cols, x, wide) {
    const middle = x + wide / 2;
    let nearest = null;
    let far = Infinity;
    for (const column of cols) {
        const off = Math.max(column.left - middle, middle - column.right, 0);
        if (off < far) {
            far = off;
            nearest = column;
        }
    }
    return nearest;
}

// Where it stands on a column's top, `air` above it as it sits.
function standsOn(column, at) {
    return column.top - at.tall - at.air;
}

function best(choices, cost) {
    return choices.reduce((most, choice) =>
        most === null || cost(choice) < cost(most) ? choice : most, null);
}

// A way begun at `from`, the legs it takes added as it goes, each from
// where the last left it.
function begun(at, from) {
    return { at, x: from.x, y: from.y, facing: at.facing ?? 0, turn: 0, legs: [], low: false };
}

function leg(way, kind, to, more) {
    way.legs.push(Object.assign({
        kind,
        from: { x: way.x, y: way.y },
        to: { x: to.x, y: to.y },
        facing: way.facing,
        turn: way.turn
    }, more));
    way.x = to.x;
    way.y = to.y;
}

// The pose of each beat it holds still, or nearly: standing as it turns,
// thinks, settles or stands up, and crouched as it gathers itself, lands
// or stops before the marks.
const HOLDS = { turns: "walkingB", thinks: "walkingB", settles: "walkingB",
    stands: "walkingB", crouches: "pouncing", lands: "pouncing", stops: "pouncing" };

// A beat held `ms`, moving it to `to` on the theme's ease-out if it's
// given, as a crouch draws back.
function holds(way, name, ms, to) {
    leg(way, "holds", to ?? way, { name, pose: HOLDS[name], ms, ease: to ? "out" : null });
}

// Walking, or creeping while it's low, to `x` on the ground there.
function goes(way, x, y) {
    if (Math.abs(x - way.x) < 1e-6 && Math.abs(y - way.y) < 1e-6) {
        return;
    }
    leg(way, way.low ? "creeps" : "goes", { x, y });
}

// A leap to `to`, as a thing thrown falls, its height `rises` cells above
// the higher end, and never above the screen where both ends are on it.
function leap(way, to, rises, name) {
    const highest = Math.min(way.y, to.y);
    const top = highest - rises * way.at.cell;
    const apex = highest >= 0 ? Math.max(0, top) : top;
    leg(way, "leaps", to, { name: name ?? "leaps", apex, pose: way.low ? "pouncing" : "leaping" });
}

// Along the tops of `cols` from where it is to `x`: each gap between two
// columns it passes is leapt, a crouch before and after, and the `lit`
// marks (marks()) are crept behind, stopping crouched before them and
// standing up past them, half a mark's cell off either side, where it
// sits beside them.
function along(way, x, cols, lit) {
    const at = way.at;
    const dir = Math.sign(x - way.x);
    if (dir === 0) {
        return;
    }
    way.facing = dir;
    const ground = to => {
        const column = under(cols, to, at.wide);
        return column ? standsOn(column, at) : way.y;
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
                to: { x: dir > 0 ? next : next - at.wide, y: standsOn(ordered[i], at) } });
        }
    }
    if (lit) {
        const left = lit.from - at.wide - lit.each / 2;
        const right = lit.to + lit.each / 2;
        const before = dir > 0 ? left : right;
        const past = dir > 0 ? right : left;
        if ((before - way.x) * dir >= 0 && (x - past) * dir >= 0) {
            stops.push({ at: before, kind: "stops" }, { at: past, kind: "stands" });
        }
    }
    stops.sort((one, other) => (one.at - other.at) * dir);
    for (const stop of stops) {
        goes(way, stop.at, ground(stop.at));
        if (stop.kind === "leaps") {
            holds(way, "crouches", MarginCat.CROUCHES);
            leap(way, stop.to, way.low ? RISES / 2 : HOPS);
            holds(way, "lands", MarginCat.LANDS);
        } else {
            holds(way, stop.kind, stop.kind === "stops" ? STOPS : STANDS);
            way.low = stop.kind === "stops";
        }
    }
    goes(way, x, ground(x));
}

// Its last steps onto its seat `to`: a cell and a half past it, if it was
// walking, then back onto it, and a crouch a moment as it stops.
function arrives(way, to) {
    const at = way.at;
    const last = way.legs[way.legs.length - 1];
    const dir = Math.sign(to.x - (last?.from.x ?? to.x));
    if (last?.kind === "goes" && dir !== 0) {
        last.to = { x: to.x + dir * MarginCat.OVERSHOOTS * at.cell, y: to.y };
        way.x = last.to.x;
        holds(way, "settles", MarginCat.SETTLES, to);
    } else {
        goes(way, to.x, to.y);
    }
    holds(way, "lands", MarginCat.LANDS);
}

// Up or down a column's side to `y`, the top of its box, each gap between
// two of its rows leapt.
function climb(way, column, y) {
    const at = way.at;
    const dir = Math.sign(y - way.y);
    const rows = dir > 0 ? column.rows : column.rows.slice().reverse();
    for (let i = 1; i < rows.length; i++) {
        const leaves = dir > 0 ? rows[i - 1].bottom - at.tall : rows[i - 1].top;
        const reaches = dir > 0 ? rows[i].top : rows[i].bottom - at.tall;
        if ((leaves - way.y) * dir > 0 && (y - reaches) * dir > 0 && (reaches - leaves) * dir > 0) {
            leg(way, "climbs", { x: way.x, y: leaves });
            leap(way, { x: way.x, y: reaches }, RISES / 2);
        }
    }
    leg(way, "climbs", { x: way.x, y });
}

// How it hangs on a side: head the way it climbs (`dir`, 1 down), belly to
// the window, on its left side (-1) or right (1). The theme's profile
// faces left, belly down: mirrored for 1, then a quarter turn, clockwise
// for 1 and back for -1.
function clings(way, side, dir) {
    way.facing = side * dir;
    way.turn = side;
}

// Down a workspace: off its old column's top at the side nearer its seat,
// down that side, and a drop onto the column below it on the new one.
function down(way, home, now, to) {
    const at = way.at;
    const goal = to.x + at.wide / 2;
    const side = best([-1, 1], s => {
        const edge = s < 0 ? home.left : home.right;
        return Math.abs(way.x + at.wide / 2 - edge) + Math.abs(edge - goal);
    });
    along(way, side < 0 ? home.left : home.right - at.wide, [home], null);
    const beside = side < 0 ? home.left - at.wide : home.right;
    clings(way, side, 1);
    leap(way, { x: beside, y: home.top }, RISES / 2, "over");
    climb(way, home, home.bottom - at.tall);
    const below = under(now, beside, at.wide);
    const x = Math.min(Math.max(beside, below.left), below.right - at.wide);
    way.turn = 0;
    way.facing = Math.sign(goal - (x + at.wide / 2)) || way.facing;
    // It lets go, falling out of sight to just above the screen's top,
    // and from there falls in, so all of its fall that's seen is a fall.
    leap(way, { x: beside, y: -at.tall - at.cell }, 0, "drops");
    leap(way, { x, y: standsOn(below, at) }, 0, "drops");
    holds(way, "lands", MarginCat.LANDS);
}

// Up a workspace: off its old column's top, up across the gap between
// workspaces onto the side of a column on the new one, the side the
// shortest way round to its seat that's on the screen, up it, and over
// its top.
function up(way, home, now, to) {
    const at = way.at;
    // Qt's engine has no flatMap.
    const sides = now.reduce((all, each) => all.concat([
        { column: each, side: -1, x: each.left - at.wide },
        { column: each, side: 1, x: each.right }
    ]), []);
    const inSight = sides.filter(one => one.x >= 0 && one.x + at.wide <= at.across);
    const pick = best(inSight.length > 0 ? inSight : sides,
        one => Math.abs(way.x - one.x) + Math.abs(one.x - to.x));
    const { column, side } = pick;
    along(way, Math.min(Math.max(pick.x, home.left), home.right - at.wide), [home], null);
    holds(way, "crouches", MarginCat.CROUCHES);
    clings(way, side, -1);
    // Up out of sight to just under the screen's bottom, and from there
    // up onto the column's side, so all of its leap that's seen is one.
    leap(way, { x: pick.x, y: at.height + at.cell }, 0);
    leap(way, { x: pick.x, y: column.bottom - at.tall }, RISES);
    climb(way, column, column.top - at.tall);
    way.turn = 0;
    way.facing = -side;
    leap(way, { x: side < 0 ? column.left : column.right - at.wide, y: standsOn(column, at) },
        RISES, "mantles");
    holds(way, "lands", MarginCat.LANDS);
}

// How long a leg lasts at `pace`, its cells a second, before the theme's
// slowdown: a leap as long as falling its height takes.
function lasts(one, pace, at) {
    if (one.kind === "holds") {
        return one.ms;
    }
    const ground = Math.abs(one.to.x - one.from.x) + Math.abs(one.to.y - one.from.y);
    if (one.kind === "leaps") {
        const fall = from => Math.sqrt((2 * Math.max(0, from - one.apex)) / at.cell / GRAVITY);
        return Math.max(LEAPS_AT_LEAST, (fall(one.from.y) + fall(one.to.y)) * MarginCat.MS);
    }
    const paces = { creeps: CREEPS, climbs: CLIMBS };
    const ms = (ground / at.cell / (pace * (paces[one.kind] ?? 1))) * MarginCat.MS;
    return one.kind === "climbs" && at.height > 0
        ? Math.min(ms, (CLIMBS_AT_MOST * ground) / at.height) : ms;
}

// Whether any of a leg can be seen on the screen, `across` by `height`.
function seen(one, at) {
    if (at.height === undefined) {
        return true;
    }
    const xs = [one.from.x, one.to.x];
    const ys = [one.from.y, one.to.y, one.apex ?? one.from.y];
    return Math.max(...xs) + at.wide > 0 && Math.min(...xs) < at.across
        && Math.max(...ys) + at.tall > 0 && Math.min(...ys) < at.height;
}

// A way's legs as beats in time: its pace by all the ground it walks, as
// a walk's is, about CRUISES ms where it can and never past HURRIES; its
// stride by that pace; the theme's slowdown on all of it but the ride,
// which niri's own clock already slows.
function timed(way) {
    const at = way.at;
    const slow = at.slowdown ?? 1;
    const walked = way.legs.filter(one => STRIDING[one.kind]).reduce((all, one) =>
        all + Math.abs(one.to.x - one.from.x) + Math.abs(one.to.y - one.from.y), 0) / at.cell;
    const pace = Math.min(Math.max(walked / (MarginCat.CRUISES / MarginCat.MS), MarginCat.AMBLES),
        MarginCat.HURRIES);
    const stride = Math.min(Math.max(pace * MarginCat.STRIDE_SECONDS, MarginCat.SHORTEST_STRIDE),
        MarginCat.LONGEST_STRIDE) * at.cell;
    const natural = way.legs.map(one => one.kind === "rides" ? 0 : lasts(one, pace, at));
    const unseen = way.legs.reduce((all, one, i) =>
        one.kind === "rides" || seen(one, at) ? all : all + natural[i], 0);
    const hurries = Math.min(1, UNSEEN / Math.max(unseen, 1));
    const beats = [];
    let ends = 0;
    for (const [i, one] of way.legs.entries()) {
        const ms = one.kind === "rides" ? one.ms
            : natural[i] * (seen(one, at) ? 1 : hurries) * slow;
        if (ms > 0) {
            beats.push(Object.assign({}, one, { name: one.name ?? one.kind, begins: ends,
                ends: ends + ms, stride: STRIDING[one.kind] ? stride : 0 }));
            ends += ms;
        }
    }
    const from = way.legs[0]?.from ?? { x: way.x, y: way.y };
    return { from, to: { x: way.x, y: way.y }, facing: way.facing, stride, beats, ends };
}

// A walk along the margin from `from` to its seat `to`, each { x, y }, over
// `at`'s `cols` (columns()) and past its `marks`: it turns its head the
// way it goes, thinks `pause` ms, draws back a cell crouched, and walks,
// as MarginCat.js's walk says, leaping each gap and creeping behind the
// marks on the way. `hurried`, its seat moved under it, it neither thinks
// nor crouches, and turns only to go back the way it came from `facing`.
// Null where it's already there, to the screen's pixel.
function walk(from, to, at) {
    const dir = Math.sign(to.x - from.x);
    if (Math.round(from.x * at.ratio) === Math.round(to.x * at.ratio)) {
        return null;
    }
    const way = begun(at, from);
    if (!at.hurried || at.facing !== dir) {
        way.facing = dir;
        holds(way, "turns", MarginCat.TURNS);
    }
    if (!at.hurried) {
        holds(way, "thinks", at.pause ?? 0);
        holds(way, "crouches", MarginCat.CROUCHES,
            { x: from.x - dir * MarginCat.DRAWS_BACK * at.cell, y: from.y });
    }
    along(way, to.x, at.cols ?? [], at.marks);
    arrives(way, to);
    return timed(way);
}

// A move between workspaces, `way` 1 to the one below and -1 above, a
// workspace's `step` apart (its height and niri's gap). Through niri's
// slide it stays on its old column, moving with it exactly as the world
// moves (MarginCat.slid), off the screen's top or bottom. Then it finds
// its way to its seat `to` on the new workspace's columns (`now`, tiles as
// columns() takes them) from the old one's (`was`): down its old column's
// side and a drop, or a leap up onto a new column's side, a climb and over
// its top; then along the tops to its seat, as a walk goes. With nowhere
// to go, `to` null, it only rides.
function moves(from, to, at) {
    const slow = at.slowdown ?? 1;
    const spring = Object.assign({}, at.spring, { slowdown: slow });
    const dy = -at.way * at.step;
    const way = begun(at, from);
    leg(way, "rides", { x: from.x, y: from.y + dy }, {
        ms: MarginCat.slides(spring),
        slide: t => MarginCat.slid(t, spring)
    });
    const was = columns(at.was, dy);
    const now = columns(at.now, 0);
    if (to === null || now.length === 0) {
        return Object.assign(timed(way), { way: at.way });
    }
    const home = under(was, from.x, at.wide) ?? {
        left: from.x, right: from.x + at.wide, top: from.y + dy + at.tall + at.air,
        bottom: from.y + dy + at.step, rows: []
    };
    holds(way, "thinks", at.pause ?? 0);
    (at.way > 0 ? down : up)(way, home, now, to);
    along(way, to.x, now, at.marks);
    arrives(way, to);
    return Object.assign(timed(way), { way: at.way });
}

// Where a way has it `t` ms in: its `x` and `y`, the top left of its box;
// its `pose` by the theme's name, or null for its own as it rides; its
// `turn`, a quarter turn clockwise (1) or back (-1) as it climbs; the way
// it's `facing`; the `beat` it's on; and at its end, `done`, on its seat.
function frame(route, t) {
    const beat = route.beats.find(one => t < one.ends);
    if (!beat) {
        return { x: route.to.x, y: route.to.y, pose: null, turn: 0, facing: route.facing,
            beat: "sits", done: true };
    }
    const p = (t - beat.begins) / (beat.ends - beat.begins);
    const { from, to } = beat;
    let s = p;
    if (beat.kind === "rides") {
        s = beat.slide(t - beat.begins);
    } else if (beat.kind === "holds") {
        s = beat.ease ? MarginCat.easeOut(p) : p;
    } else if (beat.kind !== "leaps") {
        s = MarginCat.easeInOut(p);
    }
    const x = from.x + (to.x - from.x) * s;
    let y = from.y + (to.y - from.y) * s;
    if (beat.kind === "leaps") {
        // A parabola through both ends and its apex, as a thing thrown.
        const rise = Math.sqrt(Math.max(0, from.y - beat.apex));
        const fall = Math.sqrt(Math.max(0, to.y - beat.apex));
        const peak = rise + fall === 0 ? 0.5 : rise / (rise + fall);
        y = beat.apex + (rise + fall) ** 2 * (p - peak) ** 2;
    }
    const strides = STRIDING[beat.kind];
    const ground = Math.abs(x - from.x) + Math.abs(y - from.y);
    const pose = strides ? strides[Math.floor(ground / beat.stride) % 2] : beat.pose ?? null;
    return { x, y, pose, turn: beat.turn, facing: beat.facing, beat: beat.name, done: false };
}
