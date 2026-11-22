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
// Beside them, `faulted`, the pages it had to wait to have read back in
// (KeyTimes.faulted), so a sleep says whether it was on the thread's own
// memory: Joel's "s" slept 448 ms of 566 searching, in work that never
// waits on its event loop, with memory short (2026-11-21).

// The thread's clocks across a span, from `from` to `to`; null where
// either end wasn't read, or its wait read 0, as KeyTimes.waited gives a
// failed read and one taken from it would be the thread's whole life's,
// or the clocks went back, which the thread's own never do.
function spent(from, to) {
    if (!from || !to || !(from.waited > 0) || !(to.waited > 0) || to.ran < from.ran
        || to.waited < from.waited) {
        return null;
    }
    return { ran: to.ran - from.ran, waited: to.waited - from.waited, faulted: faults(from, to) };
}

// The pages faulted in across a span, or undefined where either end
// wasn't read, as KeyTimes.faulted gives -1, or the count went back.
function faults(from, to) {
    if (!(from?.faulted >= 0) || !(to?.faulted >= from.faulted)) {
        return undefined;
    }
    return to.faulted - from.faulted;
}

// What the thread did in `all` ms of `of` (the stall, its searching), and
// the pages it had read back in where there were any, said last on its
// line; or nothing when the span was under a millisecond or its clocks
// weren't read.
function said(all, inSpan, of) {
    const whole = Math.round(all);
    if (whole < 1 || !inSpan) {
        return "";
    }
    const ran = Math.round(inSpan.ran);
    const waited = Math.round(inSpan.waited);
    const paged = inSpan.faulted > 0 ? ` with ${inSpan.faulted} pages read back in` : "";
    return `; its thread ran ${ran} ms of ${of}, waited ${waited} for a CPU and slept`
        + ` ${Math.max(0, whole - ran - waited)}${paged}`;
}
