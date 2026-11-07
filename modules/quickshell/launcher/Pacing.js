.pragma library

// When an open launcher next refreshes a source that said it changed. The
// first change is taken at once, and those after it within the pace
// together, since each refresh joins the whole corpus again and searches
// what's shown: 86 of Slack's answers, one a tenth of a second just after a
// restart, held the open launcher for a minute (2026-10-30, 19:14).
//
// A pace is when the last refresh began, `at`, and when a paced one is due,
// `due`, or null when none waits. The launcher's timer rings at `due`.

// A launcher that hasn't refreshed yet.
function rested() {
    return { at: 0, due: null };
}

// A source has changed at `now`: whether it's refreshed at once, and the
// pace after it, holding when the waiting refresh is due.
function heard(pace, now, every) {
    if (pace.due !== null) {
        return { pace, now: false };
    }
    const due = pace.at + every;
    if (due <= now) {
        return { pace, now: true };
    }
    return { pace: { at: pace.at, due }, now: false };
}

// A refresh began at `now`, however it was asked for.
function took(pace, now) {
    return { at: now, due: pace.due };
}

// The timer rang: the refresh it waited for is taken.
function rang(pace) {
    return { at: pace.at, due: null };
}
