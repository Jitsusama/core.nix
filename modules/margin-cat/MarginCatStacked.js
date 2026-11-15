// The margin cat going down a workspace as if they were all on one screen
// (MarginCatScreen.qml's moves): niri stacks a screen's workspaces one
// above another, a tenth of its height apart, and it walks the one it
// leaves to the foot of a side, drops across the gap between them, catches
// the corner of a top below with its claws, climbs on and walks to its
// seat (Joel, Sat 14 17:30: "so the top of the left would have to walk to
// the right, climb down the right side, drop down and grab with its claws
// on the top of the window below, climb up, and then walk over"; VKCF3Z
// fcd95edd). Through niri's slide it keeps walking as its ground goes up
// off the screen, and what's then out of sight is cut short, so it comes
// in at the top where that way crosses the edge, mid-drop, about half a
// second later. Its way is laid over the workspace it goes to, with the
// one it left `lift` above, and MarginCatPaths.js's frame() adds what of
// the slide is still to come.
.pragma library
.import "MarginCat.js" as MarginCat
.import "MarginCatPaths.js" as Paths
.import "MarginCatRoutes.js" as Routes
.import "MarginCatWays.js" as Ways
.import "MarginCatFalls.js" as Falls
.import "MarginCatFloats.js" as Floats

// How far above where it's first seen it drops from, in its heights, once
// out of sight: so it comes in already falling, about a quarter of a
// second into its fall, never lost long.
const DROPS_FROM = 2;
// How long it hangs from a corner it's caught before it climbs on, in ms:
// two frames at 60 Hz.
const GRABS = 34;
// How long at least it's out of sight before it drops in, in ms.
const AWAY = 120;
// How far across a corner it reaches as it drops past one, in its widths.
const REACHES = 1;

// How far apart niri stacks a screen's workspaces, in logical pixels: a
// tenth of its height between each, to its device pixel and never under
// one (niri's monitor.rs, workspace_gap).
function apart(at) {
    const ratio = at.ratio ?? 1;
    return at.height + Math.max(1, Math.round(at.height * 0.1 * ratio)) / ratio;
}

// Where it drops in from, coming from `place` above the screen: no
// higher than a little over it, so what's out of sight is cut short; and
// `place` itself on the screen.
function dropsFrom(place, at) {
    return { x: place.x, y: Math.max(place.y, -(DROPS_FROM + 1) * at.tall) };
}

// A move between workspaces, `at.way` 1 to the one below, from `from` on
// `at.was`'s tiles to its seat `to` on `at.now`'s, as MarginCatPaths.js's
// moves() takes it. Going down it's walked as one screen; with nowhere to
// go yet it rides its window off the top on niri's spring. Up, or with
// no way down it can find, it's MarginCatPaths.js's move.
function moves(from, to, at) {
    if (at.way <= 0 || at.was === undefined) {
        return Paths.moves(from, to, at);
    }
    const spring = Object.assign({}, at.spring, { slowdown: at.slowdown ?? 1 });
    const lift = apart(at);
    const told = { way: at.way, ahead: (at.ahead ?? 0) + (at.frame ?? 0), from,
        stacked: { lift, spring } };
    if (to === null) {
        const way = Ways.begun(at, { x: from.x, y: from.y - lift });
        Ways.leg(way, "rides", way, { ms: MarginCat.slides(spring), slide: () => 0,
            posed: () => null });
        return Object.assign(Ways.timed(way), told);
    }
    const route = descends(from, to, Object.assign({}, at, { spring, lift }));
    return route === null ? Paths.moves(from, to, at) : Object.assign(route, told);
}

// The quickest way down from `from` to `to`, each foot it can drop off
// and each catch below it tried: null where there's none, or where its
// seat's on a floating window, which MarginCatPaths.js's walk gets onto.
// The workspace it leaves is `at.lift` above the one it goes to, so a
// seat at the same spot on the screen as `from` is still a way down.
function descends(from, to, at) {
    const now = Paths.columns(at.now, 0);
    const was = Paths.columns(at.was, 0);
    if (now.length === 0 || Floats.on(at.floats, to, at) !== null) {
        return null;
    }
    // On the workspace it leaves, it hurries, and the marks it would hop
    // stay on the glass as its ground slides away, so it keeps off them.
    const old = Object.assign({}, at, { cols: was, hurried: true, marks: null, floats: [] });
    const start = Falls.settle(Ways.begun(old, from), was, old);
    const feet = start.kind === "floor" ? [{ stops: [start], ms: 0 }]
        : start.kind === "ledge" || start.kind === "side" ? Routes.feet(start, was, old) : [];
    let best = null;
    for (const out of feet) {
        const foot = out.stops[out.stops.length - 1];
        for (const lands of catches(foot.x, now, at)) {
            const after = below(foot, lands, to, at);
            const ms = after === null ? Infinity : out.ms + after.ends;
            if (ms < (best?.ms ?? Infinity)) {
                best = { ms, out, after };
            }
        }
    }
    return best === null ? null : joined(from, best, old);
}

