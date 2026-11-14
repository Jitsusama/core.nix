// The margin cat on floating windows, which sit in front of the tiled ones
// (Joel, 2026-11-13 18:59, by 8GYNJ6 38f735c6: "we need to have the cat
// support jumping down on to a floating window, and then jumping to the
// top of a non floating one when it closes or loses focus"). It drops onto
// one's top past the tiled windows' faces, which are behind it, and leaps
// up off it onto theirs, never higher than a cat leaps: past that it
// climbs the floating window's side, or drops off its end and walks on
// (VKCF3Z 26038726). MarginCatPaths.js's walk asks it the way on and off.
.pragma library
.import "MarginCat.js" as MarginCat
.import "MarginCatRoutes.js" as Routes
.import "MarginCatWays.js" as Ways
.import "MarginCatFalls.js" as Falls

// The highest it leaps up, in its heights: a cat jumps about five times
// its height, and a longer leap reads as flying.
const LEAPS_UP = 4;
// How much longer it crouches, landing from a drop of more than LEAPS_UP
// of its heights, than from a leap: a beat longer.
const LANDS_HARD = 2;
// How high above the higher end a leap off or onto one rises, in cells.
const RISES = 1;

// Each floating window of a workspace, from its tiles as
// MarginCatPaths.js's columns() takes them, those with no column, moved
// `dy` down: across from `left` to `right`, from its `top` to its
// `bottom`, with its window's `id`.
function of(tiles, dy) {
    return (tiles ?? []).filter(tile => tile.column === undefined || tile.column === null)
        .map(tile => ({ left: tile.left, right: tile.left + tile.width, top: tile.top + dy,
            bottom: tile.top + tile.height + dy, id: tile.id }));
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
        Ways.holds(way, "crouches", MarginCat.CROUCHES);
        Ways.leap(way, up, RISES);
        Ways.holds(way, "lands", MarginCat.LANDS);
        return true;
    }
    if (closed) {
        drops(way, cols, at);
        return true;
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
    Ways.leap(way, { x: end.x, y: way.y }, RISES, "hops");
    drops(way, cols, at);
    return true;
}

// Straight down onto what's under it (MarginCatFalls.js), shaking it off
// where that's the screen's bottom.
function drops(way, cols, at) {
    const ground = Falls.under(way, cols, at);
    Ways.falls(way, { x: way.x, y: ground.y });
    Ways.holds(way, "lands", MarginCat.LANDS);
    if (ground.kind === "floor") {
        Ways.holds(way, "shakes", Falls.SHAKES);
    }
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
// a leap's height; leapt; or climbed and over its edge.
function lands(way, plan) {
    const at = way.at;
    const it = plan.float;
    const y = Routes.stands(it.top, at);
    way.turn = 0;
    if (plan.how === "drops") {
        const hard = y - way.y > LEAPS_UP * at.tall;
        Ways.falls(way, { x: way.x, y });
        Ways.holds(way, "lands", MarginCat.LANDS * (hard ? LANDS_HARD : 1));
    } else {
        const edge = plan.side < 0 ? it.left : it.right - at.wide;
        way.facing = -plan.side;
        if (plan.how === "leaps") {
            Ways.holds(way, "crouches", MarginCat.CROUCHES);
            Ways.leap(way, { x: edge, y }, RISES);
        } else {
            Ways.clings(way, plan.side, -1);
            Ways.climb(way, { rows: [it] }, it.top - at.tall, RISES);
            Ways.clings(way, plan.side, 1);
            Ways.leap(way, { x: edge, y }, RISES / 2, "over");
        }
        Ways.holds(way, "lands", MarginCat.LANDS);
    }
    way.turn = 0;
    if (Math.abs(plan.to.x - way.x) > 1e-6) {
        way.facing = Math.sign(plan.to.x - way.x);
        Ways.goes(way, plan.to.x, y);
        Ways.holds(way, "settles", MarginCat.SETTLES);
    }
}
