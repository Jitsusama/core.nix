.pragma library

// What an opening's line says of Qt's collector, from the counts
// Pacing.feltTicked keeps and Heap gives (modules/quickshell/heap).

// What the collector took of an opening, in its longest stall and in all,
// said after the stall's parts, or nothing when it took under a
// millisecond.
function collectorSaid(inStall, inAll) {
    const all = Math.round(inAll);
    if (all < 1) {
        return "";
    }
    return `; the collector ${Math.round(inStall)} ms of the stall, ${all} of the opening`;
}

// What the collector took of an opening's working, from its ask to its
// first frame's work done, said on its line beside what its thread did, or
// nothing when it took under a millisecond: Joel's slow openings into Now
// asked 15 to 22 ms, and none said whether a collection was among it
// (2026-11-21).
function workingSaid(inWorking) {
    const working = Math.round(inWorking);
    return working < 1 ? "" : `; the collector ${working} ms of its working`;
}

// Where the heap stood against the collector as the opening was asked, as
// Heap.standing says it, said after the collector's part when it took a
// millisecond or more of the longest stall, or nothing. It's read once
// the opening's room is left (Heap.leaveRoom), so it's the room each
// limit had as the opening began: outside, always the room or more; the
// chunks, the room, or more where Qt's own limit from the last sweep is
// farther, unless their cap holds them nearer. It isn't which
// limit the opening's own work went on to meet, nor what the time while
// shut left, and a collection begun after the ask isn't said under way. Collections ran
// inside 19 of 670 of Joel's openings' stalls, 57 to 136 ms each, and
// were most of the slowest, but which limit started them, the chunks', the
// strings' outside the heap, or one begun while the launcher was shut,
// went unsaid (2026-11-20).
function standingSaid(inStall, stood) {
    if (Math.round(inStall) < 1 || !stood || stood.chunks === undefined) {
        return "";
    }
    const under = stood.collecting ? ", a collection under way" : "";
    return `; at the ask, chunks ${Math.round(stood.chunks)} of`
        + ` ${Math.round(stood.collectsAt)} MB and outside ${Math.round(stood.outside)} of`
        + ` ${Math.round(stood.collectsOutsideAt)} MB${under}`;
}
