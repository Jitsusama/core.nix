// When what a URL said last still stands, and whether an answer says
// anything new, for Provider.qml's ask, in plain
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
    return entry?.pending ? Math.max(0, entry.at + patience - now) : null;
}

// What stands for a URL as it's asked again: what it last said, a value
// or a failure, until the answer comes, so nothing drawn from it changes
// while it's asked.
function askedAgain(kept, now) {
    return { pending: true, value: kept?.value, failed: kept?.failed ?? false, at: now };
}

// What an answer says, as `stands` and the providers read it.
function said(entry) {
    return JSON.stringify([entry.value, entry.failed]);
}

// The answer to what was asked, `reply` holding its status and text, kept
// in `entry`; whether it says anything new. Its provider says it was
// answered only then, since every closing makes again the rows of a
// source that said so: a lookup that failed each time it was asked, as a
// public IPv6 address does on a machine with no IPv6 route, made
// Network's 11 rows again at every closing (2026-11-11).
function heard(entry, reply, now) {
    const before = said(entry);
    entry.pending = false;
    entry.at = now;
    entry.status = reply.status;
    entry.failed = false;
    // A reply that isn't JSON fails as a bad status does. Qt's engine takes
    // no catch without a binding, so the error is named and left unread.
    try {
        if (reply.status !== 200) {
            throw new Error(String(reply.status));
        }
        entry.value = JSON.parse(reply.text);
    } catch (_unread) {
        entry.failed = true;
    }
    return said(entry) !== before;
}

// One given up on, as failed; whether that's new, as `heard` says.
function gaveUp(entry, now) {
    const before = said(entry);
    entry.pending = false;
    entry.failed = true;
    entry.at = now;
    return said(entry) !== before;
}
