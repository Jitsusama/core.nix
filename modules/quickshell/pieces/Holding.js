// What For You holds that waits on Joel, counted by kind from each source
// as it stands, for the margin's one mark (pieces/ForYou.qml): lit while
// any waits, and nothing while none does (Joel's pick, VKCF3Z,
// 2026-11-09). It's a function of the sources alone, so For You itself
// (launcher/Now.qml) can count from the same one, and tests/marks.qml
// holds it in Qt's own engine.
.pragma library

// What For You holds, { do, look, try, notice }, from `sources`, each a
// list and any left out holding nothing: `wanting`, the sessions that
// want Joel, and `toDo`, what the agents wait on him for that he can do
// now, are to-dos; `looks`, what they left him to look at, counts while
// it waits; `tries`, what's landed for him to try; and `notices`, the
// notices kept, of which one counts while it's critical, unread and off
// the screen, its id not among `onScreen`'s. For You's Desktop holds a
// notice until it's read, and of those only a critical one waits on Joel,
// the rest being what came lately; on screen its card says it, as a card
// or a mark, never both; and one saying a to-do (`waits`) is that to-do,
// counted once among them.
function held(sources) {
    const given = sources ?? {};
    const each = name => Array.isArray(given[name]) ? given[name] : [];
    const shown = each("onScreen").map(String);
    return {
        do: each("wanting").length + each("toDo").length,
        look: each("looks").filter(one => one?.state === "waiting").length,
        try: each("tries").length,
        notice: each("notices").filter(one => one?.urgency === "critical" && one.read !== true
            && !one.waits && !shown.includes(String(one.id))).length
    };
}

// How many things For You holds, of every kind.
function count(holds) {
    return holds.do + holds.look + holds.try + holds.notice;
}

// The program whose card says all of what's held, so the mark waits for
// that card to go, as a card or a mark, never both (Margin.qml): pi's,
// while all of it is sessions wanting Joel; none otherwise, since no one
// card says the rest.
function saidBy(sources, holds) {
    const wanting = Array.isArray(sources?.wanting) ? sources.wanting.length : 0;
    return wanting > 0 && wanting === count(holds) ? "pi" : "";
}
