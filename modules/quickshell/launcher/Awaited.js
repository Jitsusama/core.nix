.pragma library

// What a search of what's typed does about the sources whose answers it
// waits for (Launcher.qml's searchNow). A source such as the notices,
// found by their daemon, answers a moment after the key, and a key's list
// waits for it a little so its rows land in one frame (Y2YBTS,
// 2026-11-11). Typing holds such a key and drops one held before it, whose
// search is never made, so no stale search runs. Every other key and
// setter that reads what was found searches the held key at once, without
// the answers not come, so Return acts on what the text finds rather than
// waiting for them or acting on the rows of the letters before.
//
// `now` is the launcher as the search comes: `due`, the keys not yet
// searched, which time it; `held`, the key held for answers, { due, ... },
// or null; `typing`, whether typing asked; and `asking`, whether a question
// is open, which waits on no answer. `waitingOn()` asks the sources whose
// search awaits an answer and gives them, called only when one could be
// held.
//
// Answers null when there's nothing to search; { hold: sources, due } to
// hold the key for them; or { due, waited } to search now, `waited` the
// held key whose wait this ends, or null where typing went over it.
function next(now, waitingOn) {
    if (now.due.length === 0 && (now.typing || now.held === null)) {
        return null;
    }
    const timed = (now.held?.due ?? []).concat(now.due);
    if (now.typing && !now.asking) {
        const sources = waitingOn();
        if (sources.length > 0) {
            return { hold: sources, due: timed };
        }
    }
    return { due: timed, waited: now.typing ? null : now.held };
}
