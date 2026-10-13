// What the region picker's keys do to its box (Picker.qml), kept apart from
// drawing so each move is tested as it is (tests/picking.qml). A box is
// { x, y, width, height } in a screen's logical pixels, and a screen is
// { width, height }. Every move keeps the box on the screen and at least
// `least` across, so no key can lose it. The keys work as they do on text:
// letters, here a cell's, then the arrows to move and Shift to extend.
.pragma library

const least = 16;

// Where the box starts: the middle quarter of the screen, so the first key
// moves or sizes something already there.
function start(screen) {
    const width = Math.round(screen.width / 2), height = Math.round(screen.height / 2);
    return {
        x: Math.round((screen.width - width) / 2),
        y: Math.round((screen.height - height) / 2),
        width, height
    };
}

// An arrow moves the box a step, stopping at the screen's edge.
function moved(box, side, step, screen) {
    const clamp = (at, size, room) => Math.min(Math.max(0, at), room - size);
    const { x, y, width, height } = box;
    switch (side) {
    case "left":
        return { x: clamp(x - step, width, screen.width), y, width, height };
    case "right":
        return { x: clamp(x + step, width, screen.width), y, width, height };
    case "up":
        return { x, y: clamp(y - step, height, screen.height), width, height };
    default:
        return { x, y: clamp(y + step, height, screen.height), width, height };
    }
}

// Shift and an arrow size it from its bottom right corner, as Shift extends
// a selection of text: right and down grow it, left and up shrink it, to the
// screen's edge and no smaller than the least.
function sized(box, side, step, screen) {
    const { x, y, width, height } = box;
    switch (side) {
    case "left":
        return { x, y, width: Math.max(least, width - step), height };
    case "right":
        return { x, y, width: Math.min(screen.width - x, width + step), height };
    case "up":
        return { x, y, width, height: Math.max(least, height - step) };
    default:
        return { x, y, width, height: Math.min(screen.height - y, height + step) };
    }
}

// The grid that letters pick cells of: about sixteen across, as square as
// the screen allows. Rows are lettered from the top and columns from the
// left, alphabetically, so a cell's letters can be guessed as well as read.
function grid(screen) {
    const columns = 16;
    const rows = Math.max(1, Math.round(columns * screen.height / screen.width));
    return { columns, rows, width: screen.width / columns, height: screen.height / rows };
}

function letter(index) {
    return String.fromCharCode(97 + index);
}

// The cell two letters name, row then column, or null when there's none.
function cell(cells, typed) {
    if (typed.length !== 2)
        return null;
    const row = typed.charCodeAt(0) - 97, column = typed.charCodeAt(1) - 97;
    if (row < 0 || row >= cells.rows || column < 0 || column >= cells.columns)
        return null;
    const x = Math.round(column * cells.width), y = Math.round(row * cells.height);
    return {
        x, y,
        width: Math.round((column + 1) * cells.width) - x,
        height: Math.round((row + 1) * cells.height) - y
    };
}

// Whether a letter can still lead to a cell after what's been typed.
function fits(cells, typed) {
    const within = (code, count) => code >= 0 && code < count;
    if (typed.length === 0)
        return true;
    if (!within(typed.charCodeAt(0) - 97, cells.rows))
        return false;
    return typed.length === 1 || within(typed.charCodeAt(1) - 97, cells.columns);
}

// The box from one cell to another, each corner where it lies.
function spanned(a, b) {
    const x = Math.min(a.x, b.x), y = Math.min(a.y, b.y);
    return {
        x, y,
        width: Math.max(a.x + a.width, b.x + b.width) - x,
        height: Math.max(a.y + a.height, b.y + b.height) - y
    };
}

// How a box is given to wf-recorder and grim: where it starts on the whole
// desktop, then its size.
function geometry(box, screen) {
    return `${screen.x + box.x},${screen.y + box.y} ${box.width}x${box.height}`;
}
