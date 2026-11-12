.pragma library
// What pi's sessions landed, as Agents' Landed shows it (VKCF3Z,
// 2026-11-06): which of the landings wait in For You to try until Joel
// reads them and which he has, how Landed's row stands, and each one's
// drawer and when it landed, as words. Which landings are worth his
// reading, whose each is, and whether he has read it are the agents
// daemon's (modules/agents/src/landed.rs), which serves them ready; what
// is left here is saying them. Said apart from Agents.qml so
// tests/session-row.qml holds it in Qt's own engine.

// A change as a landing names it, `repo#number`, never a quest document's
// item, `PLAN-20261106-2ZPAI5#14`, which its drawer names apart.
const CHANGE = /^[\w.-]+#\d+$/;
const DOCUMENT = /^[A-Z]+-\d{8}-[0-9A-Z]{6}(#\d+)?$/;

function isChange(ref) {
    return CHANGE.test(ref) && !DOCUMENT.test(ref);
}

// Those of `rows` that wait in For You's To Try: each whose line asks
// Joel to try something, as the daemon says (`tries`), while he hasn't
// read it (`read` null). One whose line asks nothing of him, as "Nothing
// to try", or with none, is only Landed's to list (VKCF3Z and Y2YBTS,
// 2026-11-10; VKCF3Z, 2026-11-11).
function toTry(rows) {
    return rows.filter(one => one.tries === true && one.read === null);
}

// Those that waited there and he has read since, for For You's History,
// each { one, at }, `at` when he read it, in milliseconds. One read before
// the shell kept when (`read` true) has no time to say, so it isn't
// History's.
function triedRead(rows) {
    return rows.filter(one => one.tries === true && typeof one.read === "number")
        .map(one => ({ one, at: one.read }));
}

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

// A landing's drawer, the same in Landed and in For You: who landed it and
// that it's a landing as its eyebrow, where and what kind with no state;
// what landed as its title; when it landed, its session, quest, repository
// and machine as facts, its ref last, as identifiers go; and what to try
// as its words, since a sentence never fits beside the row (VKCF3Z,
// 2026-11-10). `said` gives the kind (`kind`), when (`when`), the
// session's title and how a fact is named; what isn't known isn't said.
function landingDrawer(one, said) {
    const eyebrow = [one.session.address, said.kind]
        .filter(part => (part ?? "") !== "").join(" \u00b7 ");
    const facts = [
        { label: said.named("landed"), value: said.when },
        { label: said.named("session"), value: said.session },
        { label: said.named("task"), value: one.quest },
        { label: said.named("repo"), value: one.repo },
        { label: said.named("machine"), value: one.machine },
        { label: said.named(isChange(one.ref) ? "change" : "item"), value: one.ref }
    ].filter(fact => (fact.value ?? "") !== "");
    return { eyebrow, eyebrowNames: true, title: one.what, facts, body: one.person };
}

// When a landing landed, as its row says it at its side and its drawer as
// a fact: the side its time today, "11:19", and before today its day as
// When says one, "yesterday", "Saturday", "7 Nov", since a row's side
// says why it's there; the fact the whole moment, "Saturday 7 November,
// 11:19". `said` gives `clock(at)`, `day(at)`, When's, and `date(at)`,
// the full day; a landing with no time says none.
// A landing's side alone, as landedWhen says it, kept in `sides` by its
// moment for the day `day` names, so Landed's thousand rows made again at
// each opening say each moment once a day. `sides` is { day, sides }, and
// starts again once `day` turns, when today's time may read as yesterday.
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

function landedWhen(at, said) {
    const time = Date.parse(String(at ?? ""));
    if (Number.isNaN(time)) {
        return { side: "", fact: "" };
    }
    const day = said.day(time);
    return {
        side: day === "" ? said.clock(time) : day,
        fact: `${said.date(time)}, ${said.clock(time)}`
    };
}
