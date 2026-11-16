// The margin cat's plays that move it, once Joel has left the keys
// (MarginCatPlays.js draws which, MarginCat.qml says when): a hop, a pounce
// at nothing it's sure of, and its ball of yarn rolling up, batted, pounced
// on and sent off (Joel, 2026-11-12: "bat balls of yarn and stuff", "proper
// smooth animations with character to them"). Each is a way of the walk's
// own beats (MarginCatWays.js), so it keeps the rules every way keeps
// (RSCH-20261113-24R2QV): its art lands in whole cells, a leap rises,
// tucks and falls, and no drawing shows for less than two frames at 60 Hz.
// The yarn keeps them too, on a track of its own beside the cat's beats.
// Imported as Games.
.pragma library
.import "MarginCat.js" as MarginCat
.import "MarginCatFalls.js" as Falls
.import "MarginCatWays.js" as Ways

// How long each of its beats lasts, in ms at the theme's pace: the yarn
// rolling up as it sits, its ears going up, eyeing it (as long as it
// thinks, besides), the bat, watching it go, each wiggle before it leaps,
// holding it under a paw, sending it off, and looking about after a pounce.
// The yarn's roll in and its sending off take at least these, and longer
// from further off (FASTEST).
const ROLLS_IN = 1200;
const NOTICES = 140;
const EYES = 450;
const BATS = 160;
const WATCHES = 700;
const WIGGLES = 70;
const PINS = 360;
const SENDS = 1400;
const LOOKS = 500;
// A hop's crouch and landing (VKCF3Z, 5e211585).
const HOP = { crouches: 220, lands: 160 };
// How far things go, in the cat's own cells: where the yarn stops beside
// its paws, how far a bat sends it, how far a pounce leaps, how high it
// leaps, and how much ground the yarn covers for each drawing of its roll,
// so it turns as it goes rather than sliding.
const NEAR = 2;
const KNOCKS = 7;
const POUNCES = 7;
const LEAPS_UP = 3;
const TURNS_EVERY = 2;
// The yarn comes from its window's column's edge, which may be hundreds of
// cells off, so it's set off as fast as this at most, in cells a ms, and
// slows from there; and however fast it goes, each drawing of its roll
// holds this long, two frames at 60 Hz (VKCF3Z, edf2b2c2).
const FASTEST = 0.2;
const HOLDS = 34;
// Its wiggle, back a cell and forth again, so many times.
const TIMES = 2;
// A click on the yarn rolls it this many cells away from the click, by
// chance, slowing over FOLLOWS ms at the least, and the cat's bat after it
// sends it at most NUDGES more (VKCF3Z 73c59942).
const CLICKED = { least: 4, most: 8 };
const FOLLOWS = 600;
const NUDGES = 3;

// The room past each side of the cat from its `seat`, in logical pixels:
// to its window's column's ends and the screen's, and on the side the
// margin's `marks` are, half a `gap` short of them, since they say what's
// true and the yarn rolling over them would cover it. Kept over its own
// window, a play needs no tile but the one under it, as a wander does.
function room(at) {
    if (!at.seat) {
        return null;
    }
    let low = Math.max(0, at.column?.left ?? 0);
    let high = Math.min(at.across, at.column ? at.column.left + at.column.width : at.across);
    if (at.marks > 0) {
        const centre = at.across / 2;
        const reach = (at.marks * at.gap) / 2 + at.gap / 2;
        if (at.seat.x + at.wide <= centre - reach) {
            high = Math.min(high, centre - reach);
        } else if (at.seat.x >= centre + reach) {
            low = Math.max(low, centre + reach);
        }
    }
    return { left: at.seat.x - low, right: high - at.seat.x - at.wide };
}

// Its crouch a cell back from the way it plays, and its wiggle there,
// back a cell and forth again.
function gathers(way, dir) {
    const crouched = { x: way.x - dir * way.at.cell, y: way.y };
    Ways.holds(way, "crouches", MarginCat.CROUCHES, crouched);
    for (let i = 0; i < TIMES; i++) {
        Ways.holds(way, "wiggles", WIGGLES, { x: crouched.x - dir * way.at.cell, y: way.y });
        Ways.holds(way, "wiggles", WIGGLES, crouched);
    }
}

// A play's way timed, with what it is, the way it plays and the yarn's
// track by the cat's beats, for frame().
function played(kind, way, dir, yarn) {
    return Object.assign(Ways.timed(way), { kind, way: dir, yarn: yarn ?? null });
}

