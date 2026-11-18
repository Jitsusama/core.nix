// How a row that ends the session arms before it's done (VKCF3Z's ruling,
// 2026-11-12 [c27ab604]): the first Return arms Log Out, Reboot or Power
// Off in place, its row reading armed and the foot saying what a second
// does, and a second Return with no other key between does it. Any other
// key lets it go, with no time limit, as a landing lasts until the next
// key (Launcher.qml's landedOn). Apart from the launcher, so
// tests/arming.qml holds it in Qt's own engine.
//
// Each press is read against the keys the line has taken (Prompt.qml's
// keysTaken), which counts every key, Return too, before the launcher
// hears it, so a Return counts itself (`counted` 1) and a click doesn't
// (0).
//
// A row may also hold Return still (holdsStill), as a secret ask hidden
// while Joel shares his screen, which neither arms nor acts.
.pragma library

// Whether Return on the row keeps the launcher open and does nothing, so
// a second Return meant for a yes never reaches the window behind. Such a
// row says it has nothing to do yet (`still`), so the foot names no
// Return, and has no run, so it has no act for the right arrow to step
// into; a still row with a run, as the file chooser's, still runs it.
function holdsStill(row) {
    return !!row && row.still === true && typeof row.run !== "function";
}

// Why a row held still is refused when a terminal asks for it (`held`),
// or `fallback` where it says none; "" for any other row.
function heldWhy(row, fallback) {
    if (!holdsStill(row)) {
        return "";
    }
    return typeof row.held === "string" && row.held !== "" ? row.held : fallback;
}

// Nothing armed.
function rest() {
    return { key: "", keys: -1, at: 0 };
}

// Whether the row of steady id `id` stands armed: armed, and no key taken
// since.
function holds(armed, id, keysTaken) {
    return armed.key !== "" && armed.key === id && keysTaken === armed.keys;
}

// What a press on a row that arms does, and how arming stands after it:
// "armed" for the first, or one after another key; "ends" for the next
// press with no key between, which does it; or "" for one so soon after
// arming (within `wait`, in ms) that it's the arming press doubled, a key
// bounced, which does nothing and leaves it armed, as a landing's doubled
// Return does.
function pressed(armed, id, how) {
    const again = armed.key !== "" && armed.key === id
        && how.keysTaken - armed.keys === how.counted;
    if (!again) {
        return { act: "armed", armed: { key: id, keys: how.keysTaken, at: how.now } };
    }
    if (how.now - armed.at < how.wait) {
        return { act: "", armed: { key: armed.key, keys: how.keysTaken, at: armed.at } };
    }
    return { act: "ends", armed: rest() };
}
