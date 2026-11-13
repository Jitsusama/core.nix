// The margin cat's play once Joel has left it a minute (MarginCat.qml), the
// yarn and the pounce ("bat balls of yarn and stuff", 2026-11-12): where
// the cat and its yarn are at each moment of one, worked out ahead as a
// walk is, so the shell only reads a frame of it each time the screen
// draws. Each move eases in and out, each leap is gathered for with a
// wiggle and lands crouched, and each is held a moment as thought (Joel,
// 2026-11-12: "proper smooth animations with character to them").
.pragma library

// How long each beat lasts, in ms at the theme's pace.
const NOTICES = 140;
const EYES = 450;
const BATS = 160;
const KNOCKED = 700;
const WIGGLES = 420;
const LEAPS = 380;
const LANDS = 120;
const HOLDS = 360;
const SENDS = 1400;
const LOOKS = 500;
const ROLLS_IN = 900;
// How far things go, in the cat's own cells: where the yarn stops beside
// its paws, how far a bat knocks it, and how far it draws back to wiggle.
const NEAR = 2;
const KNOCKS = 7;
const DRAWS_BACK = 1;
// How far the yarn comes from, at most, and how far the pounce leaps.
const COMES_FROM = 20;
const POUNCES = 7;
// How much of its own width the wiggle swings the cat, and how many times.
const SWINGS = 0.35;
const TIMES = 2;
// How much ground the yarn covers for each frame of its roll, in cells.
const TURNS_EVERY = 1.5;
// How high a leap goes, in cells, where the gap above the cat has room.
const HOPS = 3;
// Its drawings' width, in cells, the cat's and the yarn's alike.
const CELLS = 8;

// The theme's one ease-out curve's feel (docs/design.md's Motion), cheaper
// than its Bézier and near it, for anything coming to rest.
function easeOut(p) {
    return 1 - (1 - p) ** 3;
}

// Gathering speed and losing it, for a leap, which sets off from still.
function easeInOut(p) {
    return (1 - Math.cos(Math.PI * p)) / 2;
}

// The room past each side of the cat, from `at` as MarginCat.sits takes
// it, with its left `x`: to the screen's edges, and on the side the
// margin's marks are, to half a cell short of them, since they say what's
// true and the yarn rolling over them would cover it.
function room(at) {
    let left = at.x;
    let right = at.across - at.x - at.wide;
    if (at.marks > 0) {
        const centre = at.across / 2;
        const reach = (at.marks * at.cell) / 2 + at.cell / 2;
        if (at.x + at.wide <= centre - reach) {
            right = Math.min(right, centre - reach - at.x - at.wide);
        } else if (at.x >= centre + reach) {
            left = Math.min(left, at.x - centre - reach);
        }
    }
    return { left: Math.max(0, left), right: Math.max(0, right) };
}

// Which way it plays, +1 right or -1 left: toward the more room.
function side(spare) {
    return spare.right >= spare.left ? 1 : -1;
}

// A play's beats from its parts, each `[name, ms, cat, yarn]`: the cat's
// { pose, from, to, ease, lift, wiggle, facing } and the yarn's
// { from, to, ease, fades } or null while it's away, each `from` and `to`
// in logical pixels; beats lasting nothing left out.
function timed(parts, slow) {
    const beats = [];
    let ends = 0;
    for (const [name, ms, cat, yarn] of parts) {
        const begins = ends;
        ends += ms * slow;
        if (ends > begins) {
            beats.push({ name, begins, ends, cat, yarn });
        }
    }
    return { beats, ends };
}

