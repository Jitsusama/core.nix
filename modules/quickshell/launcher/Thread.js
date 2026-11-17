.pragma library

// What the shell's thread did over a span of the launcher's work, from its
// clocks, `{ ran, waited }` (KeyTimes.ran and waited), read at either end:
// what it ran, what it waited for a CPU behind the machine's other work,
// and the rest, what it slept, on the render thread, the disk or a page.
// An opening's longest stall is said so (Pacing.feltTicked), and a slow
// key's searching: Joel's "\u2193" in Agents took 121 ms searching, mid key
// repeat with every move around it under 6, and his "ca" waited 62 ms
// behind the shell's other work, neither saying which of the three it was
// (2026-11-16).

// The thread's clocks across a span, from `from` to `to`; null where
// either end wasn't read, or the clocks went back, as they would read on
// another thread than the one they were begun on.
function spent(from, to) {
    if (!from || !to || to.ran < from.ran || to.waited < from.waited) {
        return null;
    }
    return { ran: to.ran - from.ran, waited: to.waited - from.waited };
}

// What the thread did in `all` ms of `of` (the stall, its searching), said
// last on its line; or nothing when the span was under a millisecond or
// its clocks weren't read.
function said(all, inSpan, of) {
    const whole = Math.round(all);
    if (whole < 1 || !inSpan) {
        return "";
    }
    const ran = Math.round(inSpan.ran);
    const waited = Math.round(inSpan.waited);
    return `; its thread ran ${ran} ms of ${of}, waited ${waited} for a CPU and slept`
        + ` ${Math.max(0, whole - ran - waited)}`;
}
