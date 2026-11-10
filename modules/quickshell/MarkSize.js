// How large a mark draws beside type, whatever its shape, so every mark in
// a column, and every mark heading one, reads at one optical size: a wide
// one such as the keyboard doesn't read smaller than the rest (the launcher
// audit's finding 10), and a place's in the head reads as large as its
// rows' (VKCF3Z, 2026-11-06: Home's house drew smaller than Programs').
// Used by Choice.qml's rows and Prompt.qml's head.
.pragma library

// The size a mark's ink fills: fifteen sixteenths of the em, the face's
// size in pixels at 96 points to the inch, so 15 at Figure, a step lighter
// than the em beside the names (VKCF3Z, 2026-11-05).
function size(points) {
    return ((points * 4) / 3) * (15 / 16);
}

// What to scale a mark by whose ink is `wide` by `tall`: so its ink's area
// is the size's square, never taller than the size, so a narrow mark
// doesn't outgrow the type, and never wider than its `cell`, so it keeps
// its column. A mark not yet measured stays as it is.
function scale(wide, tall, points, cell) {
    if (wide <= 0 || tall <= 0) {
        return 1;
    }
    const fill = size(points);
    return Math.min(fill / Math.sqrt(wide * tall), fill / tall, cell / wide);
}

// Each glyph's ink as Mark.qml measured it, by its face, size and text
// (`key`), so a glyph is measured once and every row shown with it after
// reads what was found. Measuring is most of what a row costs to show
// anew: a row handed new words measured its glyph again at eight times its
// size and traced its outline, about 40% of a row's rebinding offscreen
// (2026-11-10). Kept for as long as the shell runs; it holds one entry for
// each glyph a mark has shown, a few hundred at most. Nothing reads it to
// be notified, so keeping in it changes no binding.
const measures = {};

// What was found of `key`'s glyph, or null while it's yet to be measured.
function known(key) {
    return measures[key] ?? null;
}

// `measure`, kept as `key`'s when `whole`: when the face drawn is the one
// asked for, not one standing in while it loads, and the outline is traced,
// which comes after the box. Either way it's what was measured.
function measured(key, measure, whole) {
    if (whole && measure.outline > 0) {
        measures[key] = measure;
    }
    return measure;
}
