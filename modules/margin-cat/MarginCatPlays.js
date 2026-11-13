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
