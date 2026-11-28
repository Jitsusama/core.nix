.pragma library

// The widest verb the words file names, in any of its states, as the foot
// draws it after Return (Launcher.qml's widestVerb, which passes how wide
// the foot draws Return and a verb). "connect", which a network's source
// says, is the least the room holds.

// The least verb the room is kept for.
const FLOOR = "connect";

// Each verb the things name, in their own or a state's, once, in the order
// first named, after the floor. A verb that isn't a string names nothing
// the foot could draw, so it's left out.
function verbsOf(things) {
    const verbs = new Set([FLOOR]);
    for (const entry of things) {
        const states = Object.values(entry.states ?? {});
        for (const verb of [entry.verb].concat(states.map(state => state.verb))) {
            if (typeof verb === "string") {
                verbs.add(verb);
            }
        }
    }
    return verbs;
}

// The widest of the things' verbs by `measure`, never less than the floor,
// the earlier kept where two are as wide. Each distinct verb is measured
// once, and the widest kept with its width: about 1,803 things share a
// handful of verbs, and measuring every thing's verb against the widest so
// far, twice a verb, cost 6 to 8 ms of a first opening's 141 to 209 ms
// before its frame, when nothing was weighed yet (2026-11-27).
function among(things, measure) {
    let widest = FLOOR;
    let widestWidth = -Infinity;
    for (const verb of verbsOf(things)) {
        const width = measure(verb);
        if (width > widestWidth) {
            widest = verb;
            widestWidth = width;
        }
    }
    return widest;
}
