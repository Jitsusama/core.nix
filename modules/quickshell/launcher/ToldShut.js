.pragma library

// What waits to be said in the launcher's foot as it next opens
// (Launcher.qml's tellOnOpen): each word as it was told, [mark, word,
// colour] as the foot's keys are, beside when it was told and whether
// it may be cut short (say's `loose`).
//
// A word told while the launcher is shut answers a press made before it
// shut, and may come long after: a viewer come after all is told up to a
// quarter hour on (Sessions.qml's comingPatience). Kept as one list that
// each word replaced whole, a late word about one press took the place
// of a newer press's about another, and a word nothing opened the
// launcher for met whatever Joel opened it for next, hours on (SEYNT2,
// 2026-11-26).

// A word waits this long for the launcher to open, then lapses, so what
// an act came to is said beside the press it answers rather than hours on.
const LAPSES_MS = 15 * 60 * 1000;

// What waits once `told` is told at `at`: each word already waiting about
// what a word of `told` names (its mark, the session or act it's about)
// gives way to the newer, and the rest stay, older first, as they were
// told. A list of told words with one mark, as a start's told words are,
// replaces each word waiting under that mark.
function waiting(kept, told, at, loose) {
    const marks = told.map(one => one[0]);
    return kept.filter(one => !marks.includes(one.said[0]))
        .concat(told.map(said => ({ said, at, loose: loose === true })));
}

// What's said as the launcher opens at `now`: the words told no more than
// LAPSES_MS before, loose only where every one of them is, and how many
// lapsed, which the journal counts without their words.
function due(kept, now) {
    const fresh = kept.filter(one => now - one.at <= LAPSES_MS);
    return {
        said: fresh.map(one => one.said),
        loose: fresh.length > 0 && fresh.every(one => one.loose),
        lapsed: kept.length - fresh.length
    };
}

// What waits once the words `shown` are taken back, each by its identity,
// as a start takes back what it told (Agents.qml's footCleared).
function without(kept, shown) {
    return kept.filter(one => !shown.includes(one.said));
}
