// When what a URL said last still stands, for Provider.qml's ask, in plain
// JavaScript so a check can give it the time rather than wait for it: while
// it's still being asked, then for `fresh` milliseconds after an answer, or
// `failedFor` after a failure, so a network that's down isn't asked on
// every key.
.pragma library

function stands(entry, now, fresh, failedFor) {
    return !!entry && (entry.pending || now - entry.at < (entry.failed ? failedFor : fresh));
}
