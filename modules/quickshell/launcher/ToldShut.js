.pragma library

// What waits to be said in the launcher's foot as it next opens
// (Launcher.qml's tellOnOpen): each tell whole, its words [mark, word,
// colour] as the foot's keys are, beside what it's about, when it was told
// and whether it may be cut short (say's `loose`).
//
// A word told while the launcher is shut answers a press made before it
// shut, and may come long after: a viewer come after all is told up to a
// quarter hour on (Sessions.qml's comingPatience). Kept as one list that
// each word replaced whole, a late word about one press took the place
// of a newer press's about another, and a word nothing opened the
// launcher for met whatever Joel opened it for next, hours on (SEYNT2,
// 2026-11-26).
//
// A tell is kept whole and given way by what it's about, never by its
// words' marks, since a tell's words aren't each about something: an act
// refused says its why and then the failed mark every refusal shares
// (Becoming.js's told), and a walk's tallies mark each count by the count
// itself (ReopenTally.js). Given way by mark, a second refusal took the
// first's what and left its why beside the second's (#7116's review).

// A word waits this long for the launcher to open, then lapses, so what
// an act came to is said beside the press it answers rather than hours on.
const LAPSES_MS = 15 * 60 * 1000;

// What a tell is about where its teller doesn't say: the mark its words
// open with, the session a session's word names or a start's own name.
function aboutOf(told) {
    return told.length > 0 ? String(told[0][0]) : "";
}

// What waits once `tell` ({told, about, at, loose}) is told: a tell waiting
// about the same gives way to the newer, and the rest stay, older first,
// as they were told.
function waiting(kept, tell) {
    return kept.filter(one => one.about !== tell.about).concat([tell]);
}

// The words of `tells`, in the order they were told, as the foot says them.
function said(tells) {
    return [].concat(...tells.map(one => one.told));
}

// What's said as the launcher opens at `now`: the tells told no more than
// LAPSES_MS before and their words, loose only where every one of them
// is, and how many words lapsed, which the journal counts without them.
function due(kept, now) {
    const fresh = kept.filter(one => now - one.at <= LAPSES_MS);
    const stale = kept.filter(one => now - one.at > LAPSES_MS);
    return {
        tells: fresh,
        said: said(fresh),
        loose: fresh.length > 0 && fresh.every(one => one.loose),
        lapsed: said(stale).length
    };
}

// What's left of `kept` once the tell whose words are `shown` is taken
// back, by its identity, as a start takes back what it told (Agents.qml's
// footCleared).
function without(kept, shown) {
    return kept.filter(one => one.told !== shown);
}
