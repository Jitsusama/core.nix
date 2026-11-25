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

// `pressed` (by each act's key, the last time it was pressed or given
// fresh words) with act `key` stamped at `now`.
function pressedAt(pressed, key, now) {
    return Object.assign({}, pressed, { [key]: now });
}

// Whether a start answered at `since` may still say over its act `key`:
// not once that act was pressed again, or given fresh words, since then
// (`pressed`, as pressedAt keeps it).
function saysOnAct(pressed, key, since) {
    return (pressed[key] ?? 0) <= since;
}

// The openings still `waiting` (Sprouting.settled's), each of the `late`
// ones carrying as `told` the words `texts` said of it, in late's order, so
// an opening takes back from the foot only what was said of itself.
function toldOf(waiting, late, texts) {
    return waiting.map(one => {
        const at = late.findIndex(was => was.held === one.held && was.machine === one.machine);
        return at < 0 ? one : Object.assign({}, one, { told: texts[at] });
    });
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

// What the foot told once opening `one` takes back the words said of it
// late (its `told`, toldOf's), as its window opens or its up word is
// told: `footed` what's left, and `cleared` the foot it was shown in where
// nothing is left, since sproutShut finds no foot then to clear; else null.
// One never said late has no `told`, which no tell reads, so it takes back
// nothing.
function takenBack(footed, one) {
    const left = footedWithout(footed, one.told);
    const was = footed ?? null;
    return { footed: left, cleared: was !== null && left === null ? was.shown : null };
}