// A play from where the cat sits, `at`: its left `x` and `cell` in logical
// pixels, the `room` past each side of it to the screen's edge or the
// margin's marks, `hop`, how high the gap above it lets it leap, `pause`,
// how long it thinks, and `slowdown`, the theme's. `kind` is "yarn", the
// yarn rolling up to it, batted away, pounced on and sent off, or
// "pounce", a leap at nothing it's sure of. Null where there's no room for
// it, so the scene moves on. It ends where the cat lands, facing the way it
// played; the shell walks it home from there.
function play(kind, at) {
    const way = side(at.room);
    const ahead = way > 0 ? at.room.right : at.room.left;
    const c = at.cell;
    const slow = at.slowdown ?? 1;
    const hop = Math.min(HOPS * c, at.hop ?? 0);
    const x = at.x;
    // Where something `cells` past the cat's side would put its own left.
    const past = cells => (way > 0 ? x + CELLS * c + cells * c : x - (CELLS + cells) * c);
    const sits = { pose: "nothingHere", from: x, to: x, facing: way };
    if (kind === "pounce") {
        if (ahead < POUNCES * c) {
            return null;
        }
        const back = x - way * DRAWS_BACK * c;
        const lands = x + way * POUNCES * c;
        return Object.assign(timed([
            ["notices", NOTICES, { pose: "earFlick", from: x, to: x, facing: way }, null],
            ["eyes", EYES + (at.pause ?? 0), Object.assign({}, sits, { pose: "tailSwish" }), null],
            ["wiggles", WIGGLES, { pose: "pouncing", from: x, to: back, ease: easeOut,
                wiggle: SWINGS * c, facing: way }, null],
            ["leaps", LEAPS, { pose: "walkingA", from: back, to: lands, ease: easeInOut,
                lift: hop, facing: way }, null],
            ["lands", LANDS, { pose: "pouncing", from: lands, to: lands, facing: way }, null],
            ["looks", LOOKS, { pose: "earFlick", from: lands, to: lands, facing: way }, null]
        ], slow), { kind, way, cell: c, from: x, to: lands });
    }
    const far = Math.min(COMES_FROM, ahead / c - CELLS);
    if (far < NEAR + KNOCKS + CELLS) {
        return null;
    }
    const near = past(NEAR);
    const knocked = past(NEAR + KNOCKS);
    const back = x - way * DRAWS_BACK * c;
    // It lands with its front paws on the yarn, a cell short of its middle.
    const lands = knocked - way * (CELLS - 2) * c;
    const gone = past(far);
    const lies = still => ({ from: still, to: still });
    return Object.assign(timed([
        ["rolls in", ROLLS_IN, { pose: "nothingHere", from: x, to: x, facing: 0 },
            { from: gone, to: near, ease: easeOut }],
        ["notices", NOTICES, { pose: "earFlick", from: x, to: x, facing: way }, lies(near)],
        ["eyes", EYES + (at.pause ?? 0), Object.assign({}, sits, { pose: "tailSwish" }),
            lies(near)],
        ["bats", BATS, Object.assign({}, sits, { pose: "batting" }), lies(near)],
        ["knocked", KNOCKED, sits, { from: near, to: knocked, ease: easeOut }],
        ["wiggles", WIGGLES, { pose: "pouncing", from: x, to: back, ease: easeOut,
            wiggle: SWINGS * c, facing: way }, lies(knocked)],
        ["leaps", LEAPS, { pose: "walkingA", from: back, to: lands, ease: easeInOut, lift: hop,
            facing: way }, lies(knocked)],
        ["lands", LANDS, { pose: "pouncing", from: lands, to: lands, facing: way },
            lies(knocked)],
        ["holds", HOLDS, { pose: "batting", from: lands, to: lands, facing: way },
            lies(knocked)],
        ["sends off", SENDS, { pose: "nothingHere", from: lands, to: lands, facing: way },
            { from: knocked, to: gone, ease: easeOut, fades: true }]
    ], slow), { kind, way, cell: c, from: x, to: lands });
}

// Where a play has the cat and its yarn `t` ms after it began: the cat's
// `x`, its `lift` above its seat, its `pose` by the theme's name and the
// way it's `facing` (0 as it sat, else +1 or -1); the yarn's `x`, `pose`,
// the way it faces and how opaque it is, or null while it's away; the
// `beat` it's on; and at its end, `done`, where it landed.
function frame(playing, t) {
    const beat = playing.beats.find(b => t < b.ends);
    if (!beat) {
        return { x: playing.to, lift: 0, pose: null, facing: playing.way, yarn: null,
            beat: "done", done: true };
    }
    const p = (t - beat.begins) / (beat.ends - beat.begins);
    const along = (move, ease) => move.from + (move.to - move.from) * (ease ? ease(p) : p);
    const cat = beat.cat;
    const swing = cat.wiggle ? cat.wiggle * Math.sin(2 * Math.PI * TIMES * p) : 0;
    const said = {
        x: along(cat, cat.ease) + swing,
        lift: cat.lift ? cat.lift * Math.sin(Math.PI * p) : 0,
        pose: cat.pose,
        facing: cat.facing,
        yarn: null,
        beat: beat.name,
        done: false
    };
    const yarn = beat.yarn;
    if (yarn) {
        const x = along(yarn, yarn.ease);
        const moving = yarn.to !== yarn.from;
        const rolled = Math.abs(x - yarn.from) / (TURNS_EVERY * playing.cell);
        said.yarn = {
            x,
            pose: moving ? ["yarnRollingA", "yarnRollingB"][Math.floor(rolled) % 2] : "yarn",
            // Rolling the way it goes, its strand behind it; lying, as drawn.
            facing: moving ? Math.sign(yarn.to - yarn.from) : 0,
            opacity: yarn.fades ? 1 - easeOut(p) : 1
        };
    }
    return said;
}
