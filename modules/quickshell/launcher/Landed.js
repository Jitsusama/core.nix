.pragma library
// What pi's sessions landed, as Agents' Landed shows it (VKCF3Z,
// 2026-11-06): how Landed's row stands, and when each landed, as words.
// Which landings are worth his reading, whose each is, whether he has
// read it, which wait in For You to try and each one's drawer are the
// agents daemon's (modules/agents/src/landed.rs), which serves them
// ready; what is left here is saying their times. Said apart from
// Agents.qml so tests/session-row.qml holds it in Qt's own engine.

// How Landed stands, said at its row's right: how many landed today, "5
// today", or "none today". What's still to read is For You's alone to
// count, since saying it here too would be a second inbox (VKCF3Z,
// 2026-11-10); nothing after its name, since a place's row says nothing
// there (VKCF3Z, adba1297). `said` gives the words, `today` holding
// `{count}`, and `none`.
function stands(today, said) {
    return today > 0 ? String(said.today ?? "").replace("{count}", today)
        : String(said.none ?? "");
}

// When a landing landed, as its row says it at its side: its time today,
// "11:19", and before today its day as When says one, "yesterday",
// "Saturday", "7 Nov", since a row's side says why it's there. `said`
// gives `clock(at)` and `day(at)`, When's; a landing with no time says
// none. It's kept in `sides` by its moment for the day `day` names, so
// Landed's thousand rows made again at each opening say each moment once
// a day. `sides` is { day, sides }, and starts again once `day` turns,
// when today's time may read as yesterday.
function landedSide(sides, at, day, said) {
    if (sides.day !== day) {
        sides.day = day;
        sides.sides = new Map();
    }
    const moment = String(at ?? "");
    if (!sides.sides.has(moment)) {
        const time = Date.parse(moment);
        const dayOf = Number.isNaN(time) ? "" : said.day(time);
        sides.sides.set(moment, Number.isNaN(time) ? "" : dayOf === "" ? said.clock(time) : dayOf);
    }
    return sides.sides.get(moment);
}
