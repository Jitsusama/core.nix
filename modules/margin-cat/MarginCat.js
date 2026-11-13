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

// The longest a walk along the margin may take, in ms before the theme's
// slowdown, for a move up to walk it at once; a seat farther off is left
// to the usual rest and walk, so the walk is never hurried (VKCF3Z,
// 01f6c940: "please don't raise the pace").
const ARRIVES_WITHIN = 1500;

// A move between workspaces, a workspace's `step` (its height and niri's
// gap) the world slides by, the cat always moving with what goes up and
// staying where things fall away (VKCF3Z, 074321bd and 294b73a0). Going
// `way` 1, to the workspace below, it rides its old column up off the
// screen the whole slide, then climbs down from above the screen's top
// onto its new seat at the walk's pace, in the walk's strides, and lands
// as a walk does, crouched a moment, before it sits. Going -1, up, its old
// column falls away below it, so it stays in the margin the whole slide,
// then walks the margin to a seat no more than ARRIVES_WITHIN away; one
// farther off it stays for, the move `rests`, its `seat` still the one
// asked, and its usual walk takes it there once the keys rest. `from` and `to` are its seats, { x, y }, `tall`
// its height, `spring` niri's.
function moves(from, to, at) {
    const slow = at.slowdown ?? 1;
    const beats = [];
    let ends = 0;
    const beat = (name, lasts, more) => {
        const begins = ends;
        ends += lasts;
        beats.push(Object.assign({ name, begins, ends }, more));
    };
    const rides = slides(Object.assign({}, at.spring, { slowdown: slow }));
    if (at.way < 0) {
        return staysAbove(from, to, at, rides);
    }
    beat("rides", rides, {
        from,
        to: { x: from.x, y: from.y - at.way * at.step },
        facing: at.facing,
        slide: t => slid(t, Object.assign({}, at.spring, { slowdown: slow }))
    });
    const cells = Math.abs(to.y + at.tall) / at.cell;
    const pace = Math.min(Math.max(cells / (CRUISES / MS), AMBLES), HURRIES);
    const stride =
        Math.min(Math.max(pace * STRIDE_SECONDS, SHORTEST_STRIDE), LONGEST_STRIDE) * at.cell;
    // Head first the way it climbs: the theme's profile faces left, so a
    // quarter turn back has it face down.
    beat("climbs", (cells / pace) * MS * slow, {
        from: { x: to.x, y: -at.tall },
        to,
        ease: easeInOut,
        stride,
        turn: -1,
        facing: -1
    });
    beat("lands", LANDS * slow, { from: to, to, pose: "pouncing" });
    return { from, to, way: at.way, facing: at.facing, beats, ends };
}

// A move up (moves): it stays where it sits for the slide, `rides` ms
// long, then walks the margin to its seat if that's near enough, unhurried
// by the theme's slowdown in judging it, or else stays there and `rests`.
function staysAbove(from, to, at, rides) {
    const stays = { name: "stays", begins: 0, ends: rides, from, to: from, facing: at.facing };
    const along = walk(from.x, to.x, {
        cell: at.cell,
        ratio: at.ratio,
        slowdown: at.slowdown,
        hurried: true,
        facing: at.facing
    });
    const still = { from, way: at.way, facing: at.facing, beats: [stays], ends: rides };
    if (along === null) {
        return Object.assign(still, { to, rests: false });
    }
    if (along.ends > ARRIVES_WITHIN * (at.slowdown ?? 1)) {
        return Object.assign(still, { to: from, seat: to, rests: true });
    }
    const beats = along.beats.map(b => ({
        name: b.name,
        begins: rides + b.begins,
        ends: rides + b.ends,
        from: { x: b.from, y: to.y },
        to: { x: b.to, y: to.y },
        ease: b.ease,
        stride: b.stride,
        pose: b.pose,
        facing: along.facing
    }));
    return {
        from,
        to,
        way: at.way,
        facing: along.facing,
        beats: [stays].concat(beats),
        ends: rides + along.ends,
        rests: false
    };
}

