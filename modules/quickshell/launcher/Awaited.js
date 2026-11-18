.pragma library
.import "Pacing.js" as Pacing

// What a search of what's typed does about the sources whose answers it
// waits for (Launcher.qml's searchNow). A source such as the notices,
// found by their daemon, answers a moment after the key, and a key's list
// waits for it a little so its rows land in one frame (Y2YBTS,
// 2026-11-11). Typing holds such a key and drops one held before it, whose
// search is never made, so no stale search runs. Every other key and
// setter that reads what was found searches the held key at once, without
// the answers not come, so Return acts on what the text finds rather than
// waiting for them or acting on the rows of the letters before.
//
// `now` is the launcher as the search comes: `due`, the keys not yet
// searched, which time it; `held`, the key held for answers, { due, ... },
// or null; `typing`, whether typing asked; and `asking`, whether a question
// is open, which waits on no answer. `waitingOn()` asks the sources whose
// search awaits an answer and gives them, called only when one could be
// held.
//
// Answers null when there's nothing to search; { hold: sources, due } to
// hold the key for them; or { due, waited } to search now, `waited` the
// held key whose wait this ends, or null where typing went over it.
// What the field read before the key now coming, so a trial counts only the
// letters it brought: the text the last key not yet searched left there,
// `due`'s or the held key's, or else `searched`, what was last searched.
function before(now, searched) {
    const keys = (now.held?.due ?? []).concat(now.due);
    return keys.length > 0 ? keys[keys.length - 1].text : searched;
}

function next(now, waitingOn) {
    if (now.due.length === 0 && (now.typing || now.held === null)) {
        return null;
    }
    const timed = (now.held?.due ?? []).concat(now.due);
    if (now.typing && !now.asking) {
        const sources = waitingOn();
        if (sources.length > 0) {
            return { hold: sources, due: timed };
        }
    }
    return { due: timed, waited: now.typing ? null : now.held };
}

// How long a held key waited for its answers, from when they were asked
// (`held.began`) to `now`, and how much of that came past its `bound`
// when the bound's timer ended it (`timedOut`): a timer fires only once
// the shell's thread is free, so time past the bound is time the key
// waited behind the shell's other work, such as frames being drawn,
// rather than on any answer. An answer ending the wait is behind nothing.
function waitedOf(held, now, bound, timedOut) {
    const waited = Math.max(0, Math.round(now - held.began));
    return { waited, behind: timedOut ? Math.max(0, waited - bound) : 0 };
}

// What a held key waited behind once its bound was out, from the work the
// launcher noted while it was held (Launcher.qml's heldSpans), each
// `{ name, from, to }` by the shell's own clock, as `held.from` and `at`,
// when the wait ended, are: most first, then what none named, as an
// opening's stall is (Pacing.stallSpent). Joel's fastest letters spent
// 730 of their 1,054 ms past 50 behind the shell's other work, which said
// nothing of what (RTOIMQ, 2026-11-17). The collector's time, from
// `held.collected` to `collected`, is said after as the whole wait's,
// since the count is read only as the key is held and listed, and it runs
// inside whatever was running; left out where it took none. Ended on its
// bound, or before, it names nothing.
function behindOn(held, at, bound, spans, collected) {
    const past = held.from + bound;
    if (!(at > past)) {
        return "";
    }
    const collecting = Math.round(collected - held.collected);
    return [Pacing.stallSpent(spans, past, at),
        collecting > 0 ? `collecting ${collecting} over the whole wait` : ""]
        .filter(part => part !== "").join("; ");
}

// A held key's wait as its line says it, once it's listed at `at`: how
// long, how far past its bound, and what that was behind, all from
// `held.from` by the shell's clock, so the time behind and what it names
// add up. Timed from `held.began`, the wall clock's, one said 2 ms behind
// and named 1 (the keys lab, 2026-11-18).
function heldFor(held, at, bound, spans, collected) {
    const timedOut = held.timedOut === true;
    return Object.assign(waitedOf({ began: held.from }, at, bound, timedOut),
        { behindOn: timedOut ? behindOn(held, at, bound, spans, collected) : "" });
}
