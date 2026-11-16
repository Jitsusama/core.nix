// Where the margin's cat sits and how big it's drawn (MarginCatScreen.qml): on
// the top edge of the column Joel is in, in the gap the windows leave above
// them, so it covers nothing of his and moves only where he already looks;
// the beats and paces of its walk, which MarginCatPaths.js lays its ways
// with; how niri's motion moves the windows it holds through or rides; and
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

// How long after the window it sat on closed the next change of its column
// is still that close's, in ms: niri lays the column out again once the
// windows left have resized, which it waits on for at most 300 ms.
const CLOSING = 500;

// A walk's beats, in ms at the theme's pace, each a pose of the theme's own
// (Joel, 2026-11-12: "proper smooth animations with character to them"):
// it stands and turns its head the way it's going, stops a moment to
// think, draws back a cell crouched, as anticipation, then walks, easing
// in over its first stride and out over its last, its strides stepped by
// the ground it covers so its feet never slide. It walks onto its seat and
// no further, and crouches a moment as it stops, before it sits (Joel,
// 2026-11-14 17:19: "i never want the kitty to skid into place").
const TURNS = 140;
const CROUCHES = 110;
const SETTLES = 180;
const LANDS = 120;
// How far, in its own cells, it draws back.
const DRAWS_BACK = 1;
// Its pace, in cells a second: an amble over a short way, each walk taking
// about `CRUISES` ms where it can, and never over `HURRIES`, so a long way
// is a gentle long walk, several seconds across a column, rather than a
// dash (VKCF3Z, 2026-11-12: fast motion at the edge of his eye pulls focus,
// and a walk blocks nothing of his). Each stride, one drawing, covers
// `STRIDE_SECONDS` at its pace: never under two cells, so four drawings
// walk its own length, nor over three at a run, so its paws are always
// seen moving (Joel, 2026-11-14 17:19: "i always want to see its paws
// moving when it is walking or running"; VKCF3Z d1d1ead2). Three cells a
// drawing held two frames at 60 Hz is about 90 cells a second, so a hurry
// is capped under it, rather than its stride stretched.
const AMBLES = 40;
const HURRIES = 80;
const CRUISES = 2000;
const STRIDE_SECONDS = 0.0375;
const SHORTEST_STRIDE = 2;
const LONGEST_STRIDE = 3;
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
// as a resize's edge goes on the same spring, so the cat rides a window's
// top exactly as it moves, and holds through a slide for as long as it
// lasts. The spring sets off still: the cat never moves before the world
// does.
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

// How far into niri's motion the cat's first frame of it shows, in ms, on
// a screen drawn every `frame` ms: niri sets a workspace's slide off
// before the shell hears of it, and the cat's frames reach the screen a
// frame after niri's own; a resize's layout waits on the window's commit
// and niri's next refresh as well. On a GPU's nested niri a slide's
// window was 42 ms ahead of a cat told nothing of it at 60 Hz and about
// 33 at 120, and a resize's 22 to 39 ms more (PLAN-20261112-V2U3HE, 24).
const HEARS = 25;
const RESIZES = 30;
function behind(frame, kind) {
    return Math.round(HEARS + frame + (kind === "resize" ? RESIZES : 0));
}

// Whether it's drawn over the windows rather than under them: on a leg
// of a walk `aboard` a floating window, onto it, along it or off it
// (MarginCatFloats.js), in front of the tiled window behind, which would
// hide it and have it pop on (Joel, 2026-11-15 23:34: "the whole kitty
// should not have to be rendered when you are seeing it come on to your
// screen"); or at rest while its `seat`, the column it sits on, is a
// floating window's, which sits in front of the tiled windows, so the one
// behind would hide it (Joel, 2026-11-13 18:59), and not once it's
// `leaving` that seat on any other leg. Anywhere else
// it's under them, so a window passing in front of it covers it rather
// than wearing it on its face, however late the shell hears niri: on top,
// a loaded machine showed it on a sliding window's face for two to four
// frames (VKCF3Z, 3fe38ef5).
function above(seat, leaving, aboard) {
    return aboard === true || (seat?.floats === true && !leaving);
}

