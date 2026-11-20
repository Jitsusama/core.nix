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

// The faces found among `families()`, every face the shell can draw in, so
// they're asked for only until a face is there: asking takes a quarter of a
// millisecond offscreen, and a mark is made for every row the launcher
// makes.
const held = {};

// Whether `family` is there to draw in.
function faceHeld(family, families) {
    if (held[family] !== true && families().includes(family)) {
        held[family] = true;
    }
    return held[family] === true;
}

// `measure`, kept as `key`'s when `whole`: when the face drawn is the one
// asked for, not one standing in while it loads. Mark.qml gives it only
// once the outline is traced from the glyph, which comes after the box;
// or, for a pictured glyph, which has no outline, once its box is measured.
// A picture is kept only by its box, and nothing with an empty box at all:
// the launcher's rows once kept an emoji traced as a letter, an empty path
// a pixel and a half across beside an empty box, and every row after drew
// it as if it had no ink, its baseline on the capitals' middle, about 7
// device pixels high at 1.5 (Joel, 2026-11-19 19:39).
function measured(key, measure, whole) {
    const boxed = measure.width > 0 && measure.height > 0;
    const picture = pictured(key.split("\t").slice(2).join("\t"));
    const found = picture ? measure.pictured === true : measure.outline > 0;
    if (whole && boxed && found) {
        measures[key] = measure;
    }
}

// The characters drawn as pictures unasked, each run as its first and last
// code point: Unicode's Emoji_Presentation, from its emoji-data.txt (16.0),
// with the blocks past the Basic Multilingual Plane taken whole where most
// of a block is pictures, since Emoji lists nothing else of them. Arrows,
// ornaments and geometric shapes past it are letters, so they're left out.
const PICTURES = [
    [0x231a, 0x231b], [0x23e9, 0x23ec], [0x23f0, 0x23f0], [0x23f3, 0x23f3],
    [0x25fd, 0x25fe], [0x2614, 0x2615], [0x2648, 0x2653], [0x267f, 0x267f],
    [0x2693, 0x2693], [0x26a1, 0x26a1], [0x26aa, 0x26ab], [0x26bd, 0x26be],
    [0x26c4, 0x26c5], [0x26ce, 0x26ce], [0x26d4, 0x26d4], [0x26ea, 0x26ea],
    [0x26f2, 0x26f3], [0x26f5, 0x26f5], [0x26fa, 0x26fa], [0x26fd, 0x26fd],
    [0x2705, 0x2705], [0x270a, 0x270b], [0x2728, 0x2728], [0x274c, 0x274c],
    [0x274e, 0x274e], [0x2753, 0x2755], [0x2757, 0x2757], [0x2795, 0x2797],
    [0x27b0, 0x27b0], [0x27bf, 0x27bf], [0x2b1b, 0x2b1c], [0x2b50, 0x2b50],
    [0x2b55, 0x2b55], [0x1f004, 0x1f004], [0x1f0cf, 0x1f0cf], [0x1f18e, 0x1f18e],
    [0x1f191, 0x1f19a], [0x1f1e6, 0x1f1ff], [0x1f201, 0x1f202], [0x1f21a, 0x1f21a],
    [0x1f22f, 0x1f22f], [0x1f232, 0x1f23a], [0x1f250, 0x1f251], [0x1f300, 0x1f64f],
    [0x1f680, 0x1f6ff], [0x1f7e0, 0x1f7f0], [0x1f900, 0x1f9ff], [0x1fa70, 0x1faff]
];
// The selector that asks for a character's picture rather than its letter.
const AS_PICTURE = 0xfe0f;

// Whether `text` is drawn as a picture, in the colour emoji face, rather
// than as a letter with an outline: an emoji, or a character asking for
// its picture. A picture has no outline to trace, so Mark.qml measures it
// by its box. The theme's marks are the Nerd Fonts' letters, in the
// private areas, and none is among these: emoji-marks.qml says so for
// every mark the theme names (Joel, 2026-11-19: Emoji's groups).
function pictured(text) {
    for (const character of text) {
        const point = character.codePointAt(0);
        if (point === AS_PICTURE || PICTURES.some(([from, to]) => point >= from && point <= to)) {
            return true;
        }
    }
    return false;
}
