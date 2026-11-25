// The margin cat going down or up a workspace as if they were all on one
// screen (MarginCatScreen.qml's moves): niri stacks a screen's workspaces
// one above another, a tenth of its height apart. Going down it walks the
// one it leaves to the foot of a side, drops across the gap between them,
// catches the corner of a top below with its claws, climbs on and walks
// to its seat (Joel, Sat 14 17:30: "so the top of the left would have to
// walk to the right, climb down the right side, drop down and grab with
// its claws on the top of the window below, climb up, and then walk
// over"; VKCF3Z fcd95edd). Going up it walks over onto the head of a
// side, or along a top to under a side above, leaps up the gap to the
// foot of a side of the window above, never its underside, and climbs it
// to its seat (VKCF3Z fcd95edd: "a cat doesn't drop off the top";
// c9474cef). Through niri's slide it keeps walking as
// its ground goes off the screen, and what's then out of sight is cut
// short, so it comes in where that way crosses the edge, mid-leap, about
// half a second later. Its way is laid over the workspace it goes to,
// with the one it left `lift` above (below, `lift` under nought), and
// MarginCatPaths.js's frame() adds what of the slide is still to come.
.pragma library
.import "MarginCat.js" as MarginCat
.import "MarginCatPaths.js" as Paths
.import "MarginCatRoutes.js" as Routes
.import "MarginCatWays.js" as Ways
.import "MarginCatFalls.js" as Falls
.import "MarginCatFloats.js" as Floats
.import "MarginCatLedges.js" as Ledges

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
// How many of its rows it puts over the screen's edge as it peeks in, its
// ears and its eyes, a row and a drawing of its paws each CREEPS 60 Hz
// frames, so the first is seen as long as the rest; and how long it
// peers, then looks about, in ms.
const PEEKS = 3;
const CREEPS = 3;
const PEERS = 200;
const LOOKS = 200;

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

// Where it leaps up in from, coming from `place` under the screen, as
// dropsFrom() is over it.
function risesFrom(place, at) {
    return { x: place.x, y: Math.min(place.y, at.height + DROPS_FROM * at.tall) };
}

// It comes onto the screen head first at `x`, from under it if it's going
// `up`, else head down from the gap over it, hanging by a side on its
// `side`: its ears and then its eyes come over the edge, its paws moving,
// and it peers and looks about before it comes on (Joel, Sun 15 21:20:
// "when transitioning screens it should be able to poke its head up and
// then come in"; 23:34: "the whole kitty should not have to be rendered
// when you are seeing it come on to your screen"). Only the screen's edge
// cuts it, so nothing new is drawn.
function peeks(x, up, side, at) {
    const way = Ways.begun(at, { x, y: up ? at.height : -at.tall });
    if (!up) {
        Ways.clings(way, side, 1);
    }
    const shows = PEEKS * at.cell;
    Ways.leg(way, "climbs", { x, y: up ? at.height - shows : shows - at.tall },
        { name: "peeks", ms: (PEEKS * CREEPS * MarginCat.MS) / 60, bounds: PEEKS / 2 });
    Ways.holds(way, "peers", PEERS);
    Ways.holds(way, "looks", LOOKS);
    way.turn = 0;
    return way;
}

// Up onto an empty workspace, its ride done under the screen at `place`:
// it peeks in at the bottom edge and springs onto the floor, its only
// seat there, and lands, so it's never seen rising through anything.
function risesOnto(place, at) {
    const way = peeks(place.x, true, 1, at);
    way.facing = at.facing || 1;
    Ways.leap(way, { x: place.x, y: Routes.floor(at) }, 0, "springs");
    Ways.holds(way, "lands", MarginCat.LANDS);
    return Ways.timed(way);
}

// A walk from `from` to `to` over `at`'s columns, as MarginCatPaths.js's
// walk(): from off the screen's side, where the window it sat on carried it
// as niri scrolled the strip (MarginCatScreen.qml's carried), it comes back
// in at that edge first (comesIn()).
function walkAlong(from, to, at) {
    const now = Paths.closing(at);
    const coming = comesIn(from, now);
    if (coming === null) {
        return Paths.walk(from, to, at);
    }
    return Ways.timed(Paths.onward(coming.way, to, Object.assign({}, now,
        { hurried: true, facing: coming.way.facing })));
}

