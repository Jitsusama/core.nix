.pragma library

// What Start New Work says of a start waiting for its session to come up
// (Agents.qml's openings), apart from Sprouting.js, which holds the start
// itself. A start on a loaded machine can take minutes to come up (Joel's
// at 14:30 on 25 November took five and a half), so its late words are said
// long after his Return, when the act may show another start, or the foot
// another start's end.

// The names a word of Start New Work's may fill.
const FILLED = /\{(machine|title|minutes)\}/gu;

// `word` of Start New Work's words, `said`, with each name FILLED takes
// filled from `fills` by a function, so a title's `$&` is only text, and a
// name `fills` doesn't give filled with nothing.
function filled(said, word, fills) {
    return String(said?.[word] ?? "").replace(FILLED, (_, name) => String(fills[name] ?? ""));
}

// Whether a start answered at `since` may still say over its act `key`:
// not once that act was pressed again, or given fresh words, since then
// (`pressed`, by each act's key, the last time either was).
function saysOnAct(pressed, key, since) {
    return (pressed[key] ?? 0) <= since;
}

// What the foot told (`footed`, Agents.qml's sproutFooted, `{ told, ... }`)
// without the first tell reading `text`, as a start's window opening
// answers what was said of it, while each other start's tell stays to be
// noticed as the launcher shuts; none when nothing is left.
function footedWithout(footed, text) {
    if (footed === null || footed === undefined) {
        return null;
    }
    const at = footed.told.indexOf(text);
    if (at < 0) {
        return footed;
    }
    const told = footed.told.filter((_, index) => index !== at);
    return told.length === 0 ? null : Object.assign({}, footed, { told });
}
