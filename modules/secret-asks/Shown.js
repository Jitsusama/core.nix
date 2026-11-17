// How long a secret ask's line was on Joel's screen when he said yes, which
// the yes tells the daemon, so that a yes made faster than a line can be read
// is refused as unread (the passwords daemon's least_shown_ms). Counted from
// the first frame niri presented with the line selected in this opening, the
// line Joel reads and acts on, so never longer than it was in front of him.
// A yes whose line no frame presented, or whose line changed since, says no
// time at all; one made while the launcher is walked from outside, as its
// IPC's do and run walk it, isn't sent, since only Joel's own key press on
// the drawn line grants an ask, and any process running as him reaches the
// IPC.
.pragma library

// What a yes says of its line, which only the launcher may say.
const stampedFields = ["shown", "opening"];

// Keys that only move through what's shown, as the launcher's table names
// them, wherever they're pressed; j, k and the rest of normal's moves arm
// only where they move rather than type.
const navigating = ["Up", "Down", "Left", "Right", "Tab", "Shift+Tab", "Escape"];

// Return, which answers a line rather than reads it.
const answering = /^(.*\+)?(Return|Enter)$/;

// What a key Joel pressed, by its name, does to the selected ask's line:
// "arms" for a key that only moves through what's shown (`moved` says the
// launcher took it as a move), so the line is timed from its next frame;
// "passes" for Return, which leaves it as it was; and "resets" for any
// other, typing above all, so its time starts again only from a move after
// it. A line is timed only from his own key, since a launcher opened from
// outside, as any process running as him may, could have its next Returns
// be his, typed into another window (ISXEG8, 2026-11-17).
function keyed(name, moved) {
    if (answering.test(name)) {
        return "passes";
    }
    return navigating.includes(name) || moved ? "arms" : "resets";
}

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

// The frames seen, keeping only the selected row's, so a line left and come
// back to counts again from its return rather than from when it was first
// shown; nothing for `key` undefined, with nothing selected. Returns the
// same object when nothing changed, as `noted` does.
function kept(seen, key) {
    const others = Object.keys(seen).filter(name => name !== key);
    if (others.length === 0) {
        return seen;
    }
    return key in seen ? { [key]: seen[key] } : {};
}

// The ask a verb sends, as the row served it, with how long its line was
// shown and in which opening where a frame presented exactly that line; any
// time the served ask held is dropped, so only the launcher's own is sent.
// `launcher` is the moment it's sent in: `now` on the frames' clock, the
// `opening` drawn, and whether it's `cataloguing`, walked from outside, when
// it's null, sending nothing. Only a yes needs its line read: a no and
// anything else go as served.
function stamped(ask, seen, key, launcher) {
    if (ask?.ask !== "release") {
        return ask;
    }
    if (launcher.cataloguing) {
        return null;
    }
    // Copied by hand, since Qt's engine has no Object.fromEntries.
    const said = {};
    for (const name of Object.keys(ask)) {
        if (!stampedFields.includes(name)) {
            said[name] = ask[name];
        }
    }
    const frame = seen?.[key];
    if (!frame || frame.digest !== ask.digest) {
        return said;
    }
    return Object.assign(said, {
        shown: Math.max(0, Math.round(launcher.now - frame.at)),
        opening: launcher.opening
    });
}
