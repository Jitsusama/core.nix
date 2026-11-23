.pragma library
// What Reopen Them All found gone, as Agents says it once the launcher
// opens next: "2 folders gone", then how many reopened. Said apart from
// Agents.qml so tests/session-row.qml holds it in Qt's own engine.

// The tally a press of Reopen Them All counts in: the walk under way's
// (`walk`, what the last press returned), while Sessions still walks it,
// else one of its own. Sessions joins a list given mid-walk to the walk's
// end, dropping the sessions already queued or begun (Reopening.js's
// queuedOnce), and Agents' foot says only what was told last, so a press
// mid-walk adds its sessions to the walk's count rather than telling a
// count of its own over it. A press once the walk has ended counts afresh,
// never on the row's: Agents keeps the row while what's to reopen stands
// (ownKept), so a count kept there added a second run's to the first's
// (core.nix#6037's F39).
function joined(walk, walking) {
    return walk !== null && walking ? walk : { asked: {}, gone: {} };
}

// One press over the sessions of `ids`, counted in `tally`: `reopen` is
// handed what to call with each session it finds gone, whenever it finds
// it, and `tell` hears how the walk reads after each. Each session counts
// once, by its id, however many presses asked for it. Gives the tally, for
// the next press to join.
function run(tally, ids, reopen, tell) {
    for (const id of ids) {
        tally.asked[id] = true;
    }
    reopen(one => {
        tally.gone[one.id] = true;
        tell(reading(tally));
    });
    return tally;
}

// How many of a tally's sessions were found gone, and how many of the rest
// were asked to reopen.
function reading(tally) {
    const gone = Object.keys(tally.gone).length;
    return { gone, reopened: Object.keys(tally.asked).length - gone };
}

// How a run's reading is said, in `said`'s words: how many folders were
// found gone, one or many, then how many reopened.
function told(now, said) {
    return [
        [String(now.gone), String(said[now.gone === 1 ? "folderGone" : "foldersGone"] ?? "")],
        [String(now.reopened), String(said.reopened ?? "")]
    ];
}
