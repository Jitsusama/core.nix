.pragma library

// What an opening's line says of Qt's collector, from the counts
// Pacing.feltTicked keeps and Heap gives (modules/quickshell/heap).

// What the collector took of an opening, in its longest stall and in all,
// said after the stall's parts, or nothing when it took under half a
// millisecond, which rounds to none.
function collectorSaid(inStall, inAll) {
    const all = Math.round(inAll);
    if (all < 1) {
        return "";
    }
    return `; the collector ${Math.round(inStall)} ms of the stall, ${all} of the opening`;
}

// What the collector took of an opening's working, from its ask to its
// first frame's work done, said on its line beside what its thread did, or
// nothing when it took under half a millisecond: Joel's slow openings into Now
// asked 15 to 22 ms, and none said whether a collection was among it
// (2026-11-21).
function workingSaid(inWorking) {
    const working = Math.round(inWorking);
    return working < 1 ? "" : `; the collector ${working} ms of its working`;
}

// The limits Heap.standing says set where the collector starts, in the
// words a line says them in.
const LIMITS = {
    sweep: "Qt's own limit",
    room: "the room past what they held",
    rooms: "the rooms",
    kept: "the rooms and what the sweeps kept since",
};

// Where the rooms alone cap the chunks, and which limit set where the
// collector starts, as Heap.standing says them, said in brackets after
// that point, or nothing from a standing that doesn't say the cap. The
// ceiling moves with what the sweeps keep, so a collector started past
// the rooms says whether what was kept or Qt's own limit put it there. A
// room of none is said as none asked, so it doesn't read as a heap with
// no room left in it.
function limitSaid(stood) {
    if (!stood || typeof stood.roomsCapAt !== "number" || Number.isNaN(stood.roomsCapAt)) {
        return "";
    }
    const by = LIMITS[stood.collectsBy];
    const set = by === undefined ? "" : `, set by ${by}`;
    const rooms = stood.roomsCapAt < 0 ? "no room asked"
        : `the rooms ${Math.round(stood.roomsCapAt)} MB`;
    return ` (${rooms}${set})`;
}

// Where the heap stood against the collector as the opening was asked, as
// Heap.standing says it, said after the collector's part when it took a
// millisecond or more of the longest stall, or nothing. It's read once
// the opening's room is left (Heap.leaveRoom), so it's the room each
// limit had as the opening began: outside, always the room or more; the
// chunks, the room, or more where Qt's own limit from the last sweep is
// farther, unless their cap holds them nearer, which limitSaid says. It
// isn't which limit the opening's own work went on to meet, nor what the
// time while shut left, and a collection begun after the ask isn't said
// under way. Collections ran inside 19 of 670 of Joel's openings' stalls,
// 57 to 136 ms each, and were most of the slowest, but which limit
// started them, the chunks', the strings' outside the heap, or one begun
// while the launcher was shut, went unsaid (2026-11-20).
function standingSaid(inStall, stood) {
    if (Math.round(inStall) < 1 || !stood || stood.chunks === undefined) {
        return "";
    }
    const under = stood.collecting ? ", a collection under way" : "";
    return `; at the ask, chunks ${Math.round(stood.chunks)} of`
        + ` ${Math.round(stood.collectsAt)} MB${limitSaid(stood)} and outside`
        + ` ${Math.round(stood.outside)} of ${Math.round(stood.collectsOutsideAt)} MB${under}`;
}

// How long an open launcher goes without a key, a click or the wheel, and
// how far its chunks grow past where the room was last left, before it
// collects while nobody's at it. Left to Qt, a long opening grows until
// the collector starts at the room's edge, wherever the next key is, and
// collections inside openings took 47 to 377 ms (Launcher.qml's
// openingRoom). The nested shell's walks through drawers never left 700 ms
// between keys, so a collection never comes between keys typed together;
// 45 drawers made 25 to 30 MB there, so 8 MB comes a few times in a long
// opening, not at every rest. Resting after every fifth drawer, in 20
// openings of 45 and one of 120, it came 72 times, 31 ms at the median and
// 91 at most, 703 to 796 ms after a key. In the 20 openings no key had the
// collector in it, against 6 keys and 107 ms without it, and the chunks
// before each closing's collection stayed at 50 MB, against 69 to 79
// (2026-11-28).
const IDLE_QUIET_MS = 700;
const IDLE_GROWN_MB = 8;

// Whether an open launcher should collect now, from the monotonic clock
// now, when it last heard a key, a click or the wheel, its chunks now, and
// its chunks where the room was last left, at the ask or the last such
// collection, all as Heap.standing and KeyTimes say them.
function idleCollectionDue(now, inputHeard, chunks, chunksFrom) {
    return now - inputHeard >= IDLE_QUIET_MS && chunks - chunksFrom >= IDLE_GROWN_MB;
}

// The journal's line for a collection while nobody's at it: how long it
// took, the chunks before and after, and how long after the last input it
// came.
function idleCollectedSaid(took, before, after, quiet) {
    return `launcher: collected idle in an opening in ${Math.round(took)} ms, chunks`
        + ` ${Math.round(before)} MB before it and ${Math.round(after)} after,`
        + ` ${Math.round(quiet)} ms after the last input`;
}
