.pragma library
// What Sessions.qml keeps of niri's windows, as the windows daemon
// (modules/windows) serves them ready, each a record saying its id, the
// process drawing it (`pid`), the app id it opened with (`opened`) and
// where it sits (`workspace`, `column`, `down`). Sessions finds each
// session's window among them, so only a change to one of those makes its
// lists again: a retitle, the keys leaving a window or coming to it,
// changes none, and is passed over. When a window was last used
// (`order`) is read from the daemon's record as it stands, only where the
// sessions are shown or a place is picked (Sessions.qml's inUse), so the
// focus moving, while the launcher is shut above all, makes nothing.

// The fields Sessions reads of a window.
const read = ["id", "pid", "opened", "workspace", "column", "down"];

// Whether a window's record differs from the one kept in what Sessions
// reads of it, or wasn't kept.
function moved(kept, record) {
    return !kept || read.some(field => kept[field] !== record[field]);
}

// The windows by the process drawing them, and by the app id each opened
// with, oldest first, as niri lists them.
function indexed(records) {
    const pids = {};
    const opened = {};
    for (const record of records.slice().sort((a, b) => a.id - b.id)) {
        if (record.pid) {
            pids[record.pid] = (pids[record.pid] ?? []).concat([record]);
        }
        const app = String(record.opened);
        opened[app] = (opened[app] ?? []).concat([record]);
    }
    return { pids, opened };
}

// The window an app id names among those opened with it: the one, or of
// several, the one drawn by the session's own kitty (`pid`). Old spawns
// share small numbers, so pi-spawned.1 named four windows of four kittys
// (2QZBX9, 2026-11-11), and the oldest was another session's; one that
// can't be told apart names none rather than the wrong one.
function named(windows, pid) {
    if (windows.length === 1) {
        return windows[0];
    }
    const drawn = pid ? windows.filter(window => String(window.pid) === String(pid)) : [];
    return drawn.length === 1 ? drawn[0] : null;
}
