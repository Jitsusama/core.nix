// What the places that paste through the clipboard's history (Copies,
// Emoji) say of it, on their own rows and on their heads while entered, as
// People says its daemon is down: once, where a place says how it stands,
// rather than beside every copy, each of which keeps its own note (VKCF3Z,
// 2026-11-10). Worked out here so tests/pasting.qml proves it in Qt's own
// engine; Pasting.qml holds the times it's asked with.
.pragma library

// How long the history may be silent before it's said, in milliseconds: a
// restart or a reconnect answers within it, and saying so at once would
// blink the count away and back.
var QUIET = 1000;

// How long a paste that never landed is said after it, in milliseconds,
// so the place it came from says why when it's next opened, and stops.
var MISSED = 60000;

// What's said, from how the history stands: whether one is kept, when it
// went silent (null while it answers) and when a paste it was asked for
// last never landed (null when none has). "" says nothing, and the place
// counts what it holds as ever; a shell keeping no history says nothing,
// its paste through Qt and wtype being how it always pastes.
function says(history, now, said) {
    if (!history.kept)
        return "";
    if (history.lostAt !== null && now - history.lostAt >= QUIET)
        return said.silent ?? "";
    if (history.missedAt !== null && now - history.missedAt < MISSED)
        return said.unpasted ?? "";
    return "";
}
