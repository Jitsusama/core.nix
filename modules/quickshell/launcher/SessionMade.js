.pragma library
// What a pi session's row in Agents is made from, so Agents keeps the row
// while it reads the same and makes it anew once anything in it changes
// (Agents.qml's made), and a change remakes only the sessions it changed.
// tests/session-row.qml holds it in Qt's own engine.

// The text of what a session's row is made from: every field of the
// session the row and its acts read, the window Go To focuses, the kitty
// window, held id and attach words it goes to without one (Sessions.qml's
// goTo), its presence, and `around`, what's read beside it, as how it
// stands and what it spent. A row kept past a change to any of them would
// show, or go to, what the session was.
function madeFrom(session, around) {
    return JSON.stringify([session.address, session.title, session.folder, session.machine,
        session.here, session.reachable, session.window?.id ?? null, session.kitty ?? null,
        session.held ?? null, session.attach ?? null, session.presence ?? null, around]);
}
