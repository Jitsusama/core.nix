.pragma library
// What Reopen Them All found gone, as Agents says it once the launcher
// opens next: "2 folders gone", then how many reopened. Said apart from
// Agents.qml so tests/session-row.qml holds it in Qt's own engine.

// One Reopen Them All over `asked` sessions: `reopen` is handed what to
// call for each one it finds gone, whenever it finds it, and `tell` hears
// how this run reads after each. Each run counts on its own, never on the
// row's: Agents keeps the row while what's to reopen stands (ownKept), so
// a count kept there added a second run's to the first's (core.nix#6037's
// F39), and one reset by each run was cut short by a second pressed while
// the first still walked, whose finds kept arriving (Sessions' reopenAll
// joins the second to the first's walk, each keeping its own `gone`).
function run(asked, reopen, tell) {
    let gone = 0;
    reopen(() => {
        gone += 1;
        tell({ gone, reopened: asked - gone });
    });
}

// How a run's reading is said, in `said`'s words: how many folders were
// found gone, one or many, then how many reopened.
function told(now, said) {
    return [
        [String(now.gone), String(said[now.gone === 1 ? "folderGone" : "foldersGone"] ?? "")],
        [String(now.reopened), String(said.reopened ?? "")]
    ];
}
