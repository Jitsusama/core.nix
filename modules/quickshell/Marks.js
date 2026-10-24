// How the margin sets its glyphs, for Margin.qml. Kept apart from
// Quickshell's types, so the `the-margins-glyphs-stand-level` check can run
// it in Qt's own engine over ink it makes up (tests/marks.qml). Ink is a
// glyph's tight bounds as TextMetrics measures them: from the pen's start
// on the baseline, so its top is negative.
.pragma library

// From the corner inward, how much each kind asks: what's on air nearest
// the corner, then what wants Joel, then what was left on.
const KINDS = ["onAir", "wants", "leftOn"];

// How a glyph measured at one size is drawn so its ink stands `height`
// tall, centred on `middle`, its left at 0: the factor its font's size is
// scaled by, where its pen starts, where its baseline goes, and how wide
// its ink is then. A font draws each glyph at its own height, so a size
// alone can't level them; the ink can. One with no ink takes no room.
function fit(ink, height, middle) {
    if (ink.height <= 0) {
        return { scale: 1, x: 0, baseline: middle, width: 0 };
    }
    const scale = height / ink.height;
    return {
        scale,
        x: -ink.x * scale,
        baseline: middle - height / 2 - ink.y * scale,
        width: ink.width * scale
    };
}

// Things as { kind, order }, left to right: the corner's last, since it's
// read from there inward, by kind and then the overview's order.
// How much bigger the corner's marks grow while What's on is open, and how
// much of its cell a mark's drawing fills: half, so the room left above,
// below and either side is the same.
const GROWN = 2;
const FILLS = 2;

// The square cell each mark stands in, its drawing's height, and the room
// around it: at rest the gap's height, a mark that wants nothing of the
// window below; grown, twice that, as large as the drawing is drawn.
function sized(gap, grown) {
    const cell = grown ? gap * GROWN : gap;
    const ink = cell / FILLS;
    return { cell, ink, room: (cell - ink) / FILLS };
}

// Whether the launcher has What's on open, from { open, collection }: the
// name of the collection it holds first, empty for none.
function asked(launcher) {
    return launcher.open && launcher.collection === "on";
}

// The mark What's on points at, from the launcher's `keys`: the key of each
// row it holds and of the one selected, in order. A thing's row is
// `on:<mark>` and its actions' `verb:on:<mark>:<name>`, so the last key
// that names a thing names the one the keys are on. Empty for none.
function pointed(launcher) {
    if (!asked(launcher)) {
        return "";
    }
    const marks = launcher.keys
        .map(key => key.split(":"))
        .map(parts => (parts[0] === "verb" ? parts.slice(1) : parts))
        .filter(parts => parts[0] === "on" && parts.length > 1)
        .map(parts => parts[1]);
    const [last] = marks.slice(-1);
    return last ?? "";
}

function arranged(things) {
    const rank = thing => KINDS.indexOf(thing.kind);
    return things.slice().sort((a, b) => rank(b) - rank(a) || b.order - a.order);
}

// The theme's role a piece's glyph wears, which says only how much it
// asks: the alert for what needs Joel now, muted for what was left on,
// and strong for the rest.
function tone(piece) {
    if (piece.now) {
        return "alert";
    }
    return piece.kind === "leftOn" ? "muted" : "strong";
}

// What waits a moment before it's drawn, as { mark, until }: a thing whose
// program says its news as a notice (`app`) and has just come true, since
// its card follows a moment later, and a glyph drawn only to go as the card
// comes would flash. One still lit keeps its time; one gone or past it goes.
function awaited(waits, before, after, clock) {
    const { at, wait } = clock;
    const was = before.map(thing => thing.mark);
    const lit = after.map(thing => thing.mark);
    const stays = waits.filter(one => lit.includes(one.mark) && one.until > at);
    const held = stays.map(one => one.mark);
    const come = after
        .filter(thing => thing.app !== "" && !was.includes(thing.mark)
            && !held.includes(thing.mark))
        .map(thing => ({ mark: thing.mark, until: at + wait }));
    return stays.concat(come);
}

// How long the desktop or its apps have waited on the same thing, from
// lag-watch's last window (modules/lag-watch), as { worst, since } in epoch
// seconds, or null while nothing waits. The run lag-watch counts is of any
// trouble, so one whose worst changes starts again from now.
function lasted(window, kept, at) {
    if (!(window?.worst && window?.who)) {
        return null;
    }
    if (kept && kept.worst === window.worst) {
        return kept;
    }
    const since = kept || window.since === null || window.since === undefined ? at : window.since;
    return { worst: window.worst, since };
}

// What lingers at a time, as { mark, until, thing }: something on air
// that was lit before and isn't now stays a while, muted, in its place,
// so a camera let go is still seen to have been on. One lit again stops
// lingering, one past its time goes, and one already lingering keeps its
// time. What was left on or wants Joel goes at once, since nothing of it
// is private. The clock says the time now and how long to linger.
function lingers(kept, before, after, clock) {
    const { at, linger } = clock;
    const lit = after.map(thing => thing.mark);
    const stays = kept.filter(one => !lit.includes(one.mark) && one.until > at);
    const held = stays.map(one => one.mark);
    const went = before
        .filter(thing => thing.kind === "onAir" && !lit.includes(thing.mark)
            && !held.includes(thing.mark))
        .map(thing => ({
            mark: thing.mark,
            until: at + linger,
            thing: Object.assign({}, thing, { tone: "muted" })
        }));
    return stays.concat(went);
}
