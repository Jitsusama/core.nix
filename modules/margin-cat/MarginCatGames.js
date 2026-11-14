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
// The yarn comes slowly enough that each drawing of its roll holds two
// frames at 60 Hz as it sets off, its fastest.
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
// its paws, how far a bat sends it, how far a pounce leaps, how far the
// yarn comes from at most, how high it leaps, and how much ground the yarn
// covers for each drawing of its roll, so it turns as it goes rather than
// sliding.
const NEAR = 2;
const KNOCKS = 7;
const POUNCES = 7;
const COMES_FROM = 20;
const LEAPS_UP = 3;
const TURNS_EVERY = 2;
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

// The yarn: rolling up from `gone` to lie `near` its paws as it sits, batted
// on to `knocked` as it watches, lying there as it leaps on it, and sent
// back off to `gone`, coming into sight and fading from it as it goes.
function yarnOf(gone, near, knocked) {
    const lies = still => ({ from: still, to: still });
    return {
        waits: { from: gone, to: near, appears: true }, notices: lies(near), eyes: lies(near),
        bats: lies(near), watches: { from: near, to: knocked }, crouches: lies(knocked),
        wiggles: lies(knocked), leaps: lies(knocked), lands: lies(knocked), pins: lies(knocked),
        sends: { from: knocked, to: gone, fades: true }
    };
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
    if (behind < 2 * c || ahead < (kind === "yarn" ? NEAR + KNOCKS + MarginCat.CELLS
        : POUNCES) * c) {
        return null;
    }
    // Where something `cells` past the cat's side would put its own left.
    const past = cells => (dir > 0 ? x + at.wide + cells * c : x - at.wide - cells * c);
    const far = Math.min(COMES_FROM, Math.floor(ahead / c) - MarginCat.CELLS);
    const yarn = kind === "yarn" ? yarnOf(past(far), past(NEAR), past(NEAR + KNOCKS)) : null;
    // A pounce lands seven cells on; on the yarn, its front paws on it, a
    // cell short of its middle.
    const lands = yarn ? yarn.watches.to - dir * (MarginCat.CELLS - 2) * c : x + dir * POUNCES * c;
    if (yarn) {
        Ways.holds(way, "waits", ROLLS_IN);
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
        Ways.holds(way, "sends", SENDS);
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
// (0 lying as drawn, else the way it rolls) and how opaque it is; null
// where the play has none, or is done.
function frame(playing, t) {
    const now = Ways.frame(playing, t);
    const beat = playing.beats.find(one => t < one.ends);
    const track = beat && playing.yarn ? playing.yarn[beat.name] : null;
    if (!track) {
        return Object.assign(now, { yarn: null });
    }
    const p = (t - beat.begins) / (beat.ends - beat.begins);
    const x = Ways.onCells(track.from + (track.to - track.from) * rolled(p),
        track.from, track.to, playing.cell);
    const rolls = track.to !== track.from;
    const turns = Math.floor(Math.abs(x - track.from) / (TURNS_EVERY * playing.cell) + 1e-6);
    const opacity = track.appears ? Math.min(1, 2 * p) : track.fades ? 1 - MarginCat.easeOut(p)
        : 1;
    return Object.assign(now, { yarn: {
        x,
        y: playing.from.y,
        pose: rolls ? ["yarnRollingA", "yarnRollingB"][turns % 2] : "yarn",
        facing: rolls ? Math.sign(track.to - track.from) : 0,
        opacity
    } });
}
