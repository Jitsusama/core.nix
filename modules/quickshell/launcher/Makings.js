// How long each kind of drawer took to make (Pacing.js's drawerMade), as
// the launcher keeps it across the shell's restarts, in
// launcher-drawers.json. Kept in memory alone, each restart, as most
// switches are, emptied it, so a kind Joel steps onto a few times a day was
// a kind not yet made at nearly every key: Wi-Fi's drawers made in a median
// of 6 ms, yet 5 of the 6 from a key came the frame after, a median of 53 ms
// from it (F8TT8O, 2026-11-18).
.pragma library
.import "Pacing.js" as Pacing

// The most kinds a file is read for, far past the few dozen a day's places
// make, so a file grown by hand costs the shell's start no more than that.
const kindsRead = 256;
// The most of a file read, a few times what 256 kinds of eight times fill,
// since it's read on the shell's thread as it starts.
const readUpTo = 65536;

// The makings `text` keeps, each kind's last eight, in ms, or null where
// there's text that can't be read as makings (not JSON, not an object of
// kinds, or past what's read), for the launcher to say it starts afresh.
// Empty text keeps none and is no fault. What isn't a kind's list of times
// is left out, and so is a kind named like what every object holds
// (constructor, toString), which no place is, so a file the shell couldn't
// finish writing only costs what it taught.
function read(text) {
    const given = String(text ?? "");
    if (given.trim() === "") {
        return {};
    }
    if (given.length > readUpTo) {
        return null;
    }
    let said = null;
    try {
        said = JSON.parse(given);
    } catch (_unread) {
        return null;
    }
    if (said === null || typeof said !== "object" || Array.isArray(said)) {
        return null;
    }
    const kept = {};
    for (const kind of Object.keys(said).slice(0, kindsRead)) {
        if (!(kind in Object.prototype)) {
            kept[kind] = said[kind];
        }
    }
    return joined({}, kept);
}

// `older`'s makings with `newer`'s after them, each kind's last eight, so
// what the file kept and what this shell made before reading it are one.
// Neither is changed.
function joined(older, newer) {
    let makings = older;
    for (const kind of Object.keys(newer)) {
        const times = Array.isArray(newer[kind]) ? newer[kind] : [];
        for (const making of times) {
            if (Number.isFinite(making) && making >= 0) {
                makings = Pacing.drawerMade(makings, kind, making);
            }
        }
    }
    return makings;
}
