.pragma library

// A throw kept from ending more than its own work: said as one line of the
// journal, and what waits on a frame run each in turn though one throws.

// A throw as one line of the journal: its message's first line and where it was thrown, its
// stack's first frame, or a fixed line for a value that can't be said as text, as a Symbol or
// an object with no prototype can't, so saying a throw never throws in its turn.
function shown(error) {
    try {
        const at = `${error?.stack ?? ""}`.split("\n")[0];
        const message = `${error?.message ?? error}`.split("\n")[0];
        return at === "" ? message : `${message} at ${at}`;
    } catch (unsaid) {
        return "a value that can't be said as text";
    }
}

// What waited on a frame, each run in turn, one that throws said in a line and the rest run
// still: the list is let go before it runs, so a throw ending the loop dropped the rest for
// good, as Quests' rows made ahead, whose callback alone clears its wait, would never be
// made ahead again (core.nix#6524's F17).
function runEach(due, said) {
    for (const then of due) {
        try {
            then();
        } catch (error) {
            said(shown(error));
        }
    }
}
