.pragma library
.import "Pacing.js" as Pacing
.import "Trials.js" as Trials

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

// What a held key whose bound ran out leaves late, so each source still to
// answer says how late it came: what was typed, those sources and when
// they were asked. Null where the launcher closed first, since closing
// doesn't stop the wait and nobody waits on answers once it's shut, so
// none is late and no key was listed without them (Launcher.qml).
function lateOf(waited, open) {
    return open ? { text: waited.text, sources: waited.sources, began: waited.began } : null;
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
// launcher noted while it was held (Launcher.qml's heldSpans), as
// `listed`, `{ at, spans, collected }`, says them as it's listed: each span
// `{ name, from, to }` by the shell's own clock, as `held.from` and `at`,
// when the wait ended, are; most first, then what none named, as an
// opening's stall is (Pacing.stallSpent). Joel's fastest letters spent
// 730 of their 1,054 ms past 50 behind the shell's other work, which said
// nothing of what (RTOIMQ, 2026-11-17). The collector's time, from
// `held.collected` to `collected`, is said after as the whole wait's,
// since the count is read only as the key is held and listed, and it runs
// inside whatever was running; left out where it took none. Ended on its
// bound, or before, it names nothing.
function behindOn(held, listed, bound) {
    const { at, spans, collected } = listed;
    const past = held.from + bound;
    if (!(at > past)) {
        return "";
    }
    const collecting = Math.round(collected - held.collected);
    return [Pacing.stallSpent(spans, past, at),
        collecting > 0 ? `collecting ${collecting} over the whole wait` : ""]
        .filter(part => part !== "").join("; ");
}

// A held key's wait as its line says it, once it's listed (behindOn's
// `listed`): how long, how far past its bound, and what that was behind,
// all from `held.from` by the shell's clock, so the time behind and what
// it names add up. Timed from `held.began`, the wall clock's, one said
// 2 ms behind and named 1 (the keys lab, 2026-11-18).
function heldFor(held, listed, bound) {
    const timedOut = held.timedOut === true;
    return Object.assign(waitedOf({ began: held.from }, listed.at, bound, timedOut),
        { behindOn: timedOut ? behindOn(held, listed, bound) : "" });
}

// The frame a key held, or an opening felt, goes on noting: the one in
// flight where a hold or an opening was already `listening`, as a key
// typed over a held one is, since a frame begun then was being noted;
// else a fresh one, as a frame begun while nothing listened would be
// noted from then.
function frameHeld(frame, listening) {
    return listening ? frame : Pacing.feltUnframed();
}

// What the frame being drawn has run that Pacing.feltFramed hasn't said
// yet, as of `at`: rows laid out for a frame not yet animating, and a sync
// not yet heard to end, said up to `at`, the bound's timer firing, the
// first the shell's thread is heard after it. A key's bound can end on the
// timer queued behind a sync, ahead of the call that notes it, so the work
// it waited behind would otherwise be said as unnamed (a council,
// 2026-11-18).
function frameOpen(frame, at) {
    const open = frame.syncFrom < 0 ? [] : [{ name: "syncing", from: frame.syncFrom, to: at }];
    if (frame.rows !== null) {
        open.push({ name: "laying out", from: frame.rows.from, to: frame.rows.to });
    }
    return open;
}

// What a held key's frame still had open when its wait ended: said up to
// the bound's timer firing, where the bound ended it, since the timer's
// own work after that isn't the sync; else up to the listing `at`.
function heldOpen(frame, held, at) {
    return frameOpen(frame, held.firedAt ?? at);
}

// The render thread's steps of each frame in `frames`, as
// FramePresentation's timesOf says them with the frame's number, that
// held the shell's thread from `from` to `to`: its render, then its swap,
// which waits on the compositor, and can wait on the GPU drawing what the
// render handed it.
function drawingHeld(frames, from, to) {
    const steps = [];
    for (const frame of frames) {
        steps.push(
            { name: "waiting on the last render", from: frame.rendering, to: frame.rendered },
            { name: "waiting on the last swap", from: frame.rendered, to: frame.swapped });
    }
    return steps.filter(step => step.from > 0 && step.to > step.from)
        .map(step => ({ name: step.name, from: Math.max(step.from, from),
            to: Math.min(step.to, to) }))
        .filter(step => step.to > step.from)
        .sort((one, other) => one.from - other.from);
}

// One span the shell's thread was said syncing, told apart by the render
// thread's times. Qt's threaded loop polishes before it says animating,
// then posts the sync and waits (qsgthreadedrenderloop.cpp's
// polishAndSync), so until the render thread takes the sync the shell's
// thread waits on it: on a frame before's render or swap where that ran,
// else before the sync, which holds both the render thread beginning the
// frame, where it can wait for a buffer (syncAndRender's beginFrame), and
// the shell's own handlers of animating run after the span opened, and so
// names neither's owner. Those handlers can run while a frame before
// still renders or swaps, so those waits are at most what it held. Then
// the sync itself, then what the shell's thread ran after it let go (its
// animations advanced, and what was queued ahead of the call ending the
// span); on a loop rendering on the shell's thread, as in a sandbox, that
// also holds the frame's own render and swap. Where no frame's sync began
// within it, as one whose window wasn't ready to render, whose slot a
// newer frame took, or one synced longer before the key's frame than the
// ring's sixteen, what no step names is said as untimed syncing. Its own
// step isn't said as syncing, which names the whole span where it isn't
// told apart, as an opening's stall does (Pacing.stallSpent).
function syncSplit(span, frames) {
    const own = frames.filter(frame => frame.synchronizing >= span.from
        && frame.synchronizing <= span.to)
        .sort((one, other) => one.synchronizing - other.synchronizing)[0];
    const took = own === undefined ? span.to : own.synchronizing;
    const rest = own === undefined ? "untimed syncing" : "before the sync";
    const parts = [];
    let at = span.from;
    for (const step of drawingHeld(frames.filter(frame => frame !== own), span.from, took)) {
        if (step.from > at) {
            parts.push({ name: rest, from: at, to: step.from });
        }
        // A step wholly inside one already said is passed over, so no
        // millisecond is said twice.
        if (step.to > at) {
            parts.push({ name: step.name, from: Math.max(step.from, at), to: step.to });
            at = step.to;
        }
    }
    if (took > at) {
        parts.push({ name: rest, from: at, to: took });
    }
    if (own === undefined) {
        return parts;
    }
    // The patch sets both sync times on the render thread at once, so a
    // sync with no end comes only from a read gone wrong; it's said as
    // taking none, so the time after isn't put back under the sync.
    const synced = own.synchronized > took ? Math.min(own.synchronized, span.to) : took;
    if (synced > took) {
        parts.push({ name: "the sync itself", from: took, to: synced });
    }
    if (span.to > synced) {
        parts.push({ name: "after the sync", from: synced, to: span.to });
    }
    return parts;
}

// Each span said syncing told apart (syncSplit), the rest as they were.
// The shell's thread is said syncing from its frame's animating to the
// sync letting it go, so "sta" said syncing 68 where its own frame's sync
// took 1 by the render thread (Joel, 2026-11-18).
function syncsSplit(spans, frames) {
    const split = [];
    for (const span of spans) {
        split.push(...(span.name === "syncing" ? syncSplit(span, frames) : [span]));
    }
    return split;
}

// What a held key waited behind (behindOn), with its syncs told apart by
// the render thread's times of the frames around it, read as its line is
// said, once the frames synced while it was held have rendered.
function behindSplit(behind, frames, bound) {
    return behindOn(behind.held, Object.assign({}, behind.listed,
        { spans: syncsSplit(behind.listed.spans, frames) }), bound);
}

// The frames FramePresentation still has the times of, oldest first, each
// with its number, read back from `last`, the newest begun. The newest
// may be begun and not yet rendered, so has no times, and is passed over;
// its slot then still holds the frame the ring's size before it, so the
// ring's frames are counted from the newest rendered. The read ends at the
// first a newer frame's slot took, as Trials.begunBefore's does, or at the
// ring's size, should timesOf ever give a slot's times for a frame not its
// own.
function framesTimed(last, timesOf) {
    const frames = [];
    const newest = last > 0 && timesOf(last).rendered > 0 ? last : last - 1;
    for (let frame = newest; frame > 0 && newest - frame < Trials.FRAMES_KEPT; frame--) {
        const times = timesOf(frame);
        if (!(times.rendered > 0)) {
            break;
        }
        frames.unshift(Object.assign({ frame }, times));
    }
    return frames;
}

// How many held keys' waits are kept for their frames' swaps: more than a
// burst types before the first's frame is swapped, which took 57 ms behind
// a compositor's swap (Joel's "sta", 2026-11-18).
const HELD_KEPT = 8;

// The held keys' waits kept until each key's frame is swapped, with one
// more, `behind` ({ held, listed }, as behindSplit takes it) for the key
// whose search made `parts`; the oldest goes past HELD_KEPT. Each is kept
// by its key's parts, so a key typed before an earlier one's frame was
// swapped doesn't take its place.
function heldKept(kept, parts, behind) {
    return kept.slice(-(HELD_KEPT - 1)).concat([Object.assign({ parts }, behind)]);
}

// The wait kept for the key whose search made `parts`, as its frame is
// swapped, or null, and what's kept without it.
function heldDrawn(kept, parts) {
    const behind = kept.find(one => one.parts === parts) ?? null;
    return { behind, kept: kept.filter(one => one !== behind) };
}
