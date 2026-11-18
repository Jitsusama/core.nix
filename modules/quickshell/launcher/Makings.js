// How long each kind of drawer took to make (Pacing.js's drawerMade), as
// the launcher keeps it across the shell's restarts, in
// launcher-drawers.json. Kept in memory alone, each restart, as most
// switches are, emptied it, so a kind Joel steps onto a few times a day was
// a kind not yet made at nearly every key: Wi-Fi's drawers made in a median
// of 6 ms, yet 5 of the 6 from a key came the frame after, a median of 53 ms
// from it (F8TT8O, 2026-11-18).
.pragma library
.import "Pacing.js" as Pacing

// The makings `text` keeps, each kind's last eight, in ms. What isn't a
// kind's list of times is left out, and text that isn't JSON keeps none,
// so a file the shell couldn't finish writing only costs what it taught.
function read(text) {
    let said = null;
    try {
        said = JSON.parse(text);
    } catch (_unread) {
        return {};
    }
    const kinds = said !== null && typeof said === "object" && !Array.isArray(said);
    return joined({}, kinds ? said : {});
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
