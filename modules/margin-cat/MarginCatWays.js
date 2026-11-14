// The beats the margin cat's ways are made of (MarginCatPaths.js), and how
// long each lasts: the legs a way adds as it goes (a walk, a hold, a leap
// or a climb), their timing as beats, and where a way has it at each
// moment of one. MarginCatPaths.js says where its ways go.
.pragma library
.import "MarginCat.js" as MarginCat

// How fast it falls, in its cells a second each second: a hop of a cell
// and a half over a gap lasts about a sixth of a second, a drop between
// workspaces about two thirds.
const GRAVITY = 400;
// The fastest it falls, in its heights a second: half its body between two
// frames at 120 Hz, near enough, so a long fall is never a jump.
const FALLS_AT_MOST = 54;
// The shortest a leap may last, so even a small one is seen.
const LEAPS_AT_LEAST = 160;
// The shortest a pose may show, in ms: two frames at 60 Hz, so no drawing
// flashes for one.
const SHOWN = 33;
// It climbs in bounds, as a cat goes up a tree rather than a lift up a
// shaft: one each `BOUNDS` of its heights, surging and gripping by
// `SURGES` either side of the pace it walks, so a climb reads as quick
// as a walk along a top and no quicker (Joel, 2026-11-13 17:22, by
// WCJQSG fc3f716a: "Going up the side of a window should be at the same
// speed roughly as walking on top").
const BOUNDS = 6;
const SURGES = 0.3;
// The longest all the beats nobody can see may take together, in ms: off
// the screen, on a workspace above or below it, it hurries, since only
// where it comes back into sight is seen, and a cat gone for seconds
// reads as lost.
const UNSEEN = 360;
// The beats it walks or climbs, its strides stepped by the ground it
// covers, and the poses of each stride in turn: each pair of legs apart
// two strides and gathered two, the hind pair a stride behind.
const WALKS = ["walkingA", "walkingC", "walkingB", "walkingD"];
const STRIDING = { goes: WALKS, climbs: WALKS };
// The pose of each beat it holds still, or nearly: standing as it turns,
// thinks or settles; crouched as it gathers itself, lands or perches on
// a mark or shoved by a window; batting at the first mark before it hops
// on; and shaking itself off after a fall. At play (MarginCatGames.js):
// sitting as the yarn comes, as it watches it go and as it sends it off;
// its ears up as it notices, its tail going as it eyes it, crouched as it
// wiggles, a paw on the yarn, and its ears again as it looks about.
const HOLDS = { turns: "walkingB", thinks: "walkingB", settles: "walkingB",
    crouches: "pouncing", lands: "pouncing", perches: "pouncing", shoved: "pouncing",
    bats: "batting", shakes: "grooming", waits: "nothingHere", watches: "nothingHere",
    sends: "nothingHere", notices: "earFlick", eyes: "tailSwish", wiggles: "pouncing",
    pins: "batting", looks: "earFlick" };

