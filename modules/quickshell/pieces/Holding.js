// What For You holds that waits on Joel, from each source as it stands:
// one count that For You's row (launcher/Now.qml's waitsNow) and the
// margin's one mark (pieces/ForYou.qml) both read through ForYouHolds.qml,
// so the two never say different numbers (Y2YBTS, 2026-11-09). The mark
// is lit while any waits, and nothing while none does (Joel's pick,
// VKCF3Z, 2026-11-09). It's a function of the sources alone, so
// tests/marks.qml holds it in Qt's own engine.
.pragma library

// The sessions wanting Joel that For You lists as rows of their own: not
// one whose want is an action, since that's one of its own items, which
// a session's presence mirrors (harness #907) and For You lists itself,
// so its session's row would say the same thing twice (2026-10-31: four
// items read as eight).
function wanting(sessions) {
    return (Array.isArray(sessions) ? sessions : [])
        .filter(session => session?.presence?.wants?.kind !== "action");
}

// What For You holds, by kind, each a list, from `sources`, any left out
// holding nothing: `wanting`, the sessions that want Joel (wanting()
// keeps those For You lists); `toDo`, what the agents wait on him for
// that he can do now; `looks`, what they left him to look at that waits;
// `given`, what the launcher's other sources give him to do (ForYou.js's
// given); and `notices`, the notices kept, of which one counts while it's
// critical, unread and off the screen, its id not among `onScreen`'s.
// For You's Desktop holds a notice until it's read, and of those only a
// critical one waits on Joel, the rest being what came lately; on screen
// its card says it, as a card or a mark, never both; and one saying a
// to-do (`waits`) is that to-do, counted once among them.
function held(sources) {
    const given = sources ?? {};
    const each = name => Array.isArray(given[name]) ? given[name] : [];
    const shown = each("onScreen").map(String);
    return {
        wanting: wanting(each("wanting")),
        toDo: each("toDo"),
        looks: each("looks"),
        given: each("given"),
        notices: each("notices").filter(one => one?.urgency === "critical"
            && one.read !== true && !one.waits && !shown.includes(String(one.id)))
    };
}

// How many things For You holds, of every kind.
function count(holds) {
    return holds.wanting.length + holds.toDo.length + holds.looks.length
        + holds.given.length + holds.notices.length;
}

// The program whose card says all of what's held, so the mark waits for
// that card to go, as a card or a mark, never both (Margin.qml): pi's,
// while all of it is sessions wanting Joel; none otherwise, since no one
// card says the rest.
function saidBy(holds) {
    const wants = holds.wanting.length;
    return wants > 0 && wants === count(holds) ? "pi" : "";
}
