// What the launcher's Dictation row says in its drawer (Dictated.qml),
// worked out here so Qt's own engine holds it (tests/speaking.qml).
.pragma library

// The drawer's facts, each { id, value }, in the order VKCF3Z set: how,
// what a terminal gets, when it was last used, how fast the model loaded,
// then how to stop. One with nothing known yet, before the first take or
// the first load, is left out rather than said as a dash.
function facts(stood, told) {
    const out = [
        { id: "how", value: told.how ?? "" },
        { id: "terminal", value: told.terminal ?? "" }
    ];
    if (stood.lastTake > 0) {
        out.push({ id: "last", value: stood.lastSaid });
    }
    if (stood.readyMs >= 0) {
        out.push({ id: "ready", value: seconds(stood.readyMs) });
    }
    out.push({ id: "stop", value: told.stop ?? "" });
    return out;
}

const MS_A_SECOND = 1000;

// A load's time in seconds, to a tenth, its figure and unit kept together.
function seconds(ms) {
    return `${(ms / MS_A_SECOND).toFixed(1)}\u00a0s`;
}