// Its way back onto the screen from `from`, off its side: null where it's
// on the screen, or where no screen is given. niri lays a workspace's
// columns on one strip, so it comes in at the edge the strip carried it
// off, as it would walk the strip were it all on the screen (Joel,
// 2026-11-14 17:24: "focusing to a window off the left or right edge").
// Out of sight, it comes in head first, as it does between workspaces: on
// the top of the window past the edge, its head over the edge, its paws
// moving, it peers and looks about (Joel, Sun 15 21:20: "poke its head up
// and then come in"). Then it comes on facing the way in, leaping the gap
// between that window and the one inside the edge, if there's one and
// that window is all past the edge, and walks on from there without
// thinking again. Partly on the screen, it walks on from where it's drawn.
// It first `holds` that many ms out of sight where given, as niri slides
// to another workspace under it (moves()).
function comesIn(from, at, holds) {
    if (at.across === undefined || Routes.onScreen(from.x, at)) {
        return null;
    }
    const side = from.x < 0 ? -1 : 1;
    const cols = at.cols ?? [];
    const seen = side > 0 ? from.x < at.across : from.x + at.wide > 0;
    const peeked = side > 0 ? at.across - PEEKS * at.cell : PEEKS * at.cell - at.wide;
    // The window past the edge it comes in on, where niri lays one there: the
    // nearest the edge, so a gap at the edge is peeked over from that
    // window's top, never from the air where the cat was drawn.
    const past = seen ? null : cols.filter(one => side > 0 ? one.right > at.across : one.left < 0)
        .sort((one, other) => side > 0 ? one.left - other.left : other.right - one.right)[0]
        ?? null;
    const y = past ? Routes.stands(past.top, at) : from.y;
    const way = Ways.begun(at, { x: seen ? from.x : side > 0 ? at.across : -at.wide, y });
    way.facing = -side;
    way.turn = 0;
    if (seen) {
        return { way, side };
    }
    if (holds) {
        Ways.leg(way, "rides", way, { ms: holds, slide: () => 0, posed: () => null });
    }
    Ways.leg(way, "climbs", { x: peeked, y },
        { name: "peeks", ms: (PEEKS * CREEPS * MarginCat.MS) / 60, bounds: PEEKS / 2 });
    Ways.holds(way, "peers", PEERS);
    Ways.holds(way, "looks", LOOKS);
    // The window inside the edge, nearest it, and the gap to it, leapt
    // only where the window it came in on is all past the edge: one partly
    // on the screen is walked along to its gap, as any walk does.
    const wholly = past !== null && (side > 0 ? past.left >= at.across : past.right <= 0);
    const inside = !wholly ? null : cols.filter(one => side > 0
        ? one.right <= past.left : one.left >= past.right)
        .sort((one, other) => (other.right - one.right) * side)[0];
    if (inside && (side > 0 ? past.left - inside.right : inside.left - past.right) > 0) {
        const onto = { x: side > 0 ? inside.right - at.wide : inside.left,
            y: Routes.stands(inside.top, at) };
        Ways.holds(way, "crouches", MarginCat.CROUCHES);
        Ways.across(way, onto, undefined, Ledges.ceiling(cols, way, onto, at.wide));
        Ways.holds(way, "lands", MarginCat.LANDS);
    }
    return { way, side };
}

// A ride on niri's scroll to `to`, which niri set off at `told.at` on its
// `told.spring`, from where the cat was drawn as niri set it off: at `place`, short
// of it by what was left then of the ride `carry` under way, or by nothing
// (MarginCatScreen.qml's rideTo). The scroll turned back mid-way carries
// on from where the cat was as niri turned it, never from where it's drawn
// now, which would count the time since twice and set it ahead of its
// window.
function rides(carry, place, to, told) {
    const { at, spring } = told;
    const left = carry === null ? { x: 0, y: 0 }
        : MarginCat.lags(carry.from, Math.min(at - carry.at, carry.ends), carry.spring);
    const from = { x: place.x + left.x - to.x, y: place.y + left.y - to.y };
    return { from, at, ends: MarginCat.slides(spring), spring };
}