// The yarn: rolling out from `gone`, just behind its room's edge, to lie
// `near` its paws as it sits, batted on to `knocked` as it watches, lying
// there as it leaps on it, and sent back behind the edge to `gone`; drawn
// only `within` its room, so it comes out from behind the edge and goes
// back behind it, never appearing or going in open space (VKCF3Z,
// edf2b2c2). Its track by the cat's beats; lying, it faces the way it
// last rolled, `dir` once batted.
function yarnOf({ gone, near, knocked }, dir, within) {
    const lies = (still, facing) => ({ from: still, to: still, facing });
    return {
        within,
        tracks: {
            waits: { from: gone, to: near }, notices: lies(near, -dir), eyes: lies(near, -dir),
            bats: lies(near, -dir), watches: { from: near, to: knocked },
            crouches: lies(knocked, dir), wiggles: lies(knocked, dir),
            leaps: lies(knocked, dir), lands: lies(knocked, dir), pins: lies(knocked, dir),
            sends: { from: knocked, to: gone }
        }
    };
}

// How long the yarn takes to roll `cells`, slowing as it goes: `least`, or
// longer where that would set it off faster than FASTEST.
function rolls(cells, least) {
    return Math.max(least, (2 * cells) / FASTEST);
}

// A play from where the cat sits, `at`: its `seat`, the `column` under it,
// the screen's `across` and `height`, the margin's lit `marks` and the
// theme's `gap` between them, its art's `cell`, its `wide` and `tall`, how
// long it thinks (`pause`) and the theme's `slowdown`, as a walk takes them
// (MarginCatPaths.js). `kind` is "hop", a cell straight up from a crouch,
// landing crouched where it sat; "pounce", a leap at nothing it's sure of;
// or "yarn". It plays toward the more room, and is null where there's too
// little for it, so the scene moves on. It ends where it lands, facing the
// way it played; the scene walks it back from there.
function play(kind, at) {
    const spare = room(at);
    if (spare === null) {
        return null;
    }
    const c = at.cell;
    const way = Ways.begun(Object.assign({}, at, { facing: 0 }), at.seat);
    if (kind === "hop") {
        Ways.holds(way, "crouches", HOP.crouches);
        Ways.leap(way, at.seat, 1, "hops");
        Ways.holds(way, "lands", HOP.lands);
        return played(kind, way, 0);
    }
    const dir = spare.right >= spare.left ? 1 : -1;
    const ahead = dir > 0 ? spare.right : spare.left;
    const behind = dir > 0 ? spare.left : spare.right;
    const x = at.seat.x;
    // The yarn needs room to lie, be batted on and sent a roll further.
    const needs = kind === "yarn" ? NEAR + KNOCKS + TURNS_EVERY + MarginCat.CELLS : POUNCES;
    if (behind < 2 * c || ahead < needs * c) {
        return null;
    }
    // Where something `cells` past the cat's side would put its own left,
    // and how far past it the yarn is wholly behind its room's edge.
    const past = cells => (dir > 0 ? x + at.wide + cells * c : x - at.wide - cells * c);
    const far = Math.floor(ahead / c) + 1;
    const within = { left: x - spare.left, right: x + at.wide + spare.right };
    const yarn = kind === "yarn"
        ? yarnOf({ gone: past(far), near: past(NEAR), knocked: past(NEAR + KNOCKS) }, dir, within)
        : null;
    // A pounce lands seven cells on; on the yarn, its front paws on it, a
    // cell short of its middle.
    const lands = yarn ? yarn.tracks.watches.to - dir * (MarginCat.CELLS - 2) * c
        : x + dir * POUNCES * c;
    if (yarn) {
        Ways.holds(way, "waits", rolls(far - NEAR, ROLLS_IN));
    }
    way.facing = dir;
    Ways.holds(way, "notices", NOTICES);
    Ways.holds(way, "eyes", EYES + (at.pause ?? 0));
    if (yarn) {
        Ways.holds(way, "bats", BATS);
        Ways.holds(way, "watches", WATCHES);
    }
    gathers(way, dir);
    Ways.leap(way, { x: lands, y: at.seat.y }, LEAPS_UP);
    Ways.holds(way, "lands", MarginCat.LANDS);
    if (yarn) {
        Ways.holds(way, "pins", PINS);
        Ways.holds(way, "sends", rolls(far - NEAR - KNOCKS, SENDS));
    } else {
        Ways.holds(way, "looks", LOOKS);
    }
    return played(kind, way, dir, yarn);
}

