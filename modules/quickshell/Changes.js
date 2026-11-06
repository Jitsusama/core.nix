// The changes u and Ctrl+R move between, as Vim's undo list: what's done,
// the last last, and what u took back, whatever each change holds. The
// held notices keep theirs here (HeldNotices.qml) and the launcher's rows
// theirs (Launcher.qml), so the two take back alike. Apart from the shell,
// so tests/query-keys.qml can hold it in Qt's own engine.
.pragma library

function changes() {
    return { done: [], undone: [] };
}

// A new change, which forgets what u took back, as Vim's does.
function changed(history, change) {
    return { done: history.done.concat([change]), undone: [] };
}

// u takes back the last change, as many as counted; Ctrl+R does again what
// u took back. Each says what moved, the newest first for u and the oldest
// first for Ctrl+R, or null where there was nothing.
function undo(history, count) {
    const by = Math.min(Math.max(count, 1), history.done.length);
    if (by === 0) {
        return { history, moved: null };
    }
    const back = history.done.slice(-by).reverse();
    return {
        history: { done: history.done.slice(0, -by), undone: history.undone.concat(back) },
        moved: back
    };
}

function redo(history, count) {
    const by = Math.min(Math.max(count, 1), history.undone.length);
    if (by === 0) {
        return { history, moved: null };
    }
    const again = history.undone.slice(-by).reverse();
    return {
        history: { done: history.done.concat(again), undone: history.undone.slice(0, -by) },
        moved: again
    };
}
