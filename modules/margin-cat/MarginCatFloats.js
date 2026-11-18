// The margin cat on floating windows, which sit in front of the tiled ones
// (Joel, 2026-11-13 18:59, by 8GYNJ6 38f735c6: "we need to have the cat
// support jumping down on to a floating window, and then jumping to the
// top of a non floating one when it closes or loses focus"). It drops onto
// one's top past the tiled windows' faces, which are behind it, and leaps
// up off it onto theirs, never higher than a cat leaps: past that it
// climbs the floating window's side, or drops off its end and walks on
// (VKCF3Z 26038726). MarginCatPaths.js's walk asks it the way on and off,
// and the legs of a route over one on its way (follows()).
.pragma library
.import "MarginCat.js" as MarginCat
.import "MarginCatRoutes.js" as Routes
.import "MarginCatWays.js" as Ways
.import "MarginCatFalls.js" as Falls
.import "MarginCatLedges.js" as Ledges

// The highest it leaps up, in its heights, as its routes do.
const LEAPS_UP = Ledges.LEAPS_UP;
// How much longer it crouches, landing from a drop of more than LEAPS_UP
// of its heights, than from a leap: a beat longer.
const LANDS_HARD = 2;
// How high above the higher end a leap off or onto one rises, in cells.
const RISES = 1;
// The most cells up or down it steps within its walk rather than leap
// (steps()).
const STEPS = 2;

// Each floating window of a workspace, from its tiles as
// MarginCatPaths.js's columns() takes them, those with no column, moved
// `dy` down: across from `left` to `right`, from its `top` to its
// `bottom`, with its window's `id`.
function of(tiles, dy) {
    return (tiles ?? []).filter(tile => tile.column === undefined || tile.column === null)
        .map(tile => ({ left: tile.left, right: tile.left + tile.width, top: tile.top + dy,
            bottom: tile.top + tile.height + dy, id: tile.id }));
}

// Every leg of `way` from its `from`th on, but those named in `but`,
// taken aboard a floating window, where it's drawn over the tiled ones
// (MarginCat.above).
function aboard(way, from, but) {
    for (const one of way.legs.slice(from)) {
        if (!(but ?? []).includes(one.name)) {
            one.aboard = true;
        }
    }
}

// The floating window of `floats` whose top it stands on at `place`, all
// of its box over it, or null.
function on(floats, place, at) {
    return (floats ?? []).find(one => place.x >= one.left - 0.5
        && place.x + at.wide <= one.right + 0.5
        && Math.abs(place.y - Routes.stands(one.top, at)) <= at.cell) ?? null;
}

// The ledge of `cols` straight over `x`, all of its box on it, the
// nearest above `y` and no more than `reach` above it: { x, y }, or null.
function over(cols, x, y, reach) {
    let found = null;
    for (const column of cols) {
        if (x < column.left - 0.5 || x + reach.wide > column.right + 0.5) {
            continue;
        }
        for (const ledge of Routes.ledges(column, reach.at)) {
            if (ledge.y < y - 1e-6 && y - ledge.y <= reach.up && (!found || ledge.y > found.y)) {
                found = { x, y: ledge.y };
            }
        }
    }
    return found;
}

// The ledge of `cols` straight under `x`, all of its box on it, the
// nearest below `y`, or else the floor, where the screen has one: { x, y },
// or null. Without the floor, a floating window in front of windows
// taller than it had no side to climb, and the cat climbed the gap behind
// it, through its face (the GPU rig, 2026-11-14).
function below(cols, x, y, at) {
    let near = null;
    for (const column of cols) {
        if (x < column.left - 0.5 || x + at.wide > column.right + 0.5) {
            continue;
        }
        for (const ledge of Routes.ledges(column, at)) {
            if (ledge.y > y + 1e-6 && (!near || ledge.y < near.y)) {
                near = { x, y: ledge.y };
            }
        }
    }
    return near ?? (at.height === undefined ? null : { x, y: Routes.floor(at) });
}

