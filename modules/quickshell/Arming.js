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

// Whether `armed` still stood with the line at `keysTaken`, so a share's
// turn letting it go (shareTurns, Launcher.qml's untold) is one the
// journal says went: an arm a key since already let go of isn't.
function stood(armed, keysTaken) {
    return holds(armed, armed.key, keysTaken);
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

// Whether Launcher.qml's run, given `choice`, changes no source, read in
// run's own order, so the line's key that runs it from inside a step is
// timed as acting only where it acts (commandRan). `inside` says a step
// is held, and `heldStill` that run holds the row still (heldWhyOf). A
// gone row does nothing, and one held still says why and stays; a start
// put in the field leaves the place, so it acts; a Return's act that asks
// for words only asks (ranReturn), and any other may do anything; a
// category or group steps in; then a verb needing a target asks for it,
// inside a step and with no run of its own (or more), and one needing
// words asks for them. Anything else may act. A category whose step-in
// is refused runs on past it in run, which is taken as the step-in here,
// as no category has a run of its own.
function changesNone(choice, inside, heldStill) {
    if (choice.gone || heldStill) {
        return true;
    }
    if (choice.start !== undefined) {
        return false;
    }
    if (choice.returns) {
        return !!choice.returns.given;
    }
    if (choice.category || choice.group) {
        return true;
    }
    return (inside && !!choice.targets && (!choice.run || choice.verb === "more"))
        || !!choice.given;
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

// Whether a place may be kept, judged by the source its name finds,
// `source`, and by the source each of its `steps` comes from (a step's
// row.from), so a chord naming a place otherwise than its source, its
// lookup unfound, can't keep a never-kept one's steps. A step from no
// source, and a name no source has, keep, as home's led places do. A
// row made showing says its source by shownBy wherever it's held, as
// For You holds an ask under Now with no from, so that counts as one,
// and a row saying both keeps only where neither says never. A row held
// still keeps nothing either, as an ask made hidden under a share says
// neither (secret-asks' Asks.qml), so its step can't be kept by saying
// no source.
function keepsSteps(source, steps) {
    return keepsPlace(source) && steps.every(step => step.row?.still !== true
        && keepsPlace(step.row?.from) && keepsPlace(step.row?.shownBy));
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
