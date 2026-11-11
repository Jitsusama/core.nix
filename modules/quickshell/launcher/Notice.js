// What the shell still works out of a notice itself: what its sender
// offers on its card, its sender's own Read, and the line of what it
// said that holds what was typed, as a search lists it. Its rows, their
// drawers and their acts are the notices daemon's (modules/notices'
// row.rs). Apart from the shell, so tests/notice.qml can hold it in Qt's
// own engine.
.pragma library

// What a notice offers, from its sender's actions: every one but the one
// that opens what it's about, each its identifier and text.
function offers(actions) {
    return actions
        .filter((action) => action.identifier !== "default")
        .map((action) => ({ identifier: action.identifier, text: action.text }));
}

// Its sender's own Read, where it offers one among its actions, by its
// identifier or its words, else null: what d sends at once as it's done
// with a notice on its card.
const READ_ACT = /^(mark[-_ ]?(as[-_ ]?)?)?read$/i;
function readOffer(actions) {
    return actions.find((action) => READ_ACT.test(action.identifier) || READ_ACT.test(action.text))
        ?? null;
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