// How the cat goes back to its seat `there` on the column it sat on, as
// niri's layout has it, from its `place` (MarginCatScreen.qml's follow):
// "stays" where it's there already, a ride under way running on as its
// window's spring does, never set off again from a lag gone by, as the
// keys come back to a column still in view; "rides" with its window from
// where it's drawn while niri's spring slides the strip, a ride under way
// (`at.riding`) or a scroll niri told of within its spring
// (`at.scrolling`), or from wholly off the screen, where only a scroll
// leaves it; and "sits" there at once otherwise, nowhere before or as a
// floating window dragged carries it, which niri doesn't animate.
function backOn(place, there, at) {
    if (place === null) {
        return "sits";
    }
    if (Math.abs(place.x - there.x) < 0.5 && Math.abs(place.y - there.y) < 0.5) {
        return "stays";
    }
    const off = place.x + at.wide <= 0 || place.x >= at.across;
    return at.riding || at.scrolling || off ? "rides" : "sits";
}

// A move between workspaces, `at.way` 1 to the one below and -1 to the
// one above, from `from` on `at.was`'s tiles to its seat `to` on
// `at.now`'s, as MarginCatPaths.js's moves() takes it. It's walked as
// one screen; with nowhere to go yet it rides its window off the top, or
// up off the bottom, on niri's spring. With no way it can find, it's
// MarginCatPaths.js's move.
function moves(from, to, at) {
    const down = at.way > 0;
    if (at.way === 0 || at.was === undefined) {
        return Paths.moves(from, to, at);
    }
    const spring = Object.assign({}, at.spring, { slowdown: at.slowdown ?? 1 });
    const lift = down ? apart(at) : -apart(at);
    // Told of the slide late by about `at.ahead`, it errs where an error
    // falls in the gap between the workspaces: down a frame further on, so
    // the window it stands on never rises over it, and up no further, as
    // MarginCatPaths.js's moves() does, since a frame on at 120 Hz carried
    // it 90 px into its window's face as niri's spring ran fastest.
    const told = { way: at.way, ahead: (at.ahead ?? 0) + (down ? at.frame ?? 0 : 0), from,
        stacked: { lift, spring } };
    // Carried wholly off the screen's side as niri scrolled the strip
    // (MarginCatScreen.qml's carried), it's out of sight as the workspace
    // slides, so it holds there and then comes in at that edge head first
    // over the new workspace's windows, as a walk does (comesIn()), never
    // crossing them from where its old window went, thousands of pixels off.
    if (to !== null && at.across !== undefined
        && (from.x >= at.across || from.x + at.wide <= 0)) {
        const now = Object.assign({}, at, { cols: Paths.columns(at.now, 0), hurried: true });
        const coming = comesIn(from, now, MarginCat.slides(spring));
        const way = Paths.onward(coming.way, to, Object.assign({}, now,
            { facing: coming.way.facing }));
        return Object.assign(Ways.timed(way), { way: at.way, ahead: told.ahead, from });
    }
    if (to === null) {
        const way = Ways.begun(at, { x: from.x, y: from.y - lift });
        Ways.leg(way, "rides", way, { ms: MarginCat.slides(spring), slide: () => 0,
            posed: () => null });
        return Object.assign(Ways.timed(way), told);
    }
    const route = crosses(from, to, Object.assign({}, at, { spring, lift }));
    return route === null ? Paths.moves(from, to, at) : Object.assign(route, told);
}

// The quickest way down or up from `from` to `to`, each foot it can drop
// off, or head it can leap up off, and each catch across the gap tried:
// null where there's none, where its seat's on a floating window, or
// where it starts under the screen, which MarginCatPaths.js's walk gets
// onto, or brings it up from. The workspace it leaves is
// `at.lift` above the one it goes to, or under it, so a seat at the same
// spot on the screen as `from` is still a way across the gap.
function crosses(from, to, at) {
    const up = at.lift < 0;
    const now = Paths.columns(at.now, 0);
    const was = Paths.columns(at.was, 0);
    if (now.length === 0 || Floats.on(at.floats, to, at) !== null) {
        return null;
    }
    // On the workspace it leaves, it hurries, and the marks it would hop
    // stay on the glass as its ground slides away, so it keeps off them.
    const old = Object.assign({}, at, { cols: was, hurried: true, marks: null, floats: [] });
    const start = Falls.settle(Ways.begun(old, from), was, old);
    // Out of sight under the screen it has no ground to leave and no place
    // to leap from, so it comes up as MarginCatPaths.js's walk brings it.
    if (start.kind === "below") {
        return null;
    }
    const feet = start.kind === "floor" ? (up ? [] : [{ stops: [start], ms: 0 }])
        : start.kind === "ledge" || start.kind === "side" ? Routes.feet(start, was, old, up)
        : [];
    if (up && start.kind !== "floor") {
        feet.push(...launches(start, was, sides(now, at), old));
    }
    let best = null;
    for (const out of feet) {
        const foot = out.stops[out.stops.length - 1];
        for (const lands of up ? reaches(foot.x, now, at) : catches(foot.x, now, at)) {
            const after = below(foot, lands, to, at);
            const ms = after === null ? Infinity : out.ms + after.ends;
            if (ms < (best?.ms ?? Infinity)) {
                best = { ms, out, after };
            }
        }
    }
    if (best === null && up) {
        best = nearest(start, now, to, at);
    }
    return best === null ? null : joined(from, best, old);
}

