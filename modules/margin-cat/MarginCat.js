// Where the margin's cat sits and how big it's drawn (MarginCat.qml): on
// the top edge of the column Joel is in, in the gap the windows leave above
// them, so it covers nothing of his and moves only where he already looks;
// and how it walks from one column to another, where it is at each moment
// of the way.
.pragma library

// The share of the gap the cat stands in, so a little of it is left above.
const STANDS = 2 / 3;
// How many cells its drawings are across and down (the theme's `pixels`).
const CELLS = 8;

// How many of the screen's pixels each of the cat's cells takes, in a gap
// `gap` logical pixels deep at `ratio` device pixels to one: whole ones,
// so it's as sharp at 1.5 as at 2, about two thirds of the gap all told,
// and never none.
function cell(gap, ratio) {
    return Math.max(1, Math.round((gap * STANDS * ratio) / CELLS));
}

// Where the cat sits, from `at`: the focused `column` ({ left, top, width }
// in the screen's logical pixels, as niri says it will come to rest), the
// screen's width `across`, how many `marks` the margin draws and each one's
// `cell`, and the cat's own size, `wide` by `tall`. It sits `air` above the
// column's top edge, one of its own cells, since the focused border is near
// its hue in both looks (1.19:1 in the light, 1.94:1 in the dark) and its
// feet would melt into it; over the middle of what's seen of the column,
// never past the screen's edge. One that would land on the margin's marks, centred on the screen,
// sits beside them on its column's side, half a cell off, since the marks
// say what's true and the cat says nothing. A column whose gap above can't
// hold it and its air, as in fullscreen, has it nowhere: null, as with
// nothing focused.
function sits(at) {
    const { column } = at;
    if (!column || column.top < at.tall + at.air) {
        return null;
    }
    const seenLeft = Math.max(0, column.left);
    const seenRight = Math.min(at.across, column.left + column.width);
    if (seenRight <= seenLeft) {
        return null;
    }
    const middle = (seenLeft + seenRight) / 2;
    let x = Math.min(Math.max(0, middle - at.wide / 2), at.across - at.wide);
    if (at.marks > 0) {
        const centre = at.across / 2;
        const reach = (at.marks * at.cell) / 2 + at.cell / 2;
        const from = centre - reach;
        const to = centre + reach;
        if (x + at.wide > from && x < to) {
            x = middle < centre ? from - at.wide : to;
        }
    }
    return { x, y: column.top - at.tall - at.air };
}

// How long the keys rest on another column before it sets off for it, in
// ms, so tapping h and l along the columns never sets it walking (VKCF3Z's
// spec: "When you rest on a column for two seconds, it trots over").
const RESTS = 2000;

// How long the keys' column may be nowhere before it goes too, in ms: longer
// than niri takes between two events of one move, short of a glance.
const LEAVES = 250;

// A walk's beats, in ms at the theme's pace, each a pose of the theme's own
// (Joel, 2026-11-12: "proper smooth animations with character to them"):
// it stands and turns its head the way it's going, stops a moment to
// think, draws back a cell crouched, as anticipation, then walks, easing
// in and out, its two strides stepped by the ground it covers so its feet
// never slide. It overshoots its seat by a cell and a half, steps back
// onto it, and crouches a moment as it stops, before it sits.
const TURNS = 140;
const CROUCHES = 110;
const SETTLES = 180;
const LANDS = 120;
// How far, in its own cells, it draws back and overshoots.
const DRAWS_BACK = 1;
const OVERSHOOTS = 1.5;
// Its pace, in cells a second: an amble over a short way, each walk taking
// about `CRUISES` ms where it can, and never over `HURRIES`, so a long way
// is a gentle long walk, several seconds across a column, rather than a
// dash (VKCF3Z, 2026-11-12: fast motion at the edge of his eye pulls focus,
// and a walk blocks nothing of his). Each stride covers a tenth of a second
// at its pace, and never under four cells, half its body, nor over ten, so
// its legs never blur.
const AMBLES = 40;
const HURRIES = 120;
const CRUISES = 2000;
const STRIDE_SECONDS = 0.1;
const SHORTEST_STRIDE = 4;
const LONGEST_STRIDE = 10;
const MS = 1000;

// The theme's one ease-out curve (docs/design.md's Motion), for the crouch
// and the steps back, as a cubic Bézier from (0, 0) to (1, 1) through
// (0.23, 1) and (0.32, 1), solved for its time by halving.
const EASE = [0.23, 1, 0.32, 1];
const HALVINGS = 24;

function bezier(a, b, s) {
    const r = 1 - s;
    return 3 * r * r * s * a + 3 * r * s * s * b + s * s * s;
}

function easeOut(p) {
    let low = 0;
    let high = 1;
    for (let i = 0; i < HALVINGS; i++) {
        const mid = (low + high) / 2;
        if (bezier(EASE[0], EASE[2], mid) < p) {
            low = mid;
        } else {
            high = mid;
        }
    }
    return bezier(EASE[1], EASE[3], (low + high) / 2);
}

