// When the daemon line being heard now was read, shared by every Daemon
// and the launcher, so a change a line makes can be timed from it to the
// frame niri presents with it (Y2YBTS, 2026-11-11: Joel's 50 ms bar holds
// for updates as for keys). Daemon.qml marks it from before it parses a
// line until every handler that line called has returned, a watch's whole
// answer aside, which tells no change but what was missed, so a provider
// telling its change from its handler is timed from the line, and one that
// tells it later finds no mark and isn't timed from a line it didn't come
// from, unless it carries its line there (within, carried). Each line is
// marked with the daemon it came from too, `by`, so a place made from
// many, as Now is, says whose lines its changes came from. A library, so
// one is shared by everything that imports it.
.pragma library

const hearing = { at: 0, by: "" };

// A line read at `at`, on Date's clock, since Daemon.qml reaches no other,
// from the daemon named `by`; what was marked before it, put back once
// it's heard.
function begun(at, by) {
    const was = { at: hearing.at, by: hearing.by };
    hearing.at = at;
    hearing.by = at > 0 ? by ?? "" : "";
    return was;
}

function ended(was) {
    hearing.at = was.at;
    hearing.by = was.by;
}

// When the line being heard was read, or 0 while none is.
function heardAt() {
    return hearing.at;
}

// The daemon the line being heard came from, or "" while none is.
function heardBy() {
    return hearing.by;
}

// Runs `tell` as though the line read at `at` were being heard, so a change
// told later than its line, as an ask's answer or a tell put off to the
// turn's end, is timed from the line that set it going rather than left
// out (Y2YBTS, 2026-11-11). With no line, 0, what's heard now stands: an
// answer to an ask nothing heard set going is timed from its own line.
function within(at, tell, by) {
    const was = at > 0 ? begun(at, by) : begun(hearing.at, hearing.by);
    try {
        return tell();
    } finally {
        ended(was);
    }
}

// A tell put off and run once however many ask for it, as Qt.callLater runs
// one function once a turn and a Timer started once runs once: `tell` is
// what's run, given as `kept.tell`, the same function each time, so
// Qt.callLater still runs it once. It's timed from the earliest line it
// was put off from since it last ran, the change that waited longest.
function carried(tell) {
    const kept = { at: 0, by: "", tell: null };
    kept.tell = (...given) => {
        const from = kept.at;
        const by = kept.by;
        kept.at = 0;
        kept.by = "";
        return within(from, () => tell(...given), by);
    };
    return kept;
}

// Puts `kept`'s tell off from the line heard now, and gives what to run
// later: `Qt.callLater(Hearing.putOff(kept))`.
function putOff(kept) {
    if (kept.at === 0) {
        kept.at = hearing.at;
        kept.by = hearing.by;
    }
    return kept.tell;
}
