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
// only, so it never guesses. Before Code has read any row, which it does
// once it's first watched, the row is landed on as it comes; a key no row
// has once they're read lands on none, `unlisted`, which the foot says.
function linked(served, said) {
    const key = String(said ?? "").trim();
    if (key === "") {
        return { key, row: null, unlisted: true };
    }
    const row = rowKey(key);
    const read = served.some(one => one.startsWith(PREFIX));
    const unlisted = read && !served.includes(row);
    return { key, row: unlisted ? null : row, unlisted };
}

// Whether a drawn row is the one a link's `row`, from linked, lands on.
function lands(row) {
    return one => row !== null && (one?.servedKey ?? "") === row;
}
