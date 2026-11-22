.pragma library
// What Reopen Them All found gone, as Agents says it once the launcher
// opens next: "2 folders gone", then how many reopened. Said apart from
// Agents.qml so tests/session-row.qml holds it in Qt's own engine.

// A tally for To Reopen's row: how many its Reopen Them All asks to
// reopen, and how many of those it found gone since they were listed.
function begun(asked) {
    return { asked, gone: 0 };
}

// How the tally reads: how many were found gone, and how many reopened.
function said(tally) {
    return { gone: tally.gone, reopened: tally.asked - tally.gone };
}

// One Reopen Them All: `reopen` is handed what to call for each one found
// gone, and `tell` hears how the tally reads after each. Each run counts
// afresh, since Agents keeps the row, and so its tally, while what's to
// reopen stands (ownKept): a second run would otherwise add to the
// first's, saying more gone than went (core.nix#6037's F39).
function run(tally, reopen, tell) {
    tally.gone = 0;
    reopen(() => {
        tally.gone += 1;
        tell(said(tally));
    });
}
