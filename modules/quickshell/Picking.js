// What the region picker's keys do to its box (Picker.qml), kept apart from
// drawing so each move is tested as it is (tests/picking.qml). A box is
// { x, y, width, height } in a screen's logical pixels, and a screen is
// { width, height }. Every move keeps the box on the screen and at least
// `least` across, so no key can lose it. The keys are a Vim user's, read
// by `pressed`: the picker opens in normal, h j k l move the box and H J K
// L size it, f waits for a cell's letters, Return takes, - goes back,
// :q closes and Space ? lists the keys.
.pragma library

const least = 16;
// The character code of "a", the letter of the first row and column.
const LETTER_A = 97;

// Where the box starts: the middle quarter of the screen, so the first key
// moves or sizes something already there.
function start(screen) {
    const width = Math.round(screen.width / 2);
    const height = Math.round(screen.height / 2);
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
    return String.fromCharCode(LETTER_A + index);
}

// The cell two letters name, row then column, or null when there's none.
function cell(cells, letters) {
    if (letters.length !== 2) {
        return null;
    }
    const row = letters.charCodeAt(0) - LETTER_A;
    const column = letters.charCodeAt(1) - LETTER_A;
    if (row < 0 || row >= cells.rows || column < 0 || column >= cells.columns) {
        return null;
    }
    const x = Math.round(column * cells.width);
    const y = Math.round(row * cells.height);
    return {
        x, y,
        width: Math.round((column + 1) * cells.width) - x,
        height: Math.round((row + 1) * cells.height) - y
    };
}

// Whether a letter can still lead to a cell after what's been typed.
function fits(cells, letters) {
    const within = (code, count) => code >= 0 && code < count;
    if (letters.length === 0) {
        return true;
    }
    if (!within(letters.charCodeAt(0) - LETTER_A, cells.rows)) {
        return false;
    }
    return letters.length === 1 || within(letters.charCodeAt(1) - LETTER_A, cells.columns);
}

// The box from one cell to another, each corner where it lies.
function spanned(a, b) {
    const x = Math.min(a.x, b.x);
    const y = Math.min(a.y, b.y);
    return {
        x, y,
        width: Math.max(a.x + a.width, b.x + b.width) - x,
        height: Math.max(a.y + a.height, b.y + b.height) - y
    };
}

// How the keys stand before any is pressed: in normal, no count, no f
// waiting, no cell picked, no command line, no Space begun and no key list.
function rested() {
    return {
        count: "", finding: false, typed: "", first: null, command: null,
        leader: false, listing: false
    };
}

// Whether nothing is begun, so Escape has nothing to cancel but the list.
function resting(stood) {
    return stood.count === "" && !stood.finding && stood.command === null && !stood.leader;
}

// The picker's keys as Space ? lists them, grouped as the launcher's are
// (ListedKeys.js): normal's, then those an f waits for. Escape is never
// said, and the arrows only as their letters.
function keyList() {
    const line = (keys, does) => ({ heading: "", keys, does });
    return [
        { heading: "Normal", keys: "", does: "" },
        line("h j k l", "Move a step"),
        line("3l", "So many steps"),
        line("H J K L", "Size from the bottom right"),
        line("f", "Show the cells, then pick one"),
        line("f f", "From that cell to another"),
        line("\u21b5", "Take the box"),
        line("-", "Back to Capture"),
        line(":q", "Close"),
        line("Space ?", "These keys"),
        { heading: "Picking a cell", keys: "", does: "" },
        line("a\u2013z", "Its row, then its column"),
        line("\u232b", "Take a letter back")
    ];
}

// What the head says of the keys begun: the command line as typed, an f
// waiting with its letters, or the count.
function typed(stood) {
    if (stood.command !== null) {
        return `:${stood.command}`;
    }
    if (stood.finding) {
        return stood.typed === "" ? "f" : `f ${stood.typed}`;
    }
    return stood.leader ? "Space" : stood.count;
}

// Which way a key moves or sizes the box: h j k l and the arrows move it,
// as in Vim; their capitals and Shift with an arrow size it.
const ways = {
    h: ["left", moved], j: ["down", moved], k: ["up", moved], l: ["right", moved],
    Left: ["left", moved], Down: ["down", moved], Up: ["up", moved], Right: ["right", moved],
    H: ["left", sized], J: ["down", sized], K: ["up", sized], L: ["right", sized],
    "Shift+Left": ["left", sized], "Shift+Down": ["down", sized],
    "Shift+Up": ["up", sized], "Shift+Right": ["right", sized]
};

