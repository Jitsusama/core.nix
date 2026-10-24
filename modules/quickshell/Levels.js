// How far a notice reaches, by the level its sender is kept to
// (jitsusama.quickshell.notices.senders), worked out apart from the
// notices so tests/levels.qml can hold it in Qt's own engine.
.pragma library

// From the least reach to the most: kept in the history alone, counted
// as waiting in the margin but never shown, shown as a card, or shown as
// a card that stays until it's answered.
const LEVELS = ["history", "count", "card", "urgent"];

// The level a notice gets: what it asks for, a card or, critical, an
// urgent one, but never past its sender's level. A sender with no level,
// or one the shell doesn't know, reaches as far as it asks.
function reached(level, critical) {
    const asked = LEVELS.indexOf(critical ? "urgent" : "card");
    const kept = LEVELS.indexOf(level);
    return LEVELS[kept < 0 ? asked : Math.min(asked, kept)];
}

// What a level does: whether the card shows, whether it's counted as
// waiting once it's gone unseen, and whether it stays until answered.
function reach(level) {
    const at = LEVELS.indexOf(level);
    return {
        shown: at >= LEVELS.indexOf("card"),
        counted: at >= LEVELS.indexOf("count"),
        urgent: level === "urgent"
    };
}