// Where the cat stands at either end of each column's top. Qt's engine
// has no flatMap, so they are gathered with reduce.
function tops(cols, at) {
    return cols.reduce((found, column) =>
        found.concat([column.left, column.right - at.wide]), []);
}

// Off the floating window it stands on, or the one closed from under it
// (`at.floated`, as they were), before it sets off for `to` on another
// window: straight up onto the nearest ledge over its top within a leap,
// from where it stands or the nearest place along the top that has one;
// with none, off its end toward `to` and down onto what's below. Closed,
// it pushes off at once from where it stands, or, with no ledge in reach
// above, falls straight down from there in front of the windows behind,
// never shoved out of them as from one grown into it. True where it went
// off.
function off(way, cols, to, at) {
    const floats = at.floats ?? [];
    const stays = on(floats, way, at);
    const closed = stays ? null : on((at.floated ?? []).filter(one =>
        !floats.some(other => other.id === one.id)), way, at);
    const from = stays ?? closed;
    if (from === null || (stays && on(floats, to, at) === stays)) {
        return false;
    }
    // Over the tiled windows until it's down off it, in front of them
    // (MarginCat.above), and under them as it lands.
    const began = way.legs.length;
    const done = () => {
        aboard(way, began, ["lands", "shakes"]);
        return true;
    };
    const reach = { wide: at.wide, at, up: LEAPS_UP * at.tall };
    const xs = closed ? [way.x] : tops(cols, at).concat([way.x])
        .map(x => Math.min(Math.max(x, from.left), from.right - at.wide));
    let up = null;
    for (const x of xs) {
        const ledge = over(cols, x, way.y, reach);
        if (ledge && (!up || Math.abs(x - way.x) < Math.abs(up.x - way.x))) {
            up = ledge;
        }
    }
    way.turn = 0;
    if (up !== null) {
        way.facing = Math.sign(up.x - way.x) || way.facing;
        Ways.goes(way, up.x, way.y);
        // Closed, nothing holds it to walk up: it pushes off at once.
        if (!closed && steps(way, up, at)) {
            walks(way, up.x, up.y);
            return done();
        }
        Ways.holds(way, "crouches", MarginCat.CROUCHES);
        Ways.leap(way, up, RISES);
        Ways.holds(way, "lands", MarginCat.LANDS);
        return done();
    }
    if (closed) {
        drops(way, cols, at);
        return done();
    }
    // Off its end toward its seat, or the other where that's off the
    // screen, a hop clear of it, and a fall onto what's below.
    const toward = Math.sign(to.x - way.x) || way.facing || 1;
    const ends = [toward, -toward]
        .map(dir => ({ dir, x: dir > 0 ? from.right : from.left - at.wide }))
        .filter(one => Routes.onScreen(one.x, at));
    if (ends.length === 0) {
        return false;
    }
    const end = ends[0];
    way.facing = end.dir;
    Ways.goes(way, end.dir > 0 ? from.right - at.wide : from.left, way.y);
    const ground = Falls.under({ x: end.x, y: way.y }, cols, at);
    const noRoom = cramped(way, { x: end.x, y: way.y }, RISES, at);
    if (aStep(way, ground, at) || noRoom && stepsDown(way, ground, at)) {
        walks(way, end.x, ground.y);
        return done();
    }
    // A hop clear of it, then a fall; with no room over it for the hop's
    // arc, walked clear of its end and dropped (VKCF3Z 13d5bf97).
    if (noRoom) {
        Ways.goes(way, end.x, way.y);
    } else {
        Ways.leap(way, { x: end.x, y: way.y }, RISES, "hops");
    }
    drops(way, cols, at);
    return done();
}

// Straight down onto what's under it (MarginCatFalls.js), shaking it off
// where that's the screen's bottom.
function drops(way, cols, at) {
    const ground = Falls.under(way, cols, at);
    falls(way, { x: way.x, y: ground.y });
    Ways.holds(way, "lands", MarginCat.LANDS);
    if (ground.kind === "floor") {
        Ways.holds(way, "shakes", Falls.SHAKES);
    }
}

