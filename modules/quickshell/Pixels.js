// How a pixel drawing of the theme's (jitsusama.theme.marks' `pixels`, the
// empty places' cat) is laid on a screen: as runs of inked cells, each cell
// a whole number of the screen's own pixels, so its edges are as sharp at
// 1.5 as at 2 rather than blurred across a pixel's split. Used by
// PixelMark.qml.
.pragma library

// The inked cells of `rows`, each a string with # for ink, as runs along
// each row: where it starts, which row and how many cells across. One run
// a row's stretch of ink, so a drawing is a handful of rectangles.
function runs(rows) {
    const found = [];
    rows.forEach((row, line) => {
        let start = -1;
        for (let x = 0; x <= row.length; x++) {
            const inked = x < row.length && row[x] === "#";
            if (inked && start < 0) {
                start = x;
            } else if (!inked && start >= 0) {
                found.push({ x: start, y: line, across: x - start });
                start = -1;
            }
        }
    });
    return found;
}

// How many of the screen's pixels a cell takes, for a drawing `cells`
// across standing about `size` logical pixels at `ratio` device pixels to
// one: the nearest whole number, never none, so every cell is the same.
function cell(size, cells, ratio) {
    return Math.max(1, Math.round((size * ratio) / cells));
}

// How far to move something at `at` logical pixels so it starts on one of
// the screen's pixels at `ratio`: at 1.5 a logical pixel is a pixel and a
// half, so a drawing at an odd one would split every edge.
function snap(at, ratio) {
    return Math.round(at * ratio) / ratio - at;
}
