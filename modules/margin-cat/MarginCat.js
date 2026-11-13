// Where the margin's cat sits and how big it's drawn (MarginCatScreen.qml): on
// the top edge of the column Joel is in, in the gap the windows leave above
// them, so it covers nothing of his and moves only where he already looks;
// the beats and paces of its walk, which MarginCatPaths.js lays its ways
// with; how niri's slide between workspaces moves the world it rides; and
// how it's drawn turned and told.
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

// How far niri's workspace slide has gone `t` ms after it set off, from 0
// to 1, on the spring niri's `workspace-switch` rides (the theme's
// motion.spring, with niri's mass of 1), slowed as niri slows its clock,
// so the cat rides its column exactly as the world moves. The spring
// sets off still: the cat never moves before the world does.
function slid(t, spring) {
    const w = Math.sqrt(spring.stiffness);
    const beta = spring.damping * w;
    const s = t / MS / (spring.slowdown ?? 1);
    const envelope = Math.exp(-beta * s);
    if (Math.abs(beta - w) < 1e-6) {
        return 1 - envelope * (1 + beta * s);
    }
    if (beta < w) {
        const w1 = Math.sqrt(w * w - beta * beta);
        return 1 - envelope * (Math.cos(w1 * s) + (beta / w1) * Math.sin(w1 * s));
    }
    const w2 = Math.sqrt(beta * beta - w * w);
    return 1 - envelope * (Math.cosh(w2 * s) + (beta / w2) * Math.sinh(w2 * s));
}

// How long the slide lasts, in ms: until it's within the spring's epsilon
// of its end and stays there, to the millisecond, as niri's clamped
// duration finds it.
const SLIDES_AT_MOST = 3000;
function slides(spring) {
    const slow = spring.slowdown ?? 1;
    let t = 1;
    while (t < SLIDES_AT_MOST && 1 - slid(t * slow, spring) > spring.epsilon) {
        t += 1;
    }
    return t * slow;
}

// How far into niri's slide the cat's first frame of it shows, in ms, on
// a screen drawn every `frame` ms: niri sets the slide off before the
// shell hears of it, and the cat's frames reach the screen a frame after
// niri's own. A GPU's nested niri had the window two and a half frames
// ahead of a cat told nothing of it, at 60 Hz and at 120.
function behind(frame) {
    return Math.round(2.5 * frame);
}

// A drawing given a quarter turn, clockwise for `turn` 1 and back for -1,
// cell for cell, so a climbing cat stays as sharp as a walking one.
function quartered(drawing, turn) {
    const n = drawing.length;
    const cells = drawing.map(row => row.split(""));
    return turn === 0 ? drawing : cells[0].map((_column, c) => cells.map((_row, r) => turn > 0
        ? cells[n - 1 - r][c]
        : cells[r][cells[0].length - 1 - c]).join(""));
}

// How a walk reached the screen, as the journal says it once it sits:
// `shown`, when niri presented each of its frames, in monotonic ms, so
// the gaps between them say whether it moved at the screen's rate;
// `drawing`, how long each took the shell to sync and render, its own
// cost of a frame; `unseen`, how many niri never showed (Joel,
// 2026-11-12: "we need proper smooth animations with character to them");
// and as `counted`, its `begun`, how many frames the shell began
// meanwhile, so a walk whose frames went unasked of says so rather than
// reading slower than it ran, and `what` it was: a walk, or a hop, said
// the same way.
function walked(shown, drawing, unseen, counted) {
    const begun = counted?.begun ?? 0;
    const gaps = shown.slice(1).map((at, i) => at - shown[i]).sort((a, b) => a - b);
    const tenths = ms => Math.round((ms ?? 0) * 10) / 10;
    const said = {
        frames: shown.length,
        median: tenths(gaps[(gaps.length - 1) >> 1]),
        p99: tenths(gaps[Math.max(0, Math.ceil(gaps.length * 0.99) - 1)]),
        slowest: tenths(gaps[gaps.length - 1]),
        drawn: tenths(drawing.reduce((most, ms) => Math.max(most, ms), 0)),
        unseen,
        begun,
        stepped: counted?.stepped ?? 0
    };
    const line = `cat: a ${counted?.what ?? "walk"} presented ${said.frames} of its ${begun} frames `
        + "a median "        + `${said.median} ms `
        + `apart, the p99 ${said.p99} ms, the slowest ${said.slowest} ms, each drawn within `
        + `${said.drawn} ms, ${unseen} never shown, stepped ${said.stepped} times`;
    return { said, line };
}

// A drawing turned the way the cat faces: the theme draws its profiles
// facing left, so one going right is drawn mirrored, cell for cell.
function turned(drawing, way) {
    return way > 0 ? drawing.map(row => row.split("").reverse().join("")) : drawing;
}
