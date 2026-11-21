// How a row that ends the session arms before it's done (VKCF3Z's ruling,
// 2026-11-12 [c27ab604]): the first Return arms Log Out, Reboot or Power
// Off in place, its row reading armed and the foot saying what a second
// does, and a second Return with no other key between does it. Any other
// key lets it go, with no time limit, as a landing lasts until the next
// key (Launcher.qml's landedOn); so does the launcher shutting, and, as
// Asks (secret-asks) tells the launcher, a share beginning (untold) or
// ending (shareTurns), and niri first saying its casts, a share among
// them or not, since until then the screen counts as shared. Apart from
// the launcher, so tests/arming.qml holds it in Qt's own engine.
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
// into; a still row with a run, as the file chooser's, still runs it. A
// still row is held whatever else it holds, since it says Return has
// nothing to do; never one gone, as Steady keeps it dimmed, whose Return
// does nothing and says so nowhere.
function holdsStill(row) {
    return !!row && row.still === true && typeof row.run !== "function" && row.gone !== true;
}

// Why a row held still does nothing (`stillWhy`), or `fallback` where it
// says none; null for any other row, so one held with no words anywhere
// is still told from one not held.
function heldWhy(row, fallback) {
    if (!holdsStill(row)) {
        return null;
    }
    const why = row.stillWhy;
    return typeof why === "string" && why !== "" ? why : fallback;
}

// Why a way in from outside is refused for a row held still: the first
// step on it held, not only the last, or the last one's Return's act
// (`act`), so nothing a held row holds is reached past it; null where
// none is held.
function wayHeldWhy(trail, act, fallback) {
    for (const row of trail.concat(act ? [act] : [])) {
        const why = heldWhy(row, fallback);
        if (why !== null) {
            return why;
        }
    }
    return null;
}

// Whether the foot, `saying`, lets go of the word it said for a Return
// held still, `heldSaying`, once what held it may no longer: only while
// it still says that word, so another said since, another place's
// among them, stays.
function letsHeldGo(saying, heldSaying) {
    return saying.length > 0 && JSON.stringify(saying) === JSON.stringify(heldSaying);
}

// What a share ending leaves (Launcher.qml's unheld), or niri saying its
// casts during one: whether the foot goes `quiet`, letting go of its word
// for a held Return (letsHeldGo), and nothing `armed`. Every arm goes, as
// one does as a share begins (untold), not only an ask's: one made on an
// ask's row in the paced refresh after a share began, before the row was
// hidden, would otherwise let the first Return after the share, no key
// between, release on a line read before it. A Reboot armed meanwhile
// takes one Return more, which errs safe.
function shareTurns(saying, heldSaying) {
    return { quiet: letsHeldGo(saying, heldSaying), armed: rest() };
}

// What Return does on the selected row, from what `how` says of it: an
// answer being given is given, a Return doubling a landing does nothing
// more, a gone row is only run, and a held row is held before anything else, as every other
// way in asks it first; then one that arms arms ("arming", for the
// caller to press), one that steps in is entered, one that ends
// something is lined, one that changes something is landed on, and any
// other runs.
function returnAct(how) {
    if (how.asking) {
        return "gave";
    }
    if (how.landed) {
        return "";
    }
    // A row whose thing has gone is only run, which does nothing (Launcher.qml's run): a line
    // written for it, or an arming, would do it again (C2597D63, 2026-11-20).
    if (how.gone) {
        return "ran";
    }
    if (how.held) {
        return "held";
    }
    if (how.arming) {
        return "arming";
    }
    if (how.steps) {
        return "entered";
    }
    if (how.ends) {
        return "lined";
    }
    return how.lands ? "landed" : "ran";
}

// Whether the place a source `source` lists may be kept, to draw at once
// at the next opening (Launcher.qml's keepPlace): never one that says its
// rows are never kept, as Asks does. A kept place's rows aren't drawn
// under a share (Wanted.keptPlaceListed), and Asks' only kept step today
// is its own, which names nothing, so this is a second guard: a way kept
// deeper into it, whose steps intoKept draws under a share, couldn't
// name an ask from before one.
function keepsPlace(source) {
    return source?.keptNever !== true;
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
