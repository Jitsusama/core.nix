// What each opening of the launcher took, kept so the homes Joel tries can
// be told apart by what they cost him rather than by what anyone guesses
// (Joel, 2026-10-25: "i have never seen what works best with my brain. i'm
// willing to experiment"). One trial is one opening: the home it opened on,
// the keys pressed by kind, what was picked, how deep and how long it took,
// and whether it closed with letters typed and nothing picked, which is a
// miss. Trials.qml keeps them; this only counts.
.pragma library

// The kinds of key counted: letters typed and erased (the left arrow
// clearing what's typed erases it too), arrows up and down and the page
// keys, steps in by the right arrow, steps back by the left, Tab, and
// Return.
const KINDS = ["letters", "erased", "arrows", "steps", "backs", "tabs", "returns"];

// An opening at `at` milliseconds, on the home called `home`.
function begun(at, home) {
    const keys = {};
    for (const kind of KINDS) {
        keys[kind] = 0;
    }
    return { at, home, keys, picked: "", kind: "", typed: "", depth: 0,
        ms: -1, open: -1, missed: false };
}

// The trial with one more key of this kind, or as it was for a kind not
// counted.
function counted(trial, kind) {
    if (!(trial && KINDS.includes(kind))) {
        return trial;
    }
    const keys = Object.assign({}, trial.keys);
    keys[kind] += 1;
    return Object.assign({}, trial, { keys });
}

// The trial after the field went from `was` to `now`: letters typed when it
// grew, erased when it shrank. Filling it from a row (Tab, a command's
// start) counts as the letters it put there, which Joel didn't type, but
// it's one key for many letters only where Tab was counted too.
function retyped(trial, was, now) {
    if (!trial) {
        return trial;
    }
    const grown = String(now).length - String(was).length;
    const keys = Object.assign({}, trial.keys);
    if (grown > 0) {
        keys.letters += grown;
    } else {
        keys.erased -= grown;
    }
    return Object.assign({}, trial, { keys });
}

// The trial with this picked at `at`, `where` saying what was typed for it
// and how many steps in it was: { typed, depth }. A later pick in the same
// opening, as a collection opened and then a thing in it, is the one kept.
function picked(trial, at, row, where) {
    if (!(trial && row)) {
        return trial;
    }
    return Object.assign({}, trial, { picked: String(row.key ?? ""), kind: String(row.kind ?? ""),
        typed: String(where?.typed ?? ""), depth: where?.depth ?? 0, ms: at - trial.at });
}

// The trial as it closed at `at`: how long it was open, and a miss if
// letters were typed and nothing was picked.
function ended(trial, at) {
    if (!trial) {
        return trial;
    }
    return Object.assign({}, trial, { open: at - trial.at,
        missed: trial.picked === "" && trial.keys.letters > 0 });
}

// The trials kept, this one last, the oldest let go past `most`.
function kept(trials, trial, most) {
    const all = (Array.isArray(trials) ? trials : []).concat(trial ? [trial] : []);
    return all.length > most ? all.slice(all.length - most) : all;
}
