// The order Now lists what's on in (WhatsOn.qml): by group, top to
// bottom, then by name within each, so what can need Joel always leads
// and a row keeps its place while its group stands (Joel, 2026-10-26,
// finding 15: "Now in its four groups"). Now's own rows, the time, the
// date, the weather and the next meeting, aren't among them; they're
// always last, after every group. Apart from the launcher, so
// tests/now-groups.qml holds it in Qt's own engine.
.pragma library

// Now's groups, top to bottom: what wants Joel, what reaches other people
// (a call, a share, a recording), what's playing, and the modes he left
// on. A piece names its group (pieces/Piece.qml), which is its kind but
// for one that has a group of its own, as what's playing does.
const GROUPS = ["wants", "onAir", "playing", "leftOn"];

// Whether a group is one Now may open on: what wants Joel, or what
// reaches other people, but never a mode he left on, which Return would
// turn off, nor what's playing (WhatsOn.qml's starts).
function pressing(group) {
    return group === "wants" || group === "onAir";
}

// Where a row's group comes: one Now doesn't know goes after them all,
// rather than ahead of what wants Joel.
function rank(row) {
    const at = GROUPS.indexOf(row.group);
    return at === -1 ? GROUPS.length : at;
}

// The rows, each { group, name } with whatever else it holds, in Now's
// order, the rows given left as they are. A launcher row's own `group`
// says it holds rows (Launcher.qml), so Now's rows are given beside
// their group rather than carrying it.
function ordered(rows) {
    return rows.slice().sort((a, b) => rank(a) - rank(b)
        || a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: "base" }));
}