// Down to `to`, from where it's drawn: from over the screen's top, the
// screen draws it at its top (MarginCatScreen.qml, which clamps its y at
// 0), where it hung in its falling drawing till its fall reached it. A
// fall wholly over the top shows nothing, so it's at `to` at once.
function falls(way, to) {
    if (way.y < 0 && to.y > way.y) {
        way.y = Math.min(to.y, 0);
    }
    if (to.y <= 0 && way.y === to.y) {
        return;
    }
    Ways.falls(way, to);
}

// Walking to `x`, `y`, and no further than it's drawn: a step to or from
// a ledge over the screen's top walks only what shows under it, and is
// at the rest at once, since walking in place at the top reads as a skid
// on the spot (VKCF3Z 13d5bf97).
function walks(way, x, y) {
    if (way.y < 0) {
        way.y = Math.min(y, 0);
    }
    Ways.goes(way, x, way.y < 0 ? y : Math.max(y, 0));
    way.y = y;
}

// The way onto `to` where it's on a floating window's top it doesn't
// stand on: `via`, where on the tiled windows it makes for first, and
// `how` it gets on from there: drops, straight down onto it from the
// nearest ledge over it; leaps, onto a floating window higher than the
// ledges there, from one beside its side edge within a leap, so the arc
// never crosses its face; or climbs, up its side from a ledge under it.
// Null where `to` isn't on one, or it's on that one already, or there's
// no way on.
function onto(from, to, cols, at) {
    const floats = at.floats ?? [];
    const it = on(floats, to, at);
    if (it === null || on(floats, from, at) === it) {
        return null;
    }
    const y = Routes.stands(it.top, at);
    const all = { wide: at.wide, at, up: Infinity };
    const xs = [to.x].concat(tops(cols, at))
        .map(x => Math.min(Math.max(x, it.left), it.right - at.wide));
    const above = xs.map(x => over(cols, x, y, all)).filter(one => one !== null);
    if (above.length > 0) {
        const near = above.reduce((most, one) =>
            Math.abs(one.x - to.x) < Math.abs(most.x - to.x) ? one : most);
        return { via: near, how: "drops", float: it, to };
    }
    const sides = [-1, 1].map(side => ({ side, x: side < 0 ? it.left - at.wide : it.right }))
        .filter(one => Routes.onScreen(one.x, at))
        .map(one => Object.assign(one, { ledge: below(cols, one.x, y, at) }))
        .filter(one => one.ledge !== null)
        .sort((one, other) => Math.abs(one.x - from.x) - Math.abs(other.x - from.x));
    const leaps = sides.find(one => one.ledge.y - y <= LEAPS_UP * at.tall);
    if (leaps) {
        return { via: leaps.ledge, how: "leaps", side: leaps.side, float: it, to };
    }
    return sides.length > 0
        ? { via: sides[0].ledge, how: "climbs", side: sides[0].side, float: it, to } : null;
}

