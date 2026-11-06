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
