// Whether every word a card draws stays inside it: inside the room it's
// given, inside the card, and off its neighbours' ink (Joel at 19:00 on
// 2026-11-15: "somehting like this should never be possible, we have
// circumstances where text can escape containment"). The launcher walks
// what it draws into boxes (Launcher.qml's drawnTexts), and this says
// which of them broke out, so a check fills every surface with long words
// and asks; tests/contained.qml holds it to its cases in Qt's own engine.
.pragma library

// How far ink may stand past a line it must keep inside, in pixels: a
// glyph's bearing past its advance, and the half pixel a box is placed on.
const SLACK = 2;

// The part of two boxes both cover, or null where they meet in no more
// than a line. A box is { x, y, width, height }.
function shared(one, other) {
    const x = Math.max(one.x, other.x);
    const y = Math.max(one.y, other.y);
    const width = Math.min(one.x + one.width, other.x + other.width) - x;
    const height = Math.min(one.y + one.height, other.y + other.height) - y;
    return width > 0 && height > 0 ? { x, y, width, height } : null;
}

// Whether `inner` stands inside `outer`, but for the slack.
function inside(inner, outer) {
    return inner.x >= outer.x - SLACK && inner.y >= outer.y - SLACK
        && inner.x + inner.width <= outer.x + outer.width + SLACK
        && inner.y + inner.height <= outer.y + outer.height + SLACK;
}

// Whether one drawn text holds the other, as a field holds its hint.
function holds(one, other) {
    return other.within.includes(one.id) || one.within.includes(other.id);
}

// The words that broke out, each `{ said, fault, with }`: "past its room"
// where the ink it shows runs past the box it's laid in, so it neither
// wrapped nor was cut there; "past the card" where it runs past the card;
// "over" where its ink lies on another's, which `with` says. Each drawn
// text is `{ id, said, ink, room, within }`: its ink as it's shown, what a
// box clipping it cuts already cut; the box it's laid in; and the ids of
// the drawn texts it's laid inside. `card` is the card's own box.
function faults(drawn, card) {
    const found = [];
    for (const text of drawn) {
        if (!inside(text.ink, text.room))
            found.push({ said: text.said, fault: "past its room", with: "" });
        else if (!inside(text.ink, card))
            found.push({ said: text.said, fault: "past the card", with: "" });
    }
    for (let at = 0; at < drawn.length; at += 1) {
        for (let next = at + 1; next < drawn.length; next += 1) {
            const one = drawn[at];
            const other = drawn[next];
            const both = holds(one, other) ? null : shared(one.ink, other.ink);
            if (both !== null && both.width > SLACK && both.height > SLACK)
                found.push({ said: one.said, fault: "over", with: other.said });
        }
    }
    return found;
}