// How far into a resize niri says began `began` ms before it told of it
// the cat's first frame shows, rather than RESIZES' guess at it, and a
// frame further: a cat late of the edge of a window growing down over it
// is in that window's face, and one early is in the gap it leaves
// (VKCF3Z, 3bc784ec). As `behind` gives with no word from niri.
function behindResize(frame, began) {
    return typeof began === "number" && began >= 0
        ? Math.round(HEARS + 2 * frame + began) : behind(frame, "resize");
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
// cost of a frame, said as the typical frame's and the slowest's, since
// a busy machine stretches a frame or two by what else it runs;
// `unseen`, how many niri never showed (Joel,
// 2026-11-12: "we need proper smooth animations with character to them");
// and as `counted`, its `begun`, how many frames the shell began
// meanwhile, so a walk whose frames went unasked of says so rather than
// reading slower than it ran, and `what` it was: a walk, or a hop, said
// the same way.
function walked(shown, drawing, unseen, counted) {
    const begun = counted?.begun ?? 0;
    const gaps = shown.slice(1).map((at, i) => at - shown[i]).sort((a, b) => a - b);
    const costs = drawing.slice().sort((a, b) => a - b);
    const tenths = ms => Math.round((ms ?? 0) * 10) / 10;
    const said = {
        frames: shown.length,
        median: tenths(gaps[(gaps.length - 1) >> 1]),
        p99: tenths(gaps[Math.max(0, Math.ceil(gaps.length * 0.99) - 1)]),
        slowest: tenths(gaps[gaps.length - 1]),
        typical: tenths(costs[(costs.length - 1) >> 1]),
        drawn: tenths(drawing.reduce((most, ms) => Math.max(most, ms), 0)),
        unseen,
        begun,
        stepped: counted?.stepped ?? 0
    };
    const line = `cat: a ${counted?.what ?? "walk"} presented ${said.frames} of its ${begun} frames `
        + "a median "        + `${said.median} ms `
        + `apart, the p99 ${said.p99} ms, the slowest ${said.slowest} ms, each drawn in `
        + `${said.typical} ms typically and within ${said.drawn} ms, ${unseen} never shown, `
        + `stepped ${said.stepped} times`;
    return { said, line };
}

// A drawing turned the way the cat faces: the theme draws its profiles
// facing left, so one going right is drawn mirrored, cell for cell.
function turned(drawing, way) {
    return way > 0 ? drawing.map(row => row.split("").reverse().join("")) : drawing;
}

// A pet (VKCF3Z, b8fc852e): a pointer moved onto the cat and resting on it
// PETS ms leans it in, held while it stays and PETTED ms after it goes, so
// a pointer passing over on its way elsewhere has left before it leans.
const PETS = 300;
const PETTED = 600;
// How long the cat has rested before a pointer over it is one that moved
// there rather than one it walked or sat down under: niri tells the shell
// of the pointer over its new place a frame or two after it gets there.
const SETTLED = 200;

// Whether the pointer, last seen over the cat at `was` (null as it comes
// onto it) and now at `at`, both in the screen's pixels, strokes it: moved
// there once the cat had rested `rested` ms (-1 while it goes), never a
// pointer at rest it came to under.
function strokes(was, at, rested) {
    return rested >= SETTLED && (was === null || was.x !== at.x || was.y !== at.y);
}

// Which way a cat `size` across leans into a pointer `across` it: -1 to
// its left, 1 to its right.
function leansTo(across, size) {
    return across < size / 2 ? -1 : 1;
}

// The rows of the cat's head and neck, which a lean moves; below them it
// sits on the seat's cells, whichever way it leans.
const HEAD_ROWS = 5;

// The cat leaning `way` (leansTo), in the theme's `pixels`: the pose leans
// to its right, toward its tail. To its left, the same head and neck a
// cell left of sitting's, over sitting's own body and tail, unmoved, so
// it never hops away from the hand (VKCF3Z, 10bacc58). Sitting's head is
// at its drawing's left edge, so that one starts a cell out (leansOut).
function leaning(way, pixels) {
    if (way > 0) {
        return pixels.leaning;
    }
    const head = row => `${row.slice(1)}..`;
    return pixels.nothingHere.map((row, r) => r < HEAD_ROWS ? head(pixels.leaning[r]) : `.${row}`);
}

// How many cells left of where it sits its drawing leaning `way` begins.
function leansOut(way) {
    return way < 0 ? 1 : 0;
}

// A swat (VKCF3Z, 73c59942): a pointer crossing the cat on its way
// elsewhere, gone before a pet would lean it in, has it bat after it about
// one time in three, its paw back down SWATS ms on, never twice within
// SWATS_APART ms, so it plays rather than nags, and never as it dozes,
// which only a pet wakes it from. The hold at full reach is two 60 Hz
// frames under 200 ms, so with the 83 ms the screen takes to show it and
// take it away the paw is back within 250 ms of showing (VKCF3Z, 38d40d08).
const SWATS = 166;
const SWATS_APART = 20000;

// Whether a pointer that `crossed` the cat, SINCE ms after its last swat,
// has it swat, by a `roll` in [0, 1); never `asleep`.
function swats(crossed, since, asleep, roll) {
    return crossed && since >= SWATS_APART && !asleep && roll < 1 / 3;
}

// The cat swatting `way` (leansTo), toward the side the pointer left by,
// in the theme's `pixels`: batting's raised paw, beside its head on its
// tail's side. To its left, the same paw mirrored beside sitting's head,
// over sitting's own body and tail, so only the paw moves; its drawing
// then starts three cells out (swatsOut).
function swatting(way, pixels) {
    if (way > 0) {
        return pixels.batting;
    }
    const paw = row => row.slice(5).split("").reverse().join("");
    return pixels.nothingHere.map((row, r) => r < HEAD_ROWS ? `${paw(pixels.batting[r])}${row}`
        : `...${row}`);
}

// How many cells left of where it sits its drawing swatting `way` begins.
function swatsOut(way) {
    return way < 0 ? 3 : 0;
}

// How the yarn at `yarn` (MarginCatGames.js's frame) looks, as a word that
// changes only as its pose or its facing does, so the screen builds its
// drawing again only then rather than each frame it rolls; "" with none.
function yarnLooks(yarn) {
    return yarn ? `${yarn.pose} ${yarn.facing}` : "";
}

// The yarn's drawing as it `looks` (yarnLooks): its pose's in `pixels`, the
// theme's, turned the way it faces.
function yarnDrawn(looks, pixels) {
    const [pose, facing] = looks.split(" ");
    return pose ? turned(pixels[pose] ?? [], Number(facing)) : [];
}
