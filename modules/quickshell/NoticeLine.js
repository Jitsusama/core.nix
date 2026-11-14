// What the arrival line says (NoticeLine.qml): which notice it shows,
// whether it's up, and how many more wait, worked out apart from the
// surface so tests/notice-line.qml can hold it in Qt's own engine. It
// words nothing, but choosing is ordering, which is the daemons' (Joel,
// 2026-11-12: "the daemons need to be the data layer"): it stands in until
// the notices daemon serves the front notice as a row of its own, critical
// first and gone once read or withdrawn, due Sun 15 with the line's quiet
// rules (2RCBOJ, 2026-11-13; Y2YBTS [042988b9]).
.pragma library

// The notice the line shows, by id, or -1: the first of the stack, in
// the order Stack.js keeps (a critical one in front, the rest as they
// came), that's still for Joel. One read or withdrawn leaves the line at
// once, even before its card goes, since it no longer asks anything of
// him (Joel, 2026-11-12: what's no longer applicable should disappear).
// While notices are held, only a critical one shows, as only a critical
// card does (Quiet.qml).
function chosen(shown, records, held) {
    for (const id of shown) {
        if (waits(records[id]) && (!held || records[id].urgency === "critical")) {
            return id;
        }
    }
    return -1;
}

// Whether a notice's record still asks something of Joel.
function waits(record) {
    return Boolean(record) && !record.read && !record.withdrawn;
}

// Whether the line is up with a notice chosen. It steps aside while the
// notices are gathered where the keys reach them, the launcher's or the
// held list's, so each is in one place; and under the talk line, which sits in
// the same place, only a critical one shows, since he's saying something.
function up(chosenId, gathered, talking, critical) {
    if (chosenId < 0 || gathered) {
        return false;
    }
    return !talking || critical;
}

// How many others wait behind the one shown, which the line counts at its
// far end, and Super+N reaches them all.
function more(shown, records, chosenId) {
    return shown.filter((id) => id !== chosenId && waits(records[id])).length;
}
