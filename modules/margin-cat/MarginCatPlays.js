// What the margin's cat plays at (MarginCat.qml), and when: how lively Joel
// leaves it, how long it rests between scenes, and each scene's beats,
// apart from where it sits and how it walks (MarginCat.js). Imported as
// Plays.
.pragma library

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
// it stays gentle: a hop a cell straight up from a crouch, landing
// crouched; a wander of 16 to 40 cells along the margin, a sit, and back
// over its column, on the walk's own steps; its ball of yarn rolling up
// to it, batted, pounced on and sent off; and a pounce at nothing it's
// sure of (Joel, 2026-11-12: "bat balls of yarn and stuff"), each of the
// last two walking it back over its column from where it lands. How a hop,
// the yarn and a pounce move is MarginCatGames.js's; how long it thinks
// before the last two, in ms, is drawn from between THINKS.
const MOVES = 0.25;
const MOVING = ["hop", "wander", "yarn", "pounce"];
const THINKS = [150, 450];
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
// hop, the yarn or a pounce is a beat with `play`, its kind, and how long
// it thinks (`pause`), the last two then walking back; a wander's
// walks are beats with `walk`, `to` "wander", its `cells` and `way`, or
// "column", back. Asleep or still, none. A play `asked` for by name, as a
// test asks for the one it watches, comes in place of the one drawn where
// the level has it, the draws taken as ever.
function scene(lively, draw, last, asked) {
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
    const names = (moving ? MOVING : Object.keys(IN_PLACE)).filter(play => play !== last);
    const drawn = pick(names, draw());
    const allowed = lively === "idle" ? [...Object.keys(IN_PLACE), ...MOVING]
        : Object.keys(IN_PLACE);
    const name = allowed.includes(asked) ? asked : drawn;
    if (name === "hop") {
        return { name, beats: [{ play: name, pause: 0 }] };
    }
    if (name === "yarn" || name === "pounce") {
        const pause = Math.round(THINKS[0] + (THINKS[1] - THINKS[0]) * draw());
        return { name, beats: [{ play: name, pause }, { walk: true, to: "column" }] };
    }
    if (name === "wander") {
        const cells = Math.round(WANDERS[0] + (WANDERS[1] - WANDERS[0]) * draw());
        const off = { walk: true, to: "wander", cells, way: draw() < 0.5 ? -1 : 1 };
        const sat = held("nothingHere", WANDER_SITS);
        return { name, beats: [off, sat, { walk: true, to: "column" }] };
    }
    return { name, beats: IN_PLACE[name].map(([pose, ms]) => held(pose, ms)) };
}

// What comes next at a level (`lively`): the rest before it, `after` ms,
// and the `scene` it plays then, drawn from `draw` never as `last`; null
// where nothing comes, asleep or still, so no timer runs at all; a play
// `asked` for comes as scene() says. MarginCat.qml waits `after` and plays
// it, and a change of level draws afresh.
function next(lively, draw, last, asked) {
    const after = rest(lively, draw());
    return after === null ? null : { after, scene: scene(lively, draw, last, asked) };
}

// Where a short wander goes, from `at` as sits takes it with the `seat` it
// sits at: its `cells`, each `step` across, along the margin the `way` it
// was drawn, or the other where that one's off its window's `column`,
// off the screen or on the margin's marks, since there it would cover
// what's true or be nowhere; null where neither fits. Kept over its own
// window, a wander needs no tile but the one under it, so the shut shell
// hears no other window's resizing while it plays.
function wander(at) {
    const { seat } = at;
    if (!seat) {
        return null;
    }
    const reach = (at.marks * at.cell) / 2 + at.cell / 2;
    const over = left => !at.column
        || (left >= at.column.left && left + at.wide <= at.column.left + at.column.width);
    const fits = left => over(left) && left >= 0 && left + at.wide <= at.across
        && (at.marks === 0 || left + at.wide <= at.across / 2 - reach
            || left >= at.across / 2 + reach);
    const x = [at.way, -at.way].map(way => seat.x + way * at.cells * at.step).find(fits);
    return x === undefined ? null : { x, y: seat.y };
}