// Its yarn by a click on the cat, `at` as play() takes it (Joel,
// 2026-11-12: "i want to be able to also use my mouse to engage with my
// animals."): rolling in from behind its column's edge toward the more
// room, as the play's does, never appearing at rest, to lie NEAR its paws
// as it notices and eyes it. It sits as it sat, and the yarn `rests` there
// once it's done, for a click on it (chase), with no timer and nothing
// drawn till then. Null where the yarn has no room to lie.
function fetch(at) {
    const spare = room(at);
    if (spare === null) {
        return null;
    }
    const c = at.cell;
    const dir = spare.right >= spare.left ? 1 : -1;
    const ahead = dir > 0 ? spare.right : spare.left;
    if (ahead < (NEAR + MarginCat.CELLS + 1) * c) {
        return null;
    }
    const x = at.seat.x;
    const past = cells => (dir > 0 ? x + at.wide + cells * c : x - at.wide - cells * c);
    const far = Math.floor(ahead / c) + 1;
    const near = past(NEAR);
    const lies = { from: near, to: near, facing: -dir };
    const yarn = {
        within: { left: x - spare.left, right: x + at.wide + spare.right },
        tracks: { waits: { from: past(far), to: near }, notices: lies, eyes: lies },
        rests: { x: near, facing: -dir }
    };
    const way = Ways.begun(Object.assign({}, at, { facing: 0 }), at.seat);
    Ways.holds(way, "waits", rolls(far - NEAR, ROLLS_IN));
    way.facing = dir;
    Ways.holds(way, "notices", NOTICES);
    Ways.holds(way, "eyes", EYES + (at.pause ?? 0));
    return played("fetch", way, dir, yarn);
}

// A click on its yarn, lying at `yarn` ({ x, facing }), on its `side` of
// the yarn (-1 its left, 1 its right), as the cat sits at `at`: the yarn
// rolls 4 to 8 cells away from the click by `chance` (0 to 1), slowing, as
// the cat's eyes follow it; never past a cell short of its room's ends, nor
// nearer the cat than NEAR. The cat walks up to it, NEAR short, where it's
// further off, and bats it on a few cells more, or, with no room for that,
// eyes it; then it sits, the yarn lying where it went, till the next click
// (VKCF3Z 73c59942: "Clicks are not a game loop"). Null where the yarn
// can't roll a cell that way.
function chase(at, yarn, side, chance) {
    const spare = room(at);
    if (spare === null || !yarn) {
        return null;
    }
    const c = at.cell;
    const x = at.seat.x;
    const within = { left: x - spare.left, right: x + at.wide + spare.right };
    const dir = side < 0 ? 1 : -1;
    // The yarn's left may go from a cell inside its room to a cell short of
    // its far end, and no nearer the cat than NEAR.
    const toward = yarn.x >= x ? 1 : -1;
    const low = Math.max(within.left + c, toward > 0 ? x + at.wide + NEAR * c : -Infinity);
    const high = Math.min(within.right - at.wide - c,
        toward < 0 ? x - at.wide - NEAR * c : Infinity);
    const free = Math.floor((dir > 0 ? high - yarn.x : yarn.x - low) / c + 1e-6);
    const wanted = CLICKED.least + Math.round(chance * (CLICKED.most - CLICKED.least));
    const cells = Math.min(wanted, free);
    if (cells < 1) {
        return null;
    }
    const rolledTo = yarn.x + dir * cells * c;
    const lies = (still, facing) => ({ from: still, to: still, facing });
    const tracks = { follows: { from: yarn.x, to: rolledTo } };
    const way = Ways.begun(Object.assign({}, at, { facing: toward }), at.seat);
    Ways.holds(way, "follows", rolls(cells, FOLLOWS));
    const stands = rolledTo - toward * (NEAR * c + at.wide);
    if (toward * (stands - x) > 1e-6) {
        Ways.goes(way, stands, at.seat.y);
        tracks.goes = lies(rolledTo, dir);
    }
    // A bat sends it on, away from the cat, short of its room's end.
    const onward = Math.floor((toward > 0 ? within.right - at.wide - c - rolledTo
        : rolledTo - within.left - c) / c + 1e-6);
    const nudges = Math.min(NUDGES, onward);
    let restsAt = rolledTo;
    let restsFacing = dir;
    if (nudges >= 1) {
        restsAt = rolledTo + toward * nudges * c;
        restsFacing = toward;
        tracks.bats = lies(rolledTo, dir);
        tracks.watches = { from: rolledTo, to: restsAt };
        Ways.holds(way, "bats", BATS);
        Ways.holds(way, "watches", rolls(nudges, WATCHES));
    } else {
        tracks.eyes = lies(rolledTo, dir);
        Ways.holds(way, "eyes", EYES);
    }
    return played("chase", way, toward, {
        within: yarn.within ?? within, tracks, rests: { x: restsAt, facing: restsFacing }
    });
}

