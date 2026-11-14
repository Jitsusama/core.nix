// What the arrival line says (NoticeLine.qml): which notice it shows,
// whether it's up, and how many more wait, worked out apart from the
// surface so tests/notice-line.qml can hold it in Qt's own engine. It
// orders nothing: the notices daemon serves which one is in front, the
// first critical one and how many wait (modules/notices' front.rs), since
// the daemons are the data layer (Joel, 2026-11-12), and the line only
// picks between the two by what the shell alone knows.
.pragma library

// The notice the line shows, by id, or -1: the one in front, or while
// notices are held (Quiet.qml) only the first critical one, as only a
// critical card shows then. One read or withdrawn leaves the front at
// once, since it no longer asks anything of him (Joel, 2026-11-12: what's
// no longer applicable should disappear).
function chosen(front, held) {
    const id = held ? front?.critical : front?.id;
    return typeof id === "number" ? id : -1;
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
// far end, and Super+N reaches them all: every one waiting, the one shown
// aside.
function more(front, chosenId) {
    const waiting = front?.waiting ?? 0;
    return Math.max(0, chosenId < 0 ? waiting : waiting - 1);
}
