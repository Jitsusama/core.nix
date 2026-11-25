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

// When `one`'s Return was, for what's pressed on its act after it: its
// `asked`, or for one with none (a start the node kept, heard later) its
// `since`.
function askedOf(one) {
    return one.asked ?? one.since;
}

// Whether `one`, up, opens and takes the keys: only where Joel `waits` on
// its act (Sprouting.waitsOn) and its act wasn't pressed or given fresh
// words since its Return (`pressed`, saysOnAct). A link offer comes only
// with the launcher shut, which spends every opening's keys (shutOn), so
// this holds against a press waitsOn can't see.
function takesKeys(waits, pressed, one) {
    return waits && saysOnAct(pressed, one.key, askedOf(one));
}

// The opening a start `how` on the act `key` waits on, answered `now`,
// from `given`, what its Return recorded (`at` its stamp, `keys` and
// `letters` the launcher's keysTaken and lettersAt): what's pressed or
// typed counts from the Return, and the minute's patience from `now`, so
// a slow answer isn't said to be still coming up seconds after it.
function opening(how, key, given, now) {
    return { key, held: how.held, machine: how.machine, since: now, asked: given.at,
        keys: given.keys, letters: given.letters };
}

// `given` (an opening, or Agents.qml's sproutGiven) once the launcher
// shuts: it takes the keys no more, as a reopening needn't move a key and
// Joel has left the act's wait.
function spent(given) {
    return Object.assign({}, given, { keys: -1 });
}

// The `openings` once the launcher shuts, each spent.
function shutOn(openings) {
    return openings.map(spent);
}

// The word said of `one` come up and left for Go To: "upLater", naming
// it, where it was said late to be still coming up, so Joel knows which
// start it was; else "up".
function upWord(one) {
    return one.late === true ? "upLater" : "up";
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
