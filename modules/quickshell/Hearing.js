// When the daemon line being heard now was read, shared by every Daemon
// and the launcher, so a change a line makes can be timed from it to the
// frame niri presents with it (Y2YBTS, 2026-11-11: Joel's 50 ms bar holds
// for updates as for keys). Daemon.qml marks it from before it parses a
// line until every handler that line called has returned, so a provider
// telling its change from its handler is timed from the line, and one that
// tells it later finds no mark and isn't timed from a line it didn't come
// from. A library, so one is shared by everything that imports it.
.pragma library

const hearing = { at: 0 };

// A line read at `at`, on Date's clock, since Daemon.qml reaches no other;
// what was marked before it, put back once it's heard.
function begun(at) {
    const was = hearing.at;
    hearing.at = at;
    return was;
}

function ended(was) {
    hearing.at = was;
}

// When the line being heard was read, or 0 while none is.
function heardAt() {
    return hearing.at;
}
