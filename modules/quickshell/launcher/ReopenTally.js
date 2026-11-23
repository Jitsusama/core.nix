.pragma library
// What Reopen Them All found gone, as Agents says it once the launcher
// opens next: "2 folders gone", then how many reopened. Sessions counts
// each walk (Reopening.js's tallyOf), and this says its reading, apart from
// Agents.qml so tests/session-row.qml holds it in Qt's own engine.

// How a walk's reading is said, in `said`'s words: how many folders were
// found gone, one or many, then how many reopened.
function told(now, said) {
    return [
        [String(now.gone), String(said[now.gone === 1 ? "folderGone" : "foldersGone"] ?? "")],
        [String(now.reopened), String(said.reopened ?? "")]
    ];
}