const quits = ["q", "q!", "quit", "quit!", "clo", "close"];
// A cell's letter, and a count's digit.
const LETTER = /^[a-z]$/;
const DIGIT = /^[0-9]$/;

// What a key does from where the keys stand, given the box, the cells,
// the screen and the step a move takes, as `view`: how the keys stand
// after it, the box, the act ("take", "back", "close" or none) and what
// to say, as Vim's E492 or "not here" for a key meaning nothing. The key
// list stays up through every key but Space ? again and an Escape with
// nothing begun.
function pressed(stood, key, view) {
    const read = stood.command !== null ? commanding(stood, key, view.box)
        : stood.finding ? finding(stood, key, view)
        : stood.leader ? leading(stood, key, view.box)
        : normal(stood, key, view);
    const listing = read.act === "keys" ? !stood.listing
        : key === "Escape" && resting(stood) ? false
        : stood.listing;
    return Object.assign(read, {
        stood: Object.assign(read.stood, { listing }),
        act: read.act === "keys" ? "" : read.act
    });
}

// After Space, which waits for its next key however long that takes, as
// the launcher's which-key does: ? lists the keys, Escape lets Space go,
// and any other means nothing here.
function leading(stood, key, box) {
    const kept = Object.assign(rested(), { first: stood.first });
    if (key === "?") {
        return answer(kept, box, "keys");
    }
    return key === "Escape" ? answer(kept, box) : answer(kept, box, "", "not here");
}

function answer(stood, box, act = "", said = "") {
    return { stood, box, act, said };
}

function commanding(stood, key, box) {
    const rest = rested();
    switch (key) {
    case "Escape":
        return answer(Object.assign(rest, { first: stood.first }), box);
    case "Return":
        return quits.includes(stood.command) ? answer(rest, box, "close")
            : answer(rest, box, "", `E492: Not an editor command: ${stood.command}`);
    case "Backspace":
        // Backspace on an empty line leaves it, as Vim's does.
        return answer(Object.assign({}, stood,
            { command: stood.command === "" ? null : stood.command.slice(0, -1) }), box);
    default:
        return key.length === 1 ? answer(Object.assign({}, stood,
            { command: stood.command + key }), box) : answer(stood, box);
    }
}

function finding(stood, key, view) {
    const { box, cells } = view;
    const waiting = Object.assign({}, stood, { finding: false, typed: "" });
    if (key === "Escape") {
        return answer(waiting, box);
    }
    if (key === "Return") {
        return answer(rested(), box, "take");
    }
    if (key === "Backspace") {
        return answer(Object.assign({}, stood, { typed: stood.typed.slice(0, -1) }), box);
    }
    const next = stood.typed + key;
    if (!LETTER.test(key) || !fits(cells, next)) {
        return answer(stood, box);
    }
    if (next.length < 2) {
        return answer(Object.assign({}, stood, { typed: next }), box);
    }
    const picked = cell(cells, next);
    // The first cell is the box; a second makes it from one to the other.
    return stood.first === null ? answer(Object.assign(waiting, { first: picked }), picked)
        : answer(Object.assign(waiting, { first: null }), spanned(stood.first, picked));
}

function normal(stood, key, view) {
    const { box, screen, step } = view;
    const counted = stood.count !== "";
    if (DIGIT.test(key) && (counted || key !== "0")) {
        return answer(Object.assign({}, stood, { count: stood.count + key }), box);
    }
    const rest = rested();
    const way = ways[key];
    if (way !== undefined) {
        const times = counted ? Number(stood.count) : 1;
        return answer(rest, way[1](box, way[0], step * times, screen));
    }
    switch (key) {
    case "f":
        return answer(Object.assign(rest, { finding: true, first: stood.first }), box);
    case "Space":
        return answer(Object.assign(rest, { leader: true, first: stood.first }), box);
    case "Return":
        return answer(rest, box, "take");
    case "-":
        return answer(rest, box, "back");
    case ":":
        return answer(Object.assign(rest, { command: "", first: stood.first }), box);
    case "Escape":
        return answer(Object.assign(rest, { first: stood.first }), box);
    default:
        return answer(Object.assign(rest, { first: stood.first }), box, "", "not here");
    }
}

// How a box is given to wf-recorder and grim: where it starts on the whole
// desktop, then its size.
function geometry(box, screen) {
    return `${screen.x + box.x},${screen.y + box.y} ${box.width}x${box.height}`;
}