// The walk's own curve, easing in as well as out, since a cat gathers speed
// from a standstill and the theme's curve sets off at full speed.
function easeInOut(p) {
    return (1 - Math.cos(Math.PI * p)) / 2;
}

// A walk from `from` to `to`, in logical pixels, at `at`'s `cell`, its own
// cell in logical pixels, and `ratio`. `pause` is how long it stops to
// think, `slowdown` the theme's (motion.slowdown), and `hurried` a walk
// begun on the way, its seat moved under it, which neither thinks nor
// crouches and turns only if it now goes back the way it came from
// `facing`. It's null where it's already there, to the screen's pixel.
function walk(from, to, at) {
    if (Math.round(from * at.ratio) === Math.round(to * at.ratio)) {
        return null;
    }
    const slow = at.slowdown ?? 1;
    const facing = Math.sign(to - from);
    const cells = Math.abs(to - from) / at.cell;
    const pace = Math.min(Math.max(cells / (CRUISES / MS), AMBLES), HURRIES);
    const stride =
        Math.min(Math.max(pace * STRIDE_SECONDS, SHORTEST_STRIDE), LONGEST_STRIDE) * at.cell;
    const back = at.hurried ? from : from - facing * DRAWS_BACK * at.cell;
    const past = to + facing * OVERSHOOTS * at.cell;
    const beats = [];
    let ends = 0;
    const beat = (name, lasts, more) => {
        const begins = ends;
        ends += lasts * slow;
        beats.push(Object.assign({ name, begins, ends, from, to: from }, more));
    };
    if (!at.hurried || at.facing !== facing) {
        beat("turns", TURNS, { pose: "walkingB" });
    }
    if (!at.hurried) {
        beat("thinks", at.pause ?? 0, { pose: "walkingB" });
        beat("crouches", CROUCHES, { pose: "pouncing", to: back, ease: easeOut });
    }
    const goes = (Math.abs(past - back) / at.cell / pace) * MS;
    beat("goes", goes, { from: back, to: past, ease: easeInOut, stride });
    beat("settles", SETTLES, { pose: "walkingB", from: past, to, ease: easeOut });
    beat("lands", LANDS, { pose: "pouncing", from: to, to });
    return { from, to, facing, stride, beats: beats.filter(b => b.ends > b.begins), ends };
}

// Where a walk has it `t` ms after it set off: its `x`, its `pose` by the
// theme's name, the way it's `facing`, and the `beat` it's on; at its end,
// sitting at `to`, `done`, with no pose of the walk's own.
function step(going, t) {
    const beat = going.beats.find(b => t < b.ends);
    if (!beat) {
        return { x: going.to, pose: null, facing: going.facing, beat: "sits", done: true };
    }
    const p = (t - beat.begins) / (beat.ends - beat.begins);
    const x = beat.from + (beat.to - beat.from) * (beat.ease ? beat.ease(p) : p);
    // Legs apart, then gathered, a stride each, by the ground covered.
    const pose = beat.stride
        ? ["walkingA", "walkingB"][Math.floor(Math.abs(x - beat.from) / beat.stride) % 2]
        : beat.pose;
    return { x, pose, facing: going.facing, beat: beat.name, done: false };
}

// How a walk reached the screen, as the journal says it once it sits:
// `shown`, when niri presented each of its frames, in monotonic ms, so
// the gaps between them say whether it moved at the screen's rate;
// `drawing`, how long each took the shell to sync and render, its own
// cost of a frame; and `unseen`, how many niri never showed (Joel,
// 2026-11-12: "we need proper smooth animations with character to them").
function walked(shown, drawing, unseen) {
    const gaps = shown.slice(1).map((at, i) => at - shown[i]).sort((a, b) => a - b);
    const tenths = ms => Math.round((ms ?? 0) * 10) / 10;
    const said = {
        frames: shown.length,
        median: tenths(gaps[(gaps.length - 1) >> 1]),
        p99: tenths(gaps[Math.max(0, Math.ceil(gaps.length * 0.99) - 1)]),
        slowest: tenths(gaps[gaps.length - 1]),
        drawn: tenths(drawing.reduce((most, ms) => Math.max(most, ms), 0)),
        unseen
    };
    const line = `cat: a walk presented ${said.frames} frames a median ${said.median} ms `
        + `apart, the p99 ${said.p99} ms, the slowest ${said.slowest} ms, each drawn within `
        + `${said.drawn} ms, ${unseen} never shown`;
    return { said, line };
}

// A drawing turned the way the cat faces: the theme draws its profiles
// facing left, so one going right is drawn mirrored, cell for cell.
function turned(drawing, way) {
    return way > 0 ? drawing.map(row => row.split("").reverse().join("")) : drawing;
}

function level(heard) {
    return "typing";
}

function rest(at, chance) {
    return 0;
}

function scene(at, chance) {
    return [];
}

function gutter(at) {
    return null;
}
