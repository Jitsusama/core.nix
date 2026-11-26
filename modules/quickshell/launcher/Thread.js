.pragma library

// What the shell's thread did over a span of the launcher's work, from its
// clocks, `{ ran, waited }` (KeyTimes.ran and waited), read at either end:
// what it ran, what it waited for a CPU behind the machine's other work,
// and the rest, what it slept: blocked on the render thread, the disk or a
// page, or idle in its event loop, as a key held for its answers is for
// up to its bound (Launcher.qml's answersWaitedFor), so a held key's
// sleep past that is the blocking alone. What it ran of a held key's
// searching includes the other work it did while the key waited.
// An opening's longest stall is said so (Pacing.feltTicked), and a slow
// key's searching: Joel's "\u2193" in Agents took 121 ms searching, mid key
// repeat with every move around it under 6, and his "ca" waited 62 ms
// behind the shell's other work, neither saying which of the three it was
// (2026-11-16).
// Beside them, `faulted`, its major page faults (KeyTimes.faulted), each
// a page read back in from swap or a file, though one may read several
// around it, so a sleep says how often it waited on its own memory: Joel's
// "s" slept 448 ms of 566 searching, in work that never waits on its
// event loop, with memory short (2026-11-21). A count isn't a time: one
// read from zswap's compressed pool (modules/memory) costs microseconds,
// one from the disk milliseconds. Nor does none clear memory: a thread
// reclaiming pages to allocate, or waiting on a page another thread is
// reading in, takes no major fault of its own.

// The thread's clocks across a span, from `from` to `to`; null where
// either end wasn't read, or its wait read 0, as KeyTimes.waited gives a
// failed read and one taken from it would be the thread's whole life's,
// or the clocks went back, which the thread's own never do. Its faults go
// with them, though getrusage may still have answered: a key's line then
// says neither, rather than a count beside clocks it couldn't read.
function spent(from, to) {
    if (!from || !to || !(from.waited > 0) || !(to.waited > 0) || to.ran < from.ran
        || to.waited < from.waited) {
        return null;
    }
    return { ran: to.ran - from.ran, waited: to.waited - from.waited, faulted: faults(from, to) };
}

// What the shell's thread and the collector spent of an opening's working,
// from the clocks read at its ask to those read as its working was done
// (Launcher.qml's feltClocks): `thread` as `spent` gives it, and
// `collecting`, the collector's milliseconds, never below none. An
// opening whose working wasn't done, or whose ask wasn't read, says
// neither, rather than a span that began with another opening's ask.
function workingSpent(asked, worked) {
    if (!asked || !worked) {
        return { thread: null, collecting: 0 };
    }
    return { thread: spent(asked.thread, worked.thread),
        collecting: Math.max(0, (worked.collected ?? 0) - (asked.collected ?? 0)) };
}

// The major page faults across a span, or undefined where either end
// wasn't read, as KeyTimes.faulted gives -1, or the count went back.
function faults(from, to) {
    if (!(from?.faulted >= 0) || !(to?.faulted >= from.faulted)) {
        return undefined;
    }
    return to.faulted - from.faulted;
}

// What the thread did in `all` ms of `of` (the stall, its searching), and
// its major page faults, said last on its line where they were read; or
// nothing when the span was under a millisecond or its clocks weren't
// read.
function said(all, inSpan, of) {
    const whole = Math.round(all);
    if (whole < 1 || !inSpan) {
        return "";
    }
    const ran = Math.round(inSpan.ran);
    const waited = Math.round(inSpan.waited);
    const paged = pageFaults(inSpan.faulted);
    return `; its thread ran ${ran} ms of ${of}, waited ${waited} for a CPU and slept`
        + ` ${Math.max(0, whole - ran - waited)}${paged}`;
}

// A span's major page faults as its line ends: none said as none, so it
// reads apart from a count that couldn't be read, which says nothing.
function pageFaults(faulted) {
    if (faulted === undefined) {
        return "";
    }
    if (faulted === 0) {
        return ", with no major page faults";
    }
    return `, with ${faulted} major page fault${faulted === 1 ? "" : "s"}`;
}

// What drawing a frame was, by the render thread's times of it where
// they were read (Launcher.qml's frameSplit): its polish and sync, its
// render and swap, and the GUI thread hearing the swap late, which the
// three add up to. The last is said as the swap heard, never the key,
// since a key's presented line says "before the shell heard it" of the
// key, and Joel's "wi", 64 ms here, was read as a key heard late
// (2026-11-12). A key's line and an opening's say it alike: his opening
// at 10:28 spent 121 ms drawing its frame, which nothing then told
// apart (2026-11-26).
function drawingApart(split) {
    if (split?.syncing === undefined) {
        return "";
    }
    return ` (${split.syncing} syncing, ${split.rendering} rendering,`
        + ` ${split.heard} before the shell heard it swapped)`;
}
