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

// A refresh began at `now`, however it was asked for, so it takes the one
// that waited with it: a key's own change or a daemon's answer refreshed
// at once, or the timer rang late and another change past the pace came
// beside it, and a refresh left due would follow within the pace.
function took(now) {
    return { at: now, due: null };
}

// Whether a change heard `ago` ms after the last key is that key's own,
// refreshed at once on its frame: only after a key that acted, within
// `within`. A key that only `moved` changed nothing, so a change beside it,
// as Now's each second, waits on the pace like any other (Joel,
// 2026-11-08: "weird lag can happen").
function keyed(ago, moved, within) {
    return !moved && ago < within;
}

// How long after the last key's work, taken at `taken` and done at `done`,
// a change came at `now`: none while the work is under way, as the act it
// does changes a source from within it. Counted from the key's press
// instead, a handler a loaded machine slowed past `within` before its act
// left the change it made to the pace, and dd on a notice never said it was
// shown (draws-still at 550a45788, cd92d7660 and 2101c92b6, 2026-11-09).
function sinceKey(now, taken, done) {
    return done < taken ? 0 : now - done;
}

// The timer rang: the refresh it waited for is taken.
function rang(pace) {
    return { at: pace.at, due: null };
}

// A key's acts on their way to the screen, each shown by the first frame
// begun after its refresh: `waiting`, refreshed with no frame begun since,
// and `framing`, in a frame begun after it, so a swap of one already under
// way when an act's refresh ran isn't taken for it.
function unframed() {
    return { waiting: [], framing: [] };
}

// The acts whose refresh has just run wait for the next frame.
function refreshedActs(acts, refreshed) {
    return { waiting: acts.waiting.concat(refreshed), framing: acts.framing };
}

// A frame began, so it draws every act that waited.
function frameBegun(acts) {
    return { waiting: [], framing: acts.framing.concat(acts.waiting) };
}

// A frame was swapped onto the screen: the acts it drew are `shown`, and
// those refreshed while it was drawn wait on for the next. Held back with
// them, as one flag for every act once held them, dd on a notice was never
// said shown: a source no row showed changed just after it, as its own
// refresh's frame began, was taken for the key's, and its refresh drew no
// frame to follow (draws-still at 7b2252da0, 2026-11-10).
function frameSwapped(acts) {
    return { shown: acts.framing, acts: { waiting: acts.waiting, framing: [] } };
}

// What an opening's own work went on, from each part's milliseconds by its
// name, said slowest first, since the slowest is what its line is read for.
// A part under a millisecond is left out, and an opening with none left
// says so.
function openingParts(parts) {
    const said = Object.keys(parts).map(name => ({ name, ms: Math.round(parts[name]) }))
        .filter(part => part.ms >= 1)
        .sort((a, b) => b.ms - a.ms || a.name.localeCompare(b.name));
    if (said.length === 0) {
        return "none over a millisecond";
    }
    return said.map(part => `${part.name} ${part.ms}`).join(", ");
}