// Where a move has it `t` ms after the slide set off: its `x` and `y`, its
// `pose` by the theme's name, or null for its own, as it rides; `turn`, a
// quarter turn clockwise (1) or back (-1) as it climbs, and 0 otherwise;
// the way it's `facing`, the `beat` it's on, and at the end, `done`.
function moved(moving, t) {
    const beat = moving.beats.find(b => t < b.ends);
    if (!beat) {
        return Object.assign({}, moving.to, {
            pose: null, turn: 0, facing: moving.facing, beat: "sits", done: true
        });
    }
    const p = (t - beat.begins) / (beat.ends - beat.begins);
    const eased = beat.slide ? beat.slide(t - beat.begins) : beat.ease ? beat.ease(p) : p;
    const x = beat.from.x + (beat.to.x - beat.from.x) * eased;
    const y = beat.from.y + (beat.to.y - beat.from.y) * eased;
    const ground = Math.abs(x - beat.from.x) + Math.abs(y - beat.from.y);
    const pose = beat.stride
        ? ["walkingA", "walkingB"][Math.floor(ground / beat.stride) % 2]
        : beat.pose ?? null;
    return {
        x,
        y,
        pose,
        turn: beat.turn ?? 0,
        facing: beat.facing ?? moving.facing,
        beat: beat.name,
        done: false
    };
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
        begun
    };
    const line = `cat: a ${counted?.what ?? "walk"} presented ${said.frames} of its ${begun} frames `
        + "a median "        + `${said.median} ms `
        + `apart, the p99 ${said.p99} ms, the slowest ${said.slowest} ms, each drawn within `
        + `${said.drawn} ms, ${unseen} never shown`;
    return { said, line };
}

// A drawing turned the way the cat faces: the theme draws its profiles
// facing left, so one going right is drawn mirrored, cell for cell.
function turned(drawing, way) {
    return way > 0 ? drawing.map(row => row.split("").reverse().join("")) : drawing;
}

// How lively the cat may be, from how engaged Joel is ("mild movements
// when i'm active, increeeasing as I'm less engaged", 2026-11-12), as
// `heard` says: `paused` ten seconds with no keys, `idle` a minute, `asleep`
// at the doze. It keeps still whatever else is true (VKCF3Z, eb64260b)
// while Do Not Disturb holds (`quiet`), since being left alone means the
// cat too; on a call, a camera in use (`onCamera`); with the screen
// `locked`; and where it's drawn on no screen (`unseen`), fullscreen,
// shared or in the overview, since a play there wakes for no one.
function level(heard) {
    if (heard.quiet || heard.onCamera || heard.locked || heard.unseen) {
        return "still";
    }
    if (heard.asleep) {
        return "asleep";
    }
    if (heard.idle) {
        return "idle";
    }
    return heard.paused ? "paused" : "typing";
}

// The rest before its next scene at a level (`lively`), in ms, `chance` (0 to
// 1) choosing where in its span: 45 to 75 s while he types, so a move is a
// rare thing at the edge of his eye; 5 to 12 s once he's paused, and 3 to 8
// s once he's left the keys (Joel, 2026-11-12: "play a bit more and not
// wait too long"); null where nothing comes next, asleep or still. Each is
// one timer, so nothing wakes between two plays.
function rest(lively, chance) {
    const spans = { typing: [45_000, 75_000], paused: [5000, 12_000], idle: [3000, 8000] };
    const span = spans[lively];
    return span ? Math.round(span[0] + (span[1] - span[0]) * chance) : null;
}

// What a single pose held while he types takes: long enough to be seen at
// the edge of his eye, short of a word's typing.
const GLANCE = 375;
const GLANCES = ["earFlick", "tailSwish", "blinking"];

// How far each beat is drawn from its own length, a quarter either way, so
// no two plays run alike; and the quickest any beat may be, so it never
// moves more than eight poses a second.
const VARIES = 0.25;
const QUICKEST = 125;

