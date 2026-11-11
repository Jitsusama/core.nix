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
// option at the top is very confusing"). Then how its drawer tells it:
// its kind, its eyebrow and its facts; and how its row is named. Apart
// from the shell, so
// tests/notice.qml can hold it in Qt's own engine.
.pragma library

// Each act by what it does; each place says it in its own words
// (launcher/words.json) and runs it its own way.
const OPEN = "open";
const OFFER = "offer";
const LINK = "link";
const GO_TO = "goTo";
const READ = "read";
const FORGET = "forget";
const FORGET_ALL = "forgetAll";
const ALL = "all";
const COPY = "copy";

// The acts a notice has, in order. `notice` says what it is now: whether
// its sender opens what it's about (`opens`), what else its sender
// offers (`offers`, each its identifier and text), and whether it's still
// open for the sender to act on (`live`), and the links in what it said
// (`links`, each { text, href }), each an act of its own in the order it
// said them, after what its sender offers and before Read, kept whether or
// not it's still open, since a link needs nothing of its sender. `here`
// says what the place adds:
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
    for (const link of notice.links ?? []) {
        out.push({ act: LINK, href: link.href, text: link.text });
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

// Its sender's own Read, where it offers one among its actions, by its
// identifier or its words, else null: what d sends at once as it's done
// with a notice, on its card and in the launcher alike.
const READ_ACT = /^(mark[-_ ]?(as[-_ ]?)?)?read$/i;
function readOffer(actions) {
    return actions.find((action) => READ_ACT.test(action.identifier) || READ_ACT.test(action.text))
        ?? null;
}

// Whether its sender opens what it's about, from its actions.
function opens(actions) {
    return actions.some((action) => action.identifier === "default");
}

// What kind of notice it is, in its drawer's words, from the category its
// sender gives as the spec's `class.subtype`: "message" for `im`, an error
// subtype as "transfer failed", and a bounced mail as "mail bounced"
// (VKCF3Z, 2026-11-08). A category it doesn't name is left out, never
// shown raw. `said` holds each class's word (`kinds`, by class), and
// what `failed` and `bounced` say.
function kind(category, said) {
    const given = String(category ?? "");
    const dot = given.indexOf(".");
    const type = dot < 0 ? given : given.slice(0, dot);
    const subtype = dot < 0 ? "" : given.slice(dot + 1);
    const word = said.kinds[type] ?? "";
    if (word === "") {
        return "";
    }
    if (given === "email.bounced") {
        return said.bounced;
    }
    return subtype === "error" ? said.failed.replace("{kind}", word) : word;
}

// Its drawer's eyebrow: where it's from, then its kind, never a time,
// which a fact says (VKCF3Z, 2026-11-08). `from` says the sender with
// `{who}`.
function eyebrow(who, kindSaid, from) {
    return [who === "" ? "" : from.replace("{who}", who), kindSaid]
        .filter((part) => part !== "")
        .join(" \u00b7 ");
}

// How a notice's row is named, the same in every list (VKCF3Z,
// 2026-11-08): by its title, its note the line of its body holding what
// was typed, or its body's first line while nothing typed is only there.
// One with no title is named by its body's first line, with nothing in
// its note. `typed` holds the words typed, and `holds(text, word)` says
// whether a text holds one as the search finds it (Fuzzy.js), so the line
// shown is the one that was found.
function named(summary, body, typed, holds) {
    const lines = String(body ?? "")
        .split("\n")
        .map((line) => line.trim())
        .filter((line) => line !== "");
    const first = lines.length > 0 ? lines[0] : "";
    const title = String(summary ?? "").trim();
    if (title === "") {
        return { name: first, note: "" };
    }
    const bodyOnly = typed.filter((word) => !holds(title, word));
    if (bodyOnly.length === 0) {
        return { name: title, note: first };
    }
    // The line holding the most of them, the first among equals.
    let best = first;
    let most = 0;
    for (const line of lines) {
        const held = bodyOnly.filter((word) => holds(line, word)).length;
        if (held > most) {
            best = line;
            most = held;
        }
    }
    return { name: title, note: best };
}

// Each row as a copy, its note (`count`) the line of its body named
// holds the words typed, worked out only once something reads it. A
// search finds every notice that holds what's typed, but the main search
// shows only its best, the rest behind one row, so lining each at once
// spent 40 to 90 ms of a letter over 800 notices, more than all else the
// search does. Anything that reads every found row's note, a sort by it
// or a copy of them all, gives that back, so read only the rows shown.
// Each is a copy, so a search or a program's row never
// changes the note of a row shown elsewhere, its fields read from the
// row's getters: copying their descriptors instead, to leave them
// unread, took three to four times as long in Qt's engine. Its note is
// made a getter before any field is copied, never after: Qt 6.12 keeps
// the slot of a field's setter once for every shape grown from the one
// it was first turned in, so turning the row's own `count` into a
// getter wrote that setter over another field, or past the copy's end,
// which crashed the shell at 11:09 on 2026-11-11.
function lined(rows, typed, holds) {
    return rows.map((row) => {
        const copy = {};
        let note = null;
        Object.defineProperty(copy, "count", {
            enumerable: true,
            configurable: true,
            get: () => {
                if (note === null) {
                    note = named(row.title, row.wrote, typed, holds).note;
                }
                return note;
            },
            set: (said) => {
                note = said;
            },
        });
        for (const field of Object.keys(row)) {
            if (field !== "count") {
                copy[field] = row[field];
            }
        }
        return copy;
    });
}

// Its drawer's facts, each a label and a value: when it arrived, how it
// left, and the urgency its sender asked. Arrived goes where Left would
// say the same time, "put away at 10:51" then standing for both (VKCF3Z,
// 2026-11-08). Each is given as `{ label, said }`, Left's said holding
// `{at}` where it tells a time, and the times as the drawer writes them.
function facts(arrived, left, urgency) {
    const out = [];
    const leftSays = left.said.replace("{at}", left.at);
    const sameTime = left.said.includes("{at}") && left.at === arrived.at;
    if (arrived.at !== "" && !sameTime) {
        out.push({ label: arrived.label, value: arrived.at });
    }
    if (leftSays !== "") {
        out.push({ label: left.label, value: leftSays });
    }
    if (urgency.said !== "") {
        out.push({ label: urgency.label, value: urgency.said });
    }
    return out;
}
