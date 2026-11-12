// What ails the link to work, as Now's Work Network piece says it
// (pieces/WorkNetwork.qml), from the `warp:state` record the warp daemon
// tells (WarpState.qml, modules/warp/src/serve.rs) and in words.json's
// on:work-network words. Nothing ails it is an empty list, which leaves
// the piece quiet.
.pragma library

// The error warp-cli gives while WARP's service isn't running, the one
// it gives that says for certain the link is out.
const SERVICE_DOWN = "FailedToConnectToDaemon";

// How long a link may be connecting before it's said, since WARP takes a
// few seconds to come up each time, and that's how it should.
const CONNECTING_FOR = 30000;

// A template said, its holes filled from `fields`.
function filled(template, fields) {
    const hole = /\{(?<name>\w+)\}/gu;
    return String(template ?? "").replace(hole, (whole, name) => fields[name] ?? whole);
}

// Whether what ails it hangs on the time, so the piece looks again as it
// passes: only while it's connecting.
function timed(state) {
    return state?.said?.status === "Connecting";
}

// When the state began: as the daemon saw it begin, or else when it was
// first heard, which is the soonest it's known to have stood.
function began(state) {
    return state.since_ms ?? state.read_at_ms;
}

// What ails it, at `now`, with `clock` writing a time as Joel reads one.
// A link Joel took down himself (Manual, ManualDisconnection) is his
// choice, so says nothing; one that dropped otherwise says since when,
// where the daemon saw it drop. A link degraded says so, one up but
// slow, as the daemon judges a wait through it held for a minute, says
// what waits, and one connecting for longer than WARP takes to come up
// says since when.
// WARP's service down says it isn't answering; any other error warp-cli
// gives, or warp-cli missing, says nothing certain of the link, so the
// piece stays quiet rather than raise a false alarm.
function ails(state, said, now, clock) {
    if (state === null || state === undefined) {
        return [];
    }
    const stands = state.said ?? {};
    if (stands.error !== undefined) {
        return stands.error === SERVICE_DOWN ? [said.notAnswering] : [];
    }
    const reason = String(stands.reason ?? "");
    if (stands.status === "Connected") {
        if (reason === "NetworkDegraded") {
            return [said.degraded];
        }
        const waited = state.slow?.waiting_ms;
        return Number.isFinite(waited) ? [filled(said.slow, { waiting: `${waited} ms` })] : [];
    }
    if (stands.status === "Connecting") {
        const since = began(state);
        return now - since >= CONNECTING_FOR ? [filled(said.connecting, { time: clock(since) })]
            : [];
    }
    if (reason.startsWith("Manual")) {
        return [];
    }
    return state.since_ms === null || state.since_ms === undefined ? [said.down]
        : [filled(said.downSince, { time: clock(state.since_ms) })];
}
