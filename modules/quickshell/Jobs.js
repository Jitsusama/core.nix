// What Jobs.qml makes of the machine queue as the mycelium node serves it:
// the harness's file held whole as one `queue` record (modules/mycelium),
// which the node marks with the interface's own key, kind and when it was
// heard (its PROTOCOL.md, The Daemon Interface). Kept apart from
// Quickshell's types, so the `the-queue-is-heard-from-the-node` check runs
// it in Qt's own engine (tests/jobs.qml).
.pragma library

// The fields the node lays on every record, none of them the harness's.
const INTERFACE = ["key", "kind", "heard"];

// The fields the harness rewrites each time that say only when it wrote,
// which nothing draws: the machine queue is rewritten about 13 times a
// second, its `at` alone changing in nine writes of ten (2026-10-29).
const BEATS = ["heartbeat", "at"];

// The file as the harness wrote it, from the node's record of it, or null
// with none, or none of a version this reads.
function told(record) {
    if (record === null || typeof record !== "object" || record.v !== 1)
        return null;
    const file = {};
    for (const field of Object.keys(record)) {
        if (!INTERFACE.includes(field))
            file[field] = record[field];
    }
    return file;
}

// What a file says beyond its beats, as text, so a write that says only
// that the harness still runs is told from one that says something new,
// and what reads the queue runs again only for that. The node lays a
// change's fields over the record it holds, so their order is kept from
// one write to the next.
function saying(file) {
    if (file === null)
        return "";
    const kept = {};
    for (const field of Object.keys(file)) {
        if (!BEATS.includes(field))
            kept[field] = file[field];
    }
    return JSON.stringify(kept);
}

// How many milliseconds from `now` until a file whose last heartbeat was
// `beat` (seconds since the epoch) has gone `silence` seconds unheard:
// zero once it has.
function quietIn(beat, now, silence) {
    return Math.max(0, (beat + silence) * 1000 - now);
}
