// How long a secret ask's line was on Joel's screen when he said yes, which
// the yes tells the daemon, so that a yes made faster than a line can be read
// is refused as unread (the passwords daemon's least_shown_ms). Counted from
// the first frame niri presented with the line selected in this opening, the
// line Joel reads and acts on, so never longer than it was in front of him.
// A yes whose line no frame presented, whose line changed since, or made
// while the launcher is walked from outside says no time at all.
.pragma library

// What a yes says of its line, which only the launcher may say.
const stampedFields = ["shown", "opening"];

// The frames seen, by the row's key, each the line's digest and when the
// first frame with it selected reached the screen; a new digest starts the
// count again. Returns the same object when nothing changed, so a binding
// that holds it isn't told.
function noted(seen, key, digest, at) {
    if (seen[key]?.digest === digest) {
        return seen;
    }
    const next = Object.assign({}, seen);
    next[key] = { digest, at };
    return next;
}

// The ask a verb sends, as the row served it, with how long its line was
// shown and in which opening where a frame presented exactly that line; any
// time the served ask held is dropped, so only the launcher's own is sent.
// `launcher` is the moment it's sent in: `now` on the frames' clock, the
// `opening` drawn, and whether it's `cataloguing`, walked from outside. Only
// a yes needs its line read: a no and anything else go as served.
function stamped(ask, seen, key, launcher) {
    if (ask?.ask !== "release") {
        return ask;
    }
    // Copied by hand, since Qt's engine has no Object.fromEntries.
    const said = {};
    for (const name of Object.keys(ask)) {
        if (!stampedFields.includes(name)) {
            said[name] = ask[name];
        }
    }
    const frame = seen?.[key];
    if (launcher.cataloguing || !frame || frame.digest !== ask.digest) {
        return said;
    }
    return Object.assign(said, {
        shown: Math.max(0, Math.round(launcher.now - frame.at)),
        opening: launcher.opening
    });
}
