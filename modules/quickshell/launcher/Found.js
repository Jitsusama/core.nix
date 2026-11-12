// What a daemon's answer to a place's search found, as ReadyRows.qml holds
// it beside the rows the watch serves (ready-rows.md's search), and
// tests/ready-rows.qml runs it in Qt's own engine. It's apart from
// ReadyRows.js, which words rows, because a search only orders them.
.pragma library

// What a daemon's answer to a search of `words` found: its keys best
// first, the letters of each name it matched, and the rows it found that
// the watch never served, as Capture's taken files are, a few thousand of
// which Recent serves nine. Those come wrapped as the stream wraps a row,
// so they're read as any record is. No answer yet is `words` null, with
// nothing found.
function foundOf(words, answer) {
    const rows = {};
    for (const [key, row] of Object.entries(answer?.rows ?? {})) {
        rows[key] = { key, kind: "row", row };
    }
    return {
        words: answer ? words : null,
        keys: answer?.keys ?? [],
        marks: answer?.marks ?? {},
        rows
    };
}

// The record of a row a search found: the watch's where it serves one, so
// a change naming it replaces the answer's, else the answer's own, else
// null for a key served by neither.
function foundRecord(found, held, key) {
    return held[key] ?? found.rows[key] ?? null;
}

// Whether a change moves what a search found among: a row placed before
// it or after, which the daemon must be asked of again. A row served with
// no order, one a search found that's ahead and so served whole, only
// replaces the answer's copy, which stands, as a drawer served apart
// changes no row's place.
function reranks(before, now, moved) {
    const either = new Set(before.concat(now));
    return moved.some(key => either.has(key));
}