// A way begun at `from`, the legs it takes added as it goes, each from
// where the last left it.
function begun(at, from) {
    return { at, x: from.x, y: from.y, facing: at.facing ?? 0, turn: 0, legs: [] };
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

// A beat held `ms`, moving it to `to` on the theme's ease-out if it's
// given, as a crouch draws back.
function holds(way, name, ms, to) {
    leg(way, "holds", to ?? way, { name, pose: HOLDS[name], ms, ease: to ? "out" : null });
}

// Walking to `x` on the ground there.
function goes(way, x, y) {
    if (Math.abs(x - way.x) < 1e-6 && Math.abs(y - way.y) < 1e-6) {
        return;
    }
    leg(way, "goes", { x, y });
}

// A leap to `to`, as a thing thrown falls, its height `rises` cells above
// the higher end, and never above the screen where both ends are on it.
function leap(way, to, rises, name) {
    const highest = Math.min(way.y, to.y);
    const top = highest - rises * way.at.cell;
    const apex = highest >= 0 ? Math.max(0, top) : top;
    leg(way, "leaps", to, { name: name ?? "leaps", apex, pose: "leaping" });
}

// Straight down to `to`, gathering speed as a thing dropped does, up to
// the fastest it falls; a beat named `name`, or falls.
function falls(way, to, name) {
    const speed = falling(way.at);
    const ground = Math.abs(to.y - way.y);
    const seconds = ground <= speed.reaches ? Math.sqrt((2 * ground) / speed.gravity)
        : speed.until + (ground - speed.reaches) / speed.most;
    const ms = Math.max(LEAPS_AT_LEAST, seconds * MarginCat.MS);
    leg(way, "falls", to, { name: name ?? "falls", pose: "falling", speed, ms });
}

// A fall's speed: its gravity and the fastest it falls, in pixels a
// second, and how long it takes to reach it and how far it falls till
// then.
function falling(at) {
    const gravity = GRAVITY * at.cell;
    const most = FALLS_AT_MOST * at.tall;
    return { gravity, most, until: most / gravity, reaches: (most * most) / (2 * gravity) };
}

// How far a fall has fallen `s` seconds in.
function fallen(speed, s) {
    return s <= speed.until ? (speed.gravity * s * s) / 2
        : speed.reaches + speed.most * (s - speed.until);
}

// Riding niri's motion to `to` over `ride.ms`, `ride.edge` its share of
// the way at each ms of the leg. Its first frames catch up to the edge without a jump (VKCF3Z,
// 2026-11-13 a36e05f9). Carried, its seat coming up into it, it closes
// within two frames in the pose it has. Left in the air, its seat going
// from under it (`ride.away`), it goes on with the edge and falls the gap
// the edge opened before it heard, at gravity's pace or faster where it
// must sit flush by the time the edge stops, and never more than
// `ride.room` behind, before what follows would pass through it; past two
// cells in the falling drawing, landing crouched, and no pose for less
// than two frames at 60 Hz.
function rides(way, to, ride) {
    const at = way.at;
    const edge = ride.edge;
    const ms = ride.ms;
    const ground = Math.hypot(to.x - way.x, to.y - way.y);
    // Carried, it closes over two frames, never shorter than two at 60 Hz:
    // niri drew a resize's frames 16 ms apart on a 120 Hz rig.
    const closes = 2 * Math.max(ride.frame, MarginCat.MS / 60);
    let slide = t => edge(t) * (ride.frame > 0 ? Math.min(1, t / closes) : 1);
    let posed = () => null;
    if (ride.away && ground > 0) {
        let stops = ms;
        while (stops > 1 && ground * (1 - edge(stops - 1)) < 1) {
            stops -= 1;
        }
        const left = ground * edge(0);
        const gravity = Math.max(GRAVITY * at.cell, (2 * left) / (stops / MarginCat.MS) ** 2);
        const behind = t => Math.min(ride.room ?? Number.POSITIVE_INFINITY,
            Math.max(0, left - (gravity * (t / MarginCat.MS) ** 2) / 2));
        const flies = 2 * at.cell;
        let lifts = null;
        let lands = stops;
        for (let t = 0; t <= stops; t++) {
            if (lifts === null && behind(t) > flies) {
                lifts = t;
            } else if (lifts !== null && behind(t) < 0.5) {
                lands = t;
                break;
            }
        }
        slide = t => (ground * edge(t) - behind(t)) / ground;
        if (lifts !== null && lands - lifts >= SHOWN) {
            const crouches = ms - lands >= SHOWN ? lands + MarginCat.LANDS : lands;
            posed = t => t >= lifts && t < lands ? "falling"
                : t >= lands && t < crouches ? "pouncing" : null;
        }
    }
    leg(way, "rides", to, { ms, slide, posed });
}

// Up or down a column's side to `y`, the top of its box, each gap between
// two of its rows leapt `rises` cells above.
function climb(way, column, y, rises) {
    const at = way.at;
    const dir = Math.sign(y - way.y);
    const rows = dir > 0 ? column.rows : column.rows.slice().reverse();
    for (let i = 1; i < rows.length; i++) {
        const leaves = dir > 0 ? rows[i - 1].bottom - at.tall : rows[i - 1].top;
        const reaches = dir > 0 ? rows[i].top : rows[i].bottom - at.tall;
        if ((leaves - way.y) * dir > 0 && (y - reaches) * dir >= 0 && (reaches - leaves) * dir > 0) {
            leg(way, "climbs", { x: way.x, y: leaves });
            leap(way, { x: way.x, y: reaches }, rises);
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

// How long a leg lasts at `pace`, its cells a second, before the theme's
// slowdown: a leap as long as falling its height takes.
function lasts(one, pace, at) {
    if (one.kind === "holds") {
        return one.ms;
    }
    const ground = Math.abs(one.to.x - one.from.x) + Math.abs(one.to.y - one.from.y);
    if (one.kind === "falls") {
        return one.ms;
    }
    if (one.kind === "leaps") {
        const fall = from => Math.sqrt((2 * Math.max(0, from - one.apex)) / at.cell / GRAVITY);
        return Math.max(LEAPS_AT_LEAST, (fall(one.from.y) + fall(one.to.y)) * MarginCat.MS);
    }
    return (ground / at.cell / pace) * MarginCat.MS;
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
    const walked = way.legs.filter(one => one.kind === "goes" || one.kind === "climbs")
        .reduce((all, one) =>
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
            const ground = Math.abs(one.to.x - one.from.x) + Math.abs(one.to.y - one.from.y);
            beats.push(Object.assign({}, one, { name: one.name ?? one.kind, begins: ends,
                ends: ends + ms, stride: STRIDING[one.kind] ? stride : 0,
                bounds: one.kind === "climbs"
                    ? Math.max(1, Math.round(ground / (BOUNDS * at.tall))) : 0 }));
            ends += ms;
        }
    }
    const from = way.legs[0]?.from ?? { x: way.x, y: way.y };
    const route = { from, to: { x: way.x, y: way.y }, facing: way.facing, stride, beats, ends,
        cell: at.cell };
    // A striding beat too short to be seen goes on in the pose before it.
    for (const [i, beat] of beats.entries()) {
        if (i > 0 && STRIDING[beat.kind] && beat.ends - beat.begins < SHOWN) {
            beat.held = frame(route, beat.begins - 1e-6).pose;
        }
    }
    return route;
}

// A leap's pose `ms` into it, `whole` ms long, `peak` its share of the way
// at the top: the stretched drawing rising, tucked about the top for a
// fifth of it, then reaching down, each pose seen at least SHOWN or folded
// into the tuck (RSCH-20261113-24R2QV, finding 3). A hop whose fifth is
// under SHOWN skips the tuck rather than flash it (VKCF3Z c234c1d4).
function aloft(ms, whole, peak) {
    const tuck = whole / 5 < SHOWN ? 0 : whole / 5;
    let tucks = Math.min(Math.max(0, peak * whole - tuck / 2), whole - tuck);
    let untucks = tucks + tuck;
    if (tucks < SHOWN) {
        tucks = 0;
    }
    if (whole - untucks < SHOWN) {
        untucks = whole;
        tucks = tuck === 0 ? whole : tucks;
    }
    return ms < tucks ? "leaping" : ms < untucks ? "tucked" : "falling";
}

// `v` on one axis of a beat from `a` to `b`, on its art's own pixels, `cell`
// apart (RSCH-20261113-24R2QV, finding 1): whole cells short of where it
// ends, so it gets there as the beat does, and never short of where it
// starts, so its first step takes up the part-cell a ride left it at. One
// grid, so it never steps back and steps in whole cells. Where it ends
// where it starts, as a level hop, any part of a cell off it is a whole one,
// so an arc keeps its height.
function onCells(v, a, b, cell) {
    if (!(cell > 0)) {
        return v;
    }
    if (Math.abs(b - a) < 1e-9) {
        const cells = (v - a) / cell;
        return a + Math.sign(cells) * Math.ceil(Math.abs(cells) - 1e-6) * cell;
    }
    const on = Math.sign(b - a);
    const short = b - on * Math.ceil((on * (b - v)) / cell - 1e-6) * cell;
    // Gone back past where it started, as a leap rising off its ledge.
    if (on * (v - a) < 0) {
        return short;
    }
    return on * Math.max(on * a, on * short);
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
    const spot = placed(route, beat, t);
    // No stride is seen alone (RSCH-20261113-24R2QV, finding 4): one its
    // beat's last SHOWN would start isn't, and a beat too short for one
    // goes on in the pose it came in.
    const strides = STRIDING[beat.kind];
    const last = Math.max(beat.begins, beat.ends - SHOWN);
    const pose = strides
        ? beat.held ?? strides[strode(beat, t < last ? spot : placed(route, beat, last)) % strides.length]
        : spot.leaps ?? beat.posed?.(t - beat.begins) ?? beat.pose ?? null;
    return { x: spot.x, y: spot.y, pose, turn: beat.turn, facing: beat.facing,
        beat: beat.name, done: false };
}

// How many strides of `beat` it's taken at `spot`, by the ground drawn or
// the bounds of a climb.
function strode(beat, spot) {
    const ground = Math.abs(spot.x - beat.from.x) + Math.abs(spot.y - beat.from.y);
    return beat.bounds > 0 ? Math.floor(2 * beat.bounds * spot.p)
        : Math.floor(ground / beat.stride);
}

// Where `beat` of `route` has it `t` ms in, drawn, its share `p` of the beat
// and, in a leap, the pose its arc gives.
function placed(route, beat, t) {
    const p = (t - beat.begins) / (beat.ends - beat.begins);
    const { from, to } = beat;
    let s = p;
    if (beat.kind === "rides") {
        s = beat.slide(t - beat.begins);
    } else if (beat.kind === "holds") {
        s = beat.ease ? MarginCat.easeOut(p) : p;
    } else if (beat.kind === "climbs") {
        // Each bound a surge and a grip, its pace about its mean.
        const turns = 2 * Math.PI * beat.bounds;
        s = p - (SURGES * Math.sin(turns * p)) / turns;
    } else if (beat.kind === "falls") {
        // As far as it falls by now, of all it falls, its slowdown aside.
        const drop = Math.abs(to.y - from.y);
        const seconds = (p * beat.ms) / MarginCat.MS;
        s = drop === 0 ? 1 : Math.min(fallen(beat.speed, seconds) / drop, 1);
    } else if (beat.kind !== "leaps") {
        s = MarginCat.easeInOut(p);
    }
    const x = from.x + (to.x - from.x) * s;
    let y = from.y + (to.y - from.y) * s;
    let leaps = null;
    if (beat.kind === "leaps") {
        // A parabola through both ends and its apex, as a thing thrown.
        const rise = Math.sqrt(Math.max(0, from.y - beat.apex));
        const fall = Math.sqrt(Math.max(0, to.y - beat.apex));
        const peak = rise + fall === 0 ? 0.5 : rise / (rise + fall);
        y = beat.apex + (rise + fall) ** 2 * (p - peak) ** 2;
        leaps = aloft(t - beat.begins, beat.ends - beat.begins, peak);
    }
    // A window carrying or pushing it keeps the window's pixels.
    if (beat.kind === "rides" || beat.pushed) {
        return { x, y, p, leaps };
    }
    return { x: onCells(x, from.x, to.x, route.cell), y: onCells(y, from.y, to.y, route.cell),
        p, leaps };
}
