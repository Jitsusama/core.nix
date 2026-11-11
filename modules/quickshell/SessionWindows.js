.pragma library
// What Sessions.qml keeps of niri's windows, as the windows daemon
// (modules/windows) serves them ready, each a record saying its id, the
// process drawing it (`pid`), the app id it opened with (`opened`), where
// it sits (`workspace`, `column`, `down`) and when it was last used
// (`order`). Sessions finds each session's window among them, so only a
// change to one of those makes its lists again: a retitle, or the keys
// leaving a window, changes none, and is passed over.

// The fields Sessions reads of a window.
const read = ["id", "pid", "opened", "workspace", "column", "down", "order"];

// Whether a window's record differs from the one kept in what Sessions
// reads of it, or wasn't kept.
function moved(kept, record) {
    return !kept || read.some(field => kept[field] !== record[field]);
}

// The windows by the process drawing them, each kitty's oldest first, and
// by the app id each opened with, the oldest of each, as niri lists them.
function indexed(records) {
    const pids = {};
    const opened = {};
    for (const record of records.slice().sort((a, b) => a.id - b.id)) {
        if (record.pid) {
            pids[record.pid] = (pids[record.pid] ?? []).concat([record]);
        }
        if (!(String(record.opened) in opened)) {
            opened[String(record.opened)] = record;
        }
    }
    return { pids, opened };
}
