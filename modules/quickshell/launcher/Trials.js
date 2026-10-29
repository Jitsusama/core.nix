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
// A key slower than this to its frame is felt: a frame at 120 Hz.
const FELT_MS = 8;
// How many of an opening's felt keys are kept, each with what took the time.
const FELT_KEPT = 20;

// An opening at `at` milliseconds, on the home called `home`.
function begun(at, home) {
    const keys = {};
    for (const kind of KINDS) {
        keys[kind] = 0;
    }
    return { at, home, keys, picked: "", kind: "", typed: "", depth: 0,
        ms: -1, open: -1, missed: false,
        timed: { count: 0, searched: 0, searchedMost: 0, drawn: 0, drawnMost: 0, felt: [] } };
}

// Whether a key that took `drawn` milliseconds to its frame was one Joel
// could feel.
function felt(drawn) {
    return drawn > FELT_MS;
}

// The trial with one more key timed: how long its search took and how long
// until the frame that drew it, both in milliseconds, and `parts` what the
// search spent its time on ({ lists, rows, answers }, the rows' time split
// `within` matching, ordering, making and spots, with how many choices were
// looked at and found) or null. Each felt key
// is kept with its parts, the first few of an opening, so what's slow is
// named by Joel's own use rather than guessed (Joel, 2026-10-25: "i felt a
// bit of lag btw with the launcher").
function timed(trial, searched, drawn, parts) {
    if (!trial) {
        return trial;
    }
    const was = trial.timed ?? begun(0, "").timed;
    const keptFelt = felt(drawn) && was.felt.length < FELT_KEPT
        ? was.felt.concat([{ searched, drawn, parts: parts ?? null }]) : was.felt;
    return Object.assign({}, trial, { timed: {
        count: was.count + 1,
        searched: was.searched + searched,
        searchedMost: Math.max(was.searchedMost, searched),
        drawn: was.drawn + drawn,
        drawnMost: Math.max(was.drawnMost, drawn),
        felt: keptFelt
    } });
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