// What it can catch dropping in at `x` from above the screen onto `now`:
// the corner of a top within reach across, as a stop hanging on its side;
// with none, a top under it to land on; with nothing under it, the
// screen's bottom.
function catches(x, now, at) {
    const corners = [];
    const under = [];
    for (const [c, column] of now.entries()) {
        for (const side of [-1, 1]) {
            const corner = { kind: "side", c, side, x: Routes.sideX(column, side, at),
                y: column.rows[0].top - at.tall, row: 0, how: "grabs" };
            if (Math.abs(corner.x - x) <= REACHES * at.wide && Routes.onScreen(corner.x, at)
                && !Routes.inMarks(corner.x, corner.y, at)
                && Falls.within(corner, now, at) === null) {
                corners.push(corner);
            }
        }
        if (x < column.right && x + at.wide > column.left) {
            under.push({ kind: "ledge", c, x, y: Routes.stands(column.rows[0].top, at), row: 0,
                how: "drops" });
        }
    }
    if (corners.length > 0) {
        return corners;
    }
    return under.length > 0 ? under
        : [{ kind: "floor", c: -1, x, y: Routes.floor(at), row: 0, how: "drops" }];
}

// From out of sight above `foot`'s side, the drop onto what it `lands` on,
// and on to its seat `to`, timed: null where there's no way on from there.
function below(foot, lands, to, at) {
    const now = Paths.columns(at.now, 0);
    const way = Ways.begun(at, dropsFrom({ x: foot.x, y: foot.y - at.lift }, at));
    way.facing = Math.sign(lands.x - foot.x) || Math.sign(to.x - foot.x) || at.facing || 1;
    Ways.leap(way, lands, 0, "drops");
    if (lands.kind === "side") {
        Ways.clings(way, lands.side, -1);
        Ways.holds(way, "grabs", GRABS);
    } else {
        Ways.holds(way, "lands", MarginCat.LANDS);
    }
    const stops = Routes.route(lands, to, now, at);
    if (stops === null) {
        return null;
    }
    Paths.follows(way, stops, now, at.marks);
    Paths.seats(way, to, now, at.marks);
    return Ways.timed(way);
}

// All of a move down as one way: on the workspace it leaves, `best.out`'s
// walk to its foot, kept until the slide has carried it off the top;
// out of sight a moment; then `best.after`'s drop in and on to its seat.
function joined(from, best, old) {
    const lift = old.lift;
    const way = Ways.begun(old, from);
    const was = old.cols;
    Falls.settle(way, was, old);
    const stops = best.out.stops;
    const dir = Math.sign((stops[1] ?? stops[0]).x - way.x) || old.facing || 1;
    way.facing = dir;
    if (old.facing !== dir) {
        Ways.holds(way, "turns", MarginCat.TURNS);
    }
    Paths.follows(way, stops, was, null);
    const walk = Ways.timed(way);
    const slides = MarginCat.slides(old.spring);
    let off = slides;
    for (let t = 0; t < slides; t++) {
        if (Ways.frame(walk, t).y - lift * MarginCat.slid(t, old.spring) + old.tall <= 0) {
            off = t;
            break;
        }
    }
    const up = one => one && Object.assign({}, one, { y: one.y - lift });
    const beats = walk.beats.filter(beat => beat.begins < off).map(beat => Object.assign({},
        beat, { from: up(beat.from), to: up(beat.to), apex: beat.apex === undefined
            ? undefined : beat.apex - lift }, beat.ends > off
            ? { ends: off, lasts: beat.lasts ?? beat.ends - beat.begins } : {}));
    const left = Ways.frame(walk, off);
    const drops = Math.max(off + AWAY, slides);
    beats.push({ kind: "holds", name: "unseen", pose: "falling", from: up(left), to: up(left),
        facing: left.facing, turn: left.turn, ease: null, ms: drops - off, begins: off,
        ends: drops, stride: 0, bounds: 0 });
    const after = best.after;
    for (const beat of after.beats) {
        beats.push(Object.assign({}, beat, { begins: beat.begins + drops,
            ends: beat.ends + drops }));
    }
    // Till it drops into sight it can find its way again from where it
    // set off (MarginCatPaths.js's replans()).
    return { from, to: after.to, facing: after.facing, stride: after.stride, beats,
        ends: after.ends + drops, cell: old.cell, drops };
}
