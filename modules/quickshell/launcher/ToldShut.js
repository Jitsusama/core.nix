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

// The tell kept for the words `told`, told at `at`: about `about` where its
// teller says, else what aboutOf reads, and loose only where it's said to
// be, as Launcher.qml's tellOnOpen keeps it.
function tell(told, loose, about, at) {
    return { told, about: about ?? aboutOf(told), at, loose: loose === true };
}

// What waits once `told` ({told, about, at, loose}) is told: a tell waiting
// about the same gives way to the newer, and the rest stay, older first,
// as they were told.
function waiting(kept, told) {
    return kept.filter(one => one.about !== told.about).concat([told]);
}

// A tell's words as the foot draws them, Return's end last: the foot moves
// the first word it's given to the far end (Launcher.qml's foot.shown), and
// each tell's words are ordered for that, as a refusal's why comes first
// to stand at the end beside its what (Becoming.js's told).
function drawn(words) {
    return words.length === 0 ? [] : words.slice(1).concat([words[0]]);
}

// The words of `tells`, in the order they were told, as the foot is given
// them: ordered so its one move of the first word to the far end draws each
// tell whole beside itself. Joined end to end, only the first tell's first
// word moved, and every later refusal's why stood beside the one before's
// what, whether they fitted or not (#7116's third stack round, 35 and 36).
function said(tells) {
    const whole = tells.filter(one => one.told.length > 0);
    if (whole.length === 0) {
        return [];
    }
    const newest = whole.slice(-1)[0].told;
    return [newest[0]].concat(...whole.slice(0, -1).map(one => drawn(one.told)),
        newest.slice(1));
}

// Whether the foot's words `told` are the words `words` an opening said:
// the same list, or one holding the same words in the same order, as a
// list handed through a Binding's value may not keep its identity
// (#7116's third stack round, 37).
function sameWords(told, words) {
    return told === words || (Array.isArray(words) && told.length === words.length
        && JSON.stringify(told) === JSON.stringify(words));
}

// The words of `tells` that `fits` lets the foot say, a whole tell giving
// way at a time, the newest first, so the oldest keeps the foot's first
// word. The foot gives way a word at a time (Standing.keysFitted), which
// knows nothing of where one tell ends: left to it, a narrow foot dropped
// a refusal's what and left its why beside the next refusal's (#7116's
// second stack round, 34). The one tell left is cut as any words are.
function fitting(tells, fits) {
    let kept = tells;
    while (kept.length > 1 && !fits(said(kept))) {
        kept = kept.slice(0, -1);
    }
    return said(kept);
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

// What the foot does once the tell whose words are `shown` is taken back,
// given what the opening said, `state` ({tells, said}), the foot's words
// `saying`, and whether the foot `showing` them now: `foot` is null to
// leave it, or the words to say in its place, under the keys it said its
// last under, so a word taken back never says the rest again after a key
// let them go (#7116's stack round, 19). Words no longer shown are cleared,
// so an opening never says them again.
function untold(state, saying, shown, showing) {
    if (saying === shown) {
        return { state, foot: [] };
    }
    const left = state.said === saying ? without(state.tells, shown) : state.tells;
    if (left.length === state.tells.length) {
        return { state, foot: null };
    }
    if (!showing) {
        return { state: { tells: [], said: null }, foot: [] };
    }
    const words = said(left);
    return { state: { tells: left, said: words }, foot: words,
        loose: left.length > 0 && left.every(one => one.loose) };
}

// Whether the foot, saying `saying`, says the tell whose words are `told`
// among what the opening said, `state`, as an offer's foot is said beside
// the tells that waited with it (#7116's stack round, 20 and 21).
function among(state, saying, told) {
    return saying === state.said && state.tells.some(one => one.told === told);
}