// From `way.via`, where it's come to, onto the floating window and along
// its top to its seat: dropped, landing crouched, a beat longer from past
// a leap's height; leapt up beside its edge, then over it; or climbed and
// over its edge. A leap that went across as it rose cut over the corner
// of its face for 230 ms, drawn over the tiled windows there (VKCF3Z
// 4125043c: never over the floating window's own face).
function lands(way, plan) {
    const at = way.at;
    const it = plan.float;
    const began = way.legs.length;
    const y = Routes.stands(it.top, at);
    way.turn = 0;
    if (plan.how === "drops" && aStep(way, { x: way.x, y }, at)) {
        walks(way, way.x, y);
    } else if (plan.how === "drops") {
        const hard = y - way.y > LEAPS_UP * at.tall;
        falls(way, { x: way.x, y });
        Ways.holds(way, "lands", MarginCat.LANDS * (hard ? LANDS_HARD : 1));
    } else {
        const edge = plan.side < 0 ? it.left : it.right - at.wide;
        way.facing = -plan.side;
        // Stepped, up beside its edge and across onto its top, never
        // through the corner of its face, and on without landing.
        if (plan.how === "leaps"
            && steps(way, { x: edge, y }, at, { first: { x: way.x, y: it.top - at.tall } })) {
            walks(way, way.x, y);
            Ways.goes(way, edge, y);
        } else if (plan.how === "leaps") {
            Ways.holds(way, "crouches", MarginCat.CROUCHES);
            Ways.leap(way, { x: way.x, y: it.top - at.tall }, RISES);
            Ways.leap(way, { x: edge, y }, RISES / 2, "over");
            Ways.holds(way, "lands", MarginCat.LANDS);
        } else {
            // Climbed no higher than the screen shows, where it would
            // climb on the spot, and over its edge within its walk there.
            const top = it.top - at.tall;
            Ways.clings(way, plan.side, -1);
            Ways.climb(way, { rows: [it] }, Math.max(top, Math.min(way.y, 0)), RISES);
            way.y = Math.min(way.y, top);
            Ways.clings(way, plan.side, 1);
            if (y < 0) {
                walks(way, edge, y);
            } else {
                Ways.leap(way, { x: edge, y }, RISES / 2, "over");
                Ways.holds(way, "lands", MarginCat.LANDS);
            }
        }
    }
    way.turn = 0;
    if (Math.abs(plan.to.x - way.x) > 1e-6) {
        way.facing = Math.sign(plan.to.x - way.x);
        Ways.goes(way, plan.to.x, y);
        Ways.holds(way, "settles", MarginCat.SETTLES);
    }
    // Over the tiled windows the whole way on, in front of them
    // (MarginCat.above).
    aboard(way, began);
}

// Whether `way` to `to` is STEPS cells up or down or less as the screen
// draws it, no higher than its top: too little to leap or fall, so it's
// walked (steps()). Measured on the cat's own heights, a drop from over
// the top onto a ledge a pixel under it fell, and hung in its falling
// drawing for most of a fall's shortest time.
function aStep(way, to, at) {
    return Math.abs(Math.max(to.y, 0) - Math.max(way.y, 0)) <= STEPS * at.cell + 1e-6;
}

// Whether a leap or hop from `way` to `to` would have no arc to show: a
// step (aStep()), or one up whose arc has no room (cramped()). Such a
// leap glided along the edge in a leap's pose for 350 ms (VKCF3Z
// 299fb911, the GPU rig at 120 Hz, 2026-11-16); it steps up within its
// walk instead, paws moving. Only up to its own height: a taller one
// keeps its leap, held at the edge, rather than walk up a wall. Down
// off a floating window's end, see stepsDown().
function steps(way, to, at, { first = to, rises = RISES } = {}) {
    const rise = way.y - to.y;
    return aStep(way, to, at)
        || (rise > 0 && rise <= at.tall + 1e-6 && cramped(way, first, rises, at));
}

// Whether a drop off a floating window's end with no room for a hop is
// small enough to step down from its edge, paws moving: whether its
// step, from its edge to its landing as the screen draws it, crosses
// the corner of the window's face no deeper than half its height.
// Walked clear of its end on the air first, a body-length for 190 ms at
// the GPU rig's 120 Hz, then dropped 3 pixels, it read as a bug (WCJQSG
// 1ee24751: "Stepping down a small ledge with its paws moving is what a
// cat does"); a few pixels of the corner for one stride is the smaller
// break. Deeper, it walks clear and drops, never across the face
// (VKCF3Z 13d5bf97).
//
// Drawn no higher than the screen's top, its feet already sit `sunk`
// into the face as it stands at the top over it (its top is its air
// under its feet, `way.y + tall + air`). Stepping, it goes a body's
// width along as it goes `drop` down, so the depth inside the corner,
// the lesser of how far it's across the edge and how far down the top,
// is deepest where the two meet.
function stepsDown(way, to, at) {
    const from = Math.max(way.y, 0);
    const drop = Math.max(to.y, 0) - from;
    const sunk = from - way.y - at.air;
    const meets = (at.wide - sunk) / (at.wide + drop);
    const deepest = Math.max(at.wide * (1 - meets), 0);
    return drop >= 0 && deepest <= at.tall / 2 + 1e-6;
}

