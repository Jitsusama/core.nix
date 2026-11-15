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
// null where the play has none, or is done. Its drawing turns every TURNS_EVERY cells, or
// over more ground where it's set off so fast that a drawing would show
// for less than HOLDS ms; it only slows from there. It turns just past
// each stretch, so never on the cell it stops on.
function frame(playing, t) {
    const now = Ways.frame(playing, t);
    const beat = playing.beats.find(one => t < one.ends);
    const track = beat && playing.yarn ? playing.yarn.tracks[beat.name] : null;
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
