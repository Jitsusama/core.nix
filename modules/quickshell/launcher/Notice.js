// A notice's acts, said once for everywhere it shows: its card on the
// screen and its row among the notices in the launcher list the same acts
// in the same order, so Joel learns one notice, not two (Joel,
// 2026-10-28 22:20: "we need to not have different versions of these
// sort of elements"; the launcher's design, RSCH-20261028-2NK1E5, the
// notice component). First going to what it's about, which Return does,
// so the right arrow holds it too: opening it, by its sender's own
// action, or else going to the sender's window. Then what its sender
// offers, whether it's read, and forgetting it, since what changes a
// notice comes before what ends it; then, on the card, what's done to
// the whole stack; and copying what it said last of all, never first (Joel, 2026-10-28: "having the separate copy
// option at the top is very confusing"). Apart from the shell, so
// tests/notice.qml can hold it in Qt's own engine.
.pragma library

// Each act by what it does; each place says it in its own words
// (launcher/words.json) and runs it its own way.
const OPEN = "open";
const OFFER = "offer";
const GO_TO = "goTo";
const READ = "read";
const FORGET = "forget";
const FORGET_ALL = "forgetAll";
const ALL = "all";
const COPY = "copy";

// The acts a notice has, in order. `notice` says what it is now: whether
// its sender opens what it's about (`opens`), what else its sender
// offers (`offers`, each its identifier and text), and whether it's still
// open for the sender to act on (`live`). `here` says what the place adds:
// whether the sender has a window to go to (`window`) and whether the
// notice is the front of the stack on screen (`stack`). A notice gone
// from the screen keeps none of its sender's acts, since Quickshell
// destroys them as it closes. On the card, Return forgets a notice with
// nothing to open, so forgetting it leads there, as Return's act always
// leads inside.
function acts(notice, here) {
    const out = [];
    const forgets = notice.live && here.stack && !notice.opens;
    if (forgets) {
        out.push({ act: FORGET });
    }
    if (notice.live && notice.opens) {
        out.push({ act: OPEN });
    } else if (here.window) {
        out.push({ act: GO_TO });
    }
    if (notice.live) {
        for (const offer of notice.offers) {
            out.push({ act: OFFER, identifier: offer.identifier, text: offer.text });
        }
    }
    out.push({ act: READ });
    if (notice.live && !forgets) {
        out.push({ act: FORGET });
    }
    if (here.stack) {
        out.push({ act: FORGET_ALL }, { act: ALL });
    }
    out.push({ act: COPY });
    return out;
}

// What a notice offers, from its sender's actions: every one but the one
// that opens what it's about, each its identifier and text.
function offers(actions) {
    return actions
        .filter((action) => action.identifier !== "default")
        .map((action) => ({ identifier: action.identifier, text: action.text }));
}

// Whether its sender opens what it's about, from its actions.
function opens(actions) {
    return actions.some((action) => action.identifier === "default");
}