// Whether a leap's apex, `rises` cells (RISES, or a hop's half) over the
// higher end of its first leap (to `first`, a cell under its landing
// onto a floating window's top), is above the screen's top, where the
// screen draws the cat no higher than its top (MarginCatScreen.qml,
// which clamps its y at 0).
function cramped(way, first, rises, at) {
    return Math.min(way.y, first.y) - rises * at.cell < -1e-6;
}

// A route's move to `stop` onto, along or off a floating
// window on its way (MarginCatRoutes.js's ontoFloats() and fromFloat()),
// over `now`'s columns (WCJQSG ade5ce02: "it climbs onto, jumps off and
// walks along floats as it does tiled ones"). It's over the tiled windows
// from the move's first leap or step to its landing, and back under them
// on a tiled window's top, as it is getting onto a seat on one (lands(),
// VKCF3Z 4125043c).
function follows(way, stop, now) {
    const at = way.at;
    const began = way.legs.length;
    const landing = { x: stop.x, y: stop.y };
    way.turn = 0;
    if (stop.how === "goes") {
        way.facing = Math.sign(stop.x - way.x) || way.facing;
        Ways.goes(way, stop.x, stop.y);
    } else if (stop.how === "hopsOff" && (aStep(way, landing, at)
        || cramped(way, stop.via, RISES / 2, at) && stepsDown(way, landing, at))) {
        // Down off its end from its edge within its walk, paws moving,
        // its front paws landing as its back ones leave the corner.
        way.facing = Math.sign(stop.via.x - way.x) || way.facing;
        Ways.goes(way, stop.via.x - way.facing * at.wide, way.y);
        walks(way, landing.x, landing.y);
    } else if (stop.how === "leapsOn" && steps(way, landing, at, { first: stop.via })) {
        // No arc to show: up within its walk, paws moving, to its top's
        // height beside its edge, where a leap's via is a cell under its
        // landing, and on. A hop off is only ever down (Routes'
        // landsOn()), its step taken above.
        way.facing = Math.sign(stop.via.x - way.x) || Math.sign(stop.x - way.x) || way.facing;
        walks(way, stop.via.x, landing.y);
        walks(way, landing.x, landing.y);
    } else if (stop.how === "leapsOn") {
        // Straight up beside its edge till its paws are level with its
        // top, and over, as lands() leaps on.
        way.facing = -stop.side;
        Ways.holds(way, "crouches", MarginCat.CROUCHES);
        Ways.leap(way, stop.via, RISES);
        Ways.leap(way, landing, RISES / 2, "over");
        Ways.holds(way, "lands", MarginCat.LANDS);
    } else if (stop.how === "hopsOff" && cramped(way, stop.via, RISES / 2, at)) {
        // No room over it for a hop: off its end, and dropped.
        way.facing = Math.sign(stop.via.x - way.x) || way.facing;
        Ways.goes(way, stop.via.x, stop.via.y);
        falls(way, landing);
        Ways.holds(way, "lands", MarginCat.LANDS);
    } else if (stop.how === "hopsOff") {
        // Off its end clear of its face, and straight down.
        way.facing = Math.sign(stop.via.x - way.x) || way.facing;
        Ways.holds(way, "crouches", MarginCat.CROUCHES);
        Ways.leap(way, stop.via, RISES / 2, "hops");
        falls(way, landing);
        Ways.holds(way, "lands", MarginCat.LANDS);
    } else {
        // Across a gap from its end onto a column's ledge.
        way.facing = Math.sign(stop.x - way.x) || way.facing;
        Ways.holds(way, "crouches", MarginCat.CROUCHES);
        Ways.across(way, landing, "leaps", Ledges.ceiling(now, way, landing, at.wide));
        Ways.holds(way, "lands", MarginCat.LANDS);
    }
    // Its crouch on a tiled window before it leaps on, and its landing
    // back on one, are under them.
    aboard(way, began, stop.kind === "float" ? ["crouches"] : ["lands"]);
}
