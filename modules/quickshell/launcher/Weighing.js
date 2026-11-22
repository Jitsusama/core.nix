.pragma library

// What the launcher weighs of its heap while it's shut, and when
// (Launcher.qml): the holders of what's live, counted once a shell, then
// what letting go of each of the largest would free, one a turn, each a
// whole heap's marking. Its state is { counted, owed, queue }: whether the
// holders were counted, whether a count is owed, and the names still to
// weigh, the largest first. Each step gives the state after it, so a
// busy answer or an opening that put a turn off leaves the work owed for
// the next closing to take up.

// How many holders letting go of is weighed. holders() credits what two
// holders share to the one whose name sorts first, so a holder sharing
// with a larger one can sit well below it, as the ready rows' `rows`
// would under `records`; six reach further down than the three largest,
// each weighed once a shell, a second apart while the launcher is shut.
const WEIGHED = 6;

// Nothing counted yet and nothing owed, as a shell starts.
function begun() {
    return { counted: false, owed: false, queue: [] };
}

// Whether letting go of what a name holds is a cut that could be made. A
// name holding `~` is every property the journal's filter folded into it,
// which no one cut could free. A file's functions, each made anew for
// every object, are kept: moving them into a .js is the cut that frees
// them.
function cuttable(name) {
    return !name.split(" ").includes("~");
}

// The names to weigh, from holders()' rows, [name, ...] the largest first.
function toWeigh(rows) {
    return rows.map(row => row[0]).filter(cuttable).slice(0, WEIGHED);
}

// What a closing starts once it has collected: the census once it's due,
// which goes on to the count's turn itself; else a resume while a count or
// a weighing is owed; else nothing.
function afterClosing(state, censusDue) {
    if (censusDue) {
        return "census";
    }
    return state.owed || state.queue.length > 0 ? "resume" : "none";
}

// A census done: a count is owed unless the holders were counted already.
// The count's turn comes next either way, and takes up a weighing owed.
function afterCensus(state) {
    return { counted: state.counted, owed: !state.counted, queue: state.queue };
}

// What the count's turn does: count the holders if they aren't yet, else
// weigh while any is still to be weighed, else nothing.
function countsTurn(state) {
    if (!state.counted) {
        return "count";
    }
    return state.queue.length > 0 ? "weigh" : "none";
}

// A count's answer, as holders() gives it: a busy one leaves the count
// owed; else the holders are counted, for good this shell, and the names
// to weigh queued.
function counted(state, answer) {
    if (answer.busy) {
        return state;
    }
    return { counted: true, owed: false, queue: toWeigh(answer.holders) };
}

// A weighing's answer, as freedBy() gives it: a busy one keeps the name to
// weigh again; else it leaves the queue, held or not.
function weighed(state, answer) {
    if (answer.busy) {
        return state;
    }
    return { counted: state.counted, owed: state.owed, queue: state.queue.slice(1) };
}