// Each place along a top of `was` under one of `above`'s feet, where it
// can leap straight up the gap, as { stops, ms } from `start`: the leap
// starts anywhere along a top, never only from a head (VKCF3Z c9474cef).
function launches(start, was, above, old) {
    const found = [];
    for (const foot of above) {
        for (const [c, column] of was.entries()) {
            if (foot.x < column.left || foot.x + old.wide > column.right) {
                continue;
            }
            const stop = { kind: "ledge", c, x: foot.x, y: Routes.stands(column.rows[0].top, old),
                row: 0, how: "goes" };
            const stops = Routes.route(start, stop, was, old);
            if (stops !== null) {
                const way = Ways.begun(old, start);
                Paths.follows(way, stops, was, null);
                found.push({ stops, ms: Ways.timed(way).ends });
            }
        }
    }
    return found;
}

// With no side above over any top it could leap from, it stays where it
// is as its ground goes off the bottom, and comes in at the bottom edge
// on the foot of the side nearest it, never shifted sideways where it's
// seen (VKCF3Z c9474cef): null where there's no side at all.
function nearest(start, now, to, at) {
    const all = sides(now, at).sort((one, other) =>
        Math.abs(one.x - start.x) - Math.abs(other.x - start.x));
    for (const side of all) {
        const after = below({ x: side.x, y: start.y }, side, to, at);
        if (after !== null) {
            return { ms: after.ends, out: { stops: [start], ms: 0 }, after };
        }
    }
    return null;
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

// What it can leap up to from under the screen at `x` onto `now`: the
// foot of a side within reach across, as a stop hanging on it, since a
// leap up into a window's underside would go through it. None where
// there's no side in reach.
function reaches(x, now, at) {
    return sides(now, at).filter(foot => Math.abs(foot.x - x) <= REACHES * at.wide);
}

// The foot of each side of `now` it could hang on, on the screen.
function sides(now, at) {
    const feet = [];
    for (const [c, column] of now.entries()) {
        for (const side of [-1, 1]) {
            const foot = { kind: "side", c, side, x: Routes.sideX(column, side, at),
                y: column.bottom - at.tall, row: column.rows.length - 1, how: "grabs" };
            if (Routes.onScreen(foot.x, at) && !Routes.inMarks(foot.x, foot.y, at)
                && Falls.within(foot, now, at) === null) {
                feet.push(foot);
            }
        }
    }
    return feet;
}

// From out of sight beyond `foot`'s side, the drop or the leap up onto
// what it `lands` on, and on to its seat `to`, timed: null where there's
// no way on from there.
function below(foot, lands, to, at) {
    const now = Paths.columns(at.now, 0);
    const up = at.lift < 0;
    const facing = Math.sign(lands.x - foot.x) || Math.sign(to.x - foot.x) || at.facing || 1;
    const way = peeks(foot.x, up, facing, at);
    way.facing = facing;
    Ways.leap(way, lands, 0, up ? "springs" : "drops");
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

// All of a move as one way: on the workspace it leaves, `best.out`'s
// walk to its foot or head, kept until the slide has carried it off the
// top or bottom; out of sight a moment; then `best.after`'s drop or leap
// in and on to its seat.
function joined(from, best, old) {
    const lift = old.lift;
    const gone = y => (lift > 0 ? y + old.tall <= 0 : y >= old.height);
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
        if (gone(Ways.frame(walk, t).y - lift * MarginCat.slid(t, old.spring))) {
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
            ends: beat.ends + drops, since: beat.since + drops }));
    }
    // Till it drops into sight it can find its way again from where it
    // set off (MarginCatPaths.js's replans()).
    return { from, to: after.to, facing: after.facing, stride: after.stride, beats,
        ends: after.ends + drops, cell: old.cell, drops };
}
