.pragma library
.import "Sprouting.js" as Sprouting

// Starts Stopped, the row Agents' top level shows first while Joel has
// stopped agents starting sessions (modules/agents/src/starts_stopped.rs):
// how its Return goes, and who may lift the stop, held here so
// modules/quickshell/tests/starts-stopped.qml reaches them in Qt's own
// engine.

// Whether the row's Return steps in among its acts rather than doing the
// first: the daemon serves a look at its drawer first, so Return lifts no
// stop Joel meant, and a launcher with no drawer to step into drops that
// step (ReadyRows.qml's actsOf), which would leave Let Them Start first.
// Any other row's Return is as made.
function stepsIn(row, launcher) {
    return row?.kind === "starts-stopped"
        && (row.acts ?? [])[0]?.step === "drawer"
        && typeof launcher?.intoDrawer !== "function";
}

// Whether `one` is Let Them Start, the act that lifts the stop.
function lifts(one) {
    return one?.key === "verb:agents:starts:let" && typeof one.ask === "object"
        && one.ask !== null;
}

// Whether Let Them Start pressed now may ask: only Joel's own Return in
// the launcher, never a walk from outside (`launcher do`, `qs ipc call
// launcher run`), any program's an agent's among them, since both come
// on the shell's connection and the daemon takes the act from the shell
// alone. It fails closed, as a sprout does (Sprouting.fromJoel).
function mayLift(launcher) {
    return Sprouting.fromJoel(launcher);
}