// Let go of by Joel's hand at `at.seat`, `at` as play() takes it with its
// `air` under it (Joel, 2026-11-12: "i want to be able to also use my
// mouse to engage with my animals."): on the instant, it twists from its
// held drawing into the falling one and falls, as a leap's last third
// does, straight down to the nearest edge under it of `cols` (columns())
// or of the `floats` (MarginCatFloats.js's of()), a window's top or the
// gap between two, or to the screen's bottom, never rising to one; then
// lands, flicks an ear, and sits where it landed, for the keys to walk it
// on from (VKCF3Z, 9ead6710). Over a window's face, that's the next
// window's top or the floor. `landsOn` says what it landed on, a column's
// window or a floating one, `floats` or not, with its window's `id`, or
// null for the floor.
function drop(at, cols, floats) {
    const ledges = (cols ?? []).concat((floats ?? []).map(one => ({
        left: one.left, right: one.right, top: one.top, bottom: one.bottom,
        rows: [{ top: one.top, bottom: one.bottom, id: one.id }], floats: true
    })));
    const lands = Falls.under(at.seat, ledges, at, 0);
    const way = Ways.begun(Object.assign({}, at, { facing: 0 }), at.seat);
    if (lands.y > at.seat.y + 1e-6) {
        Ways.falls(way, { x: at.seat.x, y: lands.y });
    }
    Ways.holds(way, "lands", MarginCat.LANDS);
    Ways.holds(way, "looks", LOOKS);
    return Object.assign(played("drop", way, 0), {
        landsOn: lands.c < 0 ? null : { floats: ledges[lands.c].floats === true,
            id: ledges[lands.c].rows[lands.row]?.id ?? null }
    });
}

// How far a ball of yarn has rolled `p` of the way through its beat, as
// rolling slows it at a steady rate: from its fastest as it's set off to
// still as it gets there. The theme's ease-out is steeper, and had it
// cover two cells in under two frames at 60 Hz, then creep.
function rolled(p) {
    return 1 - (1 - p) * (1 - p);
}

// Where a play has the cat `t` ms in, as Ways.frame says, and its `yarn`:
// its left `x` and `y`, its drawing by the theme's name, the way it faces
// (the way it rolls, or last rolled) and the room it's drawn `within`;
// null where the play has none, or is done, but where a click's play
// leaves it lying (`rests`). Its drawing turns every TURNS_EVERY cells, or
// over more ground where it's set off so fast that a drawing would show
// for less than HOLDS ms; it only slows from there. It turns just past
// each stretch, so never on the cell it stops on.
function frame(playing, t) {
    const now = Ways.frame(playing, t);
    const beat = playing.beats.find(one => t < one.ends);
    const track = beat && playing.yarn ? playing.yarn.tracks[beat.name] : null;
    const rests = !beat ? playing.yarn?.rests : null;
    if (rests) {
        return Object.assign(now, { yarn: { x: rests.x, y: playing.from.y, pose: "yarn",
            facing: rests.facing, within: playing.yarn.within } });
    }
    if (!track) {
        return Object.assign(now, { yarn: null });
    }
    const c = playing.cell;
    const p = (t - beat.begins) / (beat.ends - beat.begins);
    const along = track.from + (track.to - track.from) * rolled(p);
    const x = Ways.onCells(along, track.from, track.to, c);
    const moving = track.to !== track.from;
    const fastest = (2 * Math.abs(track.to - track.from)) / (beat.ends - beat.begins);
    const every = Math.max(TURNS_EVERY * c, Math.ceil((fastest * HOLDS) / c) * c);
    const turns = Math.max(0, Math.ceil(Math.abs(x - track.from) / every - 1e-6) - 1);
    return Object.assign(now, { yarn: {
        x,
        y: playing.from.y,
        pose: moving ? ["yarnRollingA", "yarnRollingB"][turns % 2] : "yarn",
        facing: moving ? Math.sign(track.to - track.from) : track.facing,
        within: playing.yarn.within
    } });
}