// Its plays in place, for while he's paused and reading (VKCF3Z,
// eb64260b): the sitting cat's own small moves, each a pose and how long
// it's held, sitting between them, and none over two seconds. Grooming
// licks its paw, lowering it and raising it again.
const IN_PLACE = {
    tailFlick: [["tailSwish", 300], ["nothingHere", 220], ["tailSwish", 300]],
    batting: [["batting", 260], ["nothingHere", 200], ["batting", 220], ["nothingHere", 240],
        ["batting", 260]],
    stretch: [["stretching", 600]],
    lookingAbout: [["lookingAbout", 600], ["nothingHere", 350], ["lookingAbout", 450]],
    grooming: [["grooming", 380], ["blinking", 180], ["grooming", 380], ["blinking", 180],
        ["grooming", 380]]
};

// Its moves, for once he's left the keys, a quarter of its plays then so
// it stays gentle: a hop a cell or two straight up from a crouch, landing
// crouched; and a wander of 16 to 40 cells along the margin, a sit, and
// back over its column, on the walk's own steps.
const MOVES = 0.25;
const HOP = { crouches: 220, rises: 380, lands: 160 };
const WANDERS = [16, 40];
const WANDER_SITS = 1500;

// One of `list`, `chance` (0 to 1) choosing which.
function pick(list, chance) {
    return list[Math.min(list.length - 1, Math.floor(chance * list.length))];
}

// A scene at a level (`lively`), its chances drawn from `draw` (0 to 1, as
// Math.random gives them), never the play named `last`: its `name` and its
// beats, each a pose held `ms`, after which it sits again. While he types
// it's a single pose, an ear flick, a tail swish or a blink; paused, one of
// the plays in place; idle, those three times in four, else a move. A
// hop's rise is a beat with `hop` and how many `cells` up; a wander's
// walks are beats with `walk`, `to` "wander", its `cells` and `way`, or
// "column", back. Asleep or still, none.
function scene(lively, draw, last) {
    const held = (pose, ms) => ({
        pose,
        ms: Math.max(QUICKEST, Math.round(ms * (1 - VARIES + 2 * VARIES * draw())))
    });
    if (lively === "typing") {
        const pose = pick(GLANCES.filter(glance => glance !== last), draw());
        return { name: pose, beats: [held(pose, GLANCE)] };
    }
    if (lively !== "paused" && lively !== "idle") {
        return { name: "", beats: [] };
    }
    const moving = lively === "idle" && draw() >= 1 - MOVES;
    const names = (moving ? ["hop", "wander"] : Object.keys(IN_PLACE)).filter(play => play !== last);
    const name = pick(names, draw());
    if (name === "hop") {
        const rises = Object.assign(held("pouncing", HOP.rises), { hop: true, cells: 1 + draw() });
        const crouched = held("pouncing", HOP.crouches);
        return { name, beats: [crouched, rises, held("pouncing", HOP.lands)] };
    }
    if (name === "wander") {
        const cells = Math.round(WANDERS[0] + (WANDERS[1] - WANDERS[0]) * draw());
        const off = { walk: true, to: "wander", cells, way: draw() < 0.5 ? -1 : 1 };
        const sat = held("nothingHere", WANDER_SITS);
        return { name, beats: [off, sat, { walk: true, to: "column" }] };
    }
    return { name, beats: IN_PLACE[name].map(([pose, ms]) => held(pose, ms)) };
}

// Where a short wander goes, from `at` as sits takes it with the `seat` it
// sits at: its `cells`, each `step` across, along the margin the `way` it
// was drawn, or the other where that one's off the screen or on the
// margin's marks, since there it would cover what's true or be nowhere;
// null where neither fits.
function wander(at) {
    const { seat } = at;
    if (!seat) {
        return null;
    }
    const reach = (at.marks * at.cell) / 2 + at.cell / 2;
    const fits = left => left >= 0 && left + at.wide <= at.across
        && (at.marks === 0 || left + at.wide <= at.across / 2 - reach
            || left >= at.across / 2 + reach);
    const x = [at.way, -at.way].map(way => seat.x + way * at.cells * at.step).find(fits);
    return x === undefined ? null : { x, y: seat.y };
}

// How high a hop has it `t` ms into its rise of `lasts`, in its cells: a
// thing thrown up and falling, none as it leaves and lands, `cells`
// midway.
function hopped(t, lasts, cells) {
    const p = Math.min(Math.max(t / lasts, 0), 1);
    return cells * 4 * p * (1 - p);
}
