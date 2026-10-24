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
