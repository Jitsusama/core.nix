.pragma library
// Where a mycelium:repository/ link lands in Code (Launcher.qml's
// showCode), which Code's page offers as each repository's Open in the
// Launcher, since a page never acts: the row is where Joel pulls it,
// forgets its gone trees or gathers its store. Kept apart from Code.qml so
// Qt's own engine runs its cases (tests/code-links.qml).

// How the programs daemon keys a repository's row (code_rows.rs's PLACE).
const PREFIX = "row:code:";

// The key of the row of the repository keyed `key`, as the daemon serves it,
// so a link can't name a row by another.
function rowKey(key) {
    return `${PREFIX}${key}`;
}

// The row a link lands on, from the keys Code's rows are served by: the
// repository whose key is the whole of what's said, trimmed, and whole keys
// only, so it never guesses. A key no row has lands on none, `unlisted`,
// which the foot says, but only judged by rows read whole in the watch
// now running (`fresh`, Daemon.qml's answeredThisWatch). Rows held from an
// earlier watch may be from before a clone the page already shows, so a
// link judged by them is landed on as its row comes instead.
function linked(served, said, fresh) {
    const key = String(said ?? "").trim();
    if (key === "") {
        return { key, row: null, unlisted: true };
    }
    const row = rowKey(key);
    const unlisted = fresh === true && !served.includes(row);
    return { key, row: unlisted ? null : row, unlisted };
}

// Whether a link landed on before Code's rows were read whole names a
// repository the watch then answers without: `waiting` is the row it
// landed on, none once it's judged, and `fresh` whether the rows `served`
// are read whole in the watch now running. Only then is a miss known, so
// the foot can say it as the rows come rather than leave the cursor
// waiting unsaid, which a link followed with the launcher shut, its watch
// not yet begun, always is.
function missed(waiting, served, fresh) {
    return waiting !== null && waiting !== "" && fresh === true && !served.includes(waiting);
}

// Whether a drawn row is the one a link's `row`, from linked, lands on.
function lands(row) {
    return one => row !== null && (one?.servedKey ?? "") === row;
}
