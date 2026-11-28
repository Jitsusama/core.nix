// What a daemon's answer to a place's search found, as ReadyRows.qml holds
// it beside the rows the watch serves (ready-rows.md's search), and what
// the find's head says of the rows found (Launcher.qml's `narrowed`), and
// tests/ready-rows.qml runs it in Qt's own engine. It's apart from
// ReadyRows.js, which words rows, because a search only orders them.
.pragma library

// What a daemon's answer to a search of `words` found: its keys best
// first, the letters of each name it matched, and the rows it found that
// the watch never served, as Capture's taken files are, a few thousand of
// which Recent serves nine. Those come wrapped as the stream wraps a row,
// so they're read as any record is. Its grade is the daemon's where it
// says one, as Emoji's says 1 for a best found only loosely, else 0. How
// many it found is the daemon's where it says, past the keys it answers
// with, as Emoji's answers a hundred of all it found, else its keys'. No
// answer yet is `words` null, with nothing found.
function foundOf(words, answer) {
    const rows = {};
    for (const [key, row] of Object.entries(answer?.rows ?? {})) {
        rows[key] = { key, kind: "row", row };
    }
    const keys = answer?.keys ?? [];
    return {
        words: answer ? words : null,
        keys,
        marks: answer?.marks ?? {},
        grade: answer?.grade ?? 0,
        found: answer?.found ?? keys.length,
        rows
    };
}

// What the find's head says it found among `matches`, the rows shown, in `field`'s words
// (words.json's field.found and field.foundOf): "2 found", or "2 of 7 found" where the held
// place's daemon found `past` more than it listed. A row a steadying refresh kept dimmed where it
// was, gone from what its source answers last (Steady.kept's `went`), is no longer found, so it
// isn't counted (ZINAWZ's F8), and where every row went the head says nothing. A row its source
// draws dimmed itself is still one it found.
function foundSaid(matches, past, field) {
    const count = matches.filter(row => !row.went).length;
    if (count === 0) {
        return "";
    }
    return past > 0
        ? (field.foundOf ?? "")
              .replace("{count}", String(count))
              .replace("{all}", String(count + past))
        : (field.found ?? "").replace("{count}", String(count));
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
