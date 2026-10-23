// When what a URL said last still stands, for Provider.qml's ask, in plain
// JavaScript so a check can give it the time rather than wait for it: while
// it's still being asked, then for `fresh` milliseconds after an answer, or
// `failedFor` after a failure, so a network that's down isn't asked on
// every key.
.pragma library

function stands(entry, now, fresh, failedFor) {
    return Boolean(entry) && (entry.pending || now - entry.at < (entry.failed ? failedFor : fresh));
}

// How long until one still being asked counts as not answering, after
// `patience` milliseconds, or null when it isn't being asked: a network
// that's there but leads nowhere never answers at all, and an answer still
// being looked up keeps the launcher from copying it.
function overdueIn(entry, now, patience) {
    return entry && entry.pending ? Math.max(0, entry.at + patience - now) : null;
}
