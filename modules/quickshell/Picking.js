// What the region picker's keys do to its box (Picker.qml), kept apart from
// drawing so each move is tested as it is (tests/picking.qml). A box is
// { x, y, width, height } in a screen's logical pixels, and a screen is
// { width, height }. Every move keeps the box on the screen and at least
// `least` across, so no key can lose it.
.pragma library

const least = 16;

// The arrows halve the box, keeping the half they point at.
function halved(box, side) {
    const across = side === "left" || side === "right";
    const whole = across ? box.width : box.height;
    const kept = Math.max(least, Math.floor(whole / 2));
    if (kept >= whole)
        return box;
    const near = side === "left" || side === "up";
    const start = near ? 0 : whole - kept;
    return across
        ? { x: box.x + start, y: box.y, width: kept, height: box.height }
        : { x: box.x, y: box.y + start, width: box.width, height: kept };
}

// Shift and an arrow move it by its own size, stopping at the edge.
function moved(box, side, screen) {
    const clamp = (at, size, room) => Math.min(Math.max(0, at), room - size);
    const { x, y, width, height } = box;
    switch (side) {
    case "left":
        return { x: clamp(x - width, width, screen.width), y, width, height };
    case "right":
        return { x: clamp(x + width, width, screen.width), y, width, height };
    case "up":
        return { x, y: clamp(y - height, height, screen.height), width, height };
    default:
        return { x, y: clamp(y + height, height, screen.height), width, height };
    }
}

// Ctrl and an arrow push that edge out by a step, to the screen's edge.
function pushed(box, side, step, screen) {
    const right = box.x + box.width, bottom = box.y + box.height;
    switch (side) {
    case "left": {
        const x = Math.max(0, box.x - step);
        return { x, y: box.y, width: right - x, height: box.height };
    }
    case "right": {
        const width = Math.min(screen.width, right + step) - box.x;
        return { x: box.x, y: box.y, width, height: box.height };
    }
    case "up": {
        const y = Math.max(0, box.y - step);
        return { x: box.x, y, width: box.width, height: bottom - y };
    }
    default: {
        const height = Math.min(screen.height, bottom + step) - box.y;
        return { x: box.x, y: box.y, width: box.width, height };
    }
    }
}

// Ctrl, Shift and an arrow pull that edge in by a step, the way it points,
// as the edge the arrow faces would move.
function pulled(box, side, step) {
    const width = Math.max(least, box.width - step), height = Math.max(least, box.height - step);
    switch (side) {
    case "left":
        return { x: box.x, y: box.y, width, height: box.height };
    case "right":
        return { x: box.x + box.width - width, y: box.y, width, height: box.height };
    case "up":
        return { x: box.x, y: box.y, width: box.width, height };
    default:
        return { x: box.x, y: box.y + box.height - height, width: box.width, height };
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
