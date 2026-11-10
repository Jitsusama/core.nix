.pragma library
.import "Order.js" as Order
// What pi's sessions landed, as Agents' Landed lists it (VKCF3Z,
// 2026-11-06): which landings are worth Joel's reading, newest first,
// which of them wait in For You to try until he reads them and which he
// has, each one's drawer, and what of his reading is still worth keeping.
// A landing outlives its session: the harness keeps each 30 days (harness
// #1049), which the realm's node serves as its `landing` kind. Said
// apart from Agents.qml so tests/session-row.qml holds it in Qt's own
// engine.

// A change as a landing names it, `repo#number`, never a quest document's
// item, `PLAN-20261106-2ZPAI5#14`, and the only link a landing may open,
// one on the web (the harness's landings.ts).
const CHANGE = /^[\w.-]+#\d+$/;
const DOCUMENT = /^[A-Z]+-\d{8}-[0-9A-Z]{6}(#\d+)?$/;
const WEB = /^https:\/\//;

function isChange(ref) {
    return CHANGE.test(ref) && !DOCUMENT.test(ref);
}

// A landing's key, which Joel's reading it is kept by: its session's, its
// ref and when it landed, so the same change landed again reads anew.
function landingKey(session, one) {
    return `${session.presence?.session ?? session.address} ${one.ref} ${one.at}`;
}

// Whether a landing is worth Joel's reading: a change merged, or anything
// else that says what he should try (`person`). A plan item ticked, which
// a session lands too, is its own progress, 27 of the 77 landings on
// 2026-11-07, so it's left out unless it says what to try.
function worthReading(one) {
    return isChange(String(one.ref ?? "")) || String(one.person ?? "") !== "";
}

// One landing as Landed lists it, by the session that landed it.
function landingOf(session, one) {
    return {
        key: landingKey(session, one),
        at: String(one.at ?? ""),
        ref: String(one.ref),
        what: String(one.what || one.ref),
        person: String(one.person ?? ""),
        url: WEB.test(String(one.url ?? "")) ? String(one.url) : "",
        quest: String(one.quest ?? ""),
        repo: String(one.repo ?? ""),
        machine: String(one.machine ?? ""),
        session
    };
}

// The sessions by their own ids, the first of each, so a thousand kept
// landings find theirs among hundreds of sessions without a search each.
function byId(sessions) {
    const found = new Map();
    for (const session of sessions) {
        const id = session.presence?.session;
        if (typeof id === "string" && !found.has(id)) {
            found.set(id, session);
        }
    }
    return found;
}

// The session a kept landing names, by its own id, among those there are
// (`found`, by byId), or one standing for it, by its address, once it has
// ended.
function lander(found, one) {
    const id = String(one.session ?? "");
    return found.get(id)
        ?? { address: String(one.address ?? ""), title: "", presence: { session: id } };
}

// What sessions landed, newest first, each worth Joel's reading: those the
// harness keeps (`kept`, the node's `landing` records), which outlive
// their session, and those the sessions there are still carry from before
// the harness kept them, each once. Each is { key, at, ref, what, person,
// url, quest, repo, machine, session }, `url` only a web link.
function all(kept, sessions) {
    const found = byId(sessions);
    const rows = kept.filter(worthReading).map(one => landingOf(lander(found, one), one));
    const keys = new Set(rows.map(one => one.key));
    for (const session of sessions) {
        for (const one of (session.presence?.landings ?? []).filter(worthReading)) {
            const row = landingOf(session, one);
            if (!keys.has(row.key)) {
                rows.push(row);
                keys.add(row.key);
            }
        }
    }
    return Order.sorted(rows, Order.stream(one => one.at));
}

// Those of `rows` that wait in For You's To Try: each saying what Joel
// should try, while he hasn't read it (`read`, the keys he has). One
// without a line to try is only Landed's to list, and asks nothing of him
// (VKCF3Z and Y2YBTS, 2026-11-10).
function toTry(rows, read) {
    return rows.filter(one => one.person !== "" && !read.includes(one.key));
}

// Those that waited there and he has read since, for For You's History,
// each { one, at }, `at` when he read it (`readAt`, by key, in
// milliseconds). One read before the shell kept when has no time to say,
// so it isn't History's.
function triedRead(rows, readAt) {
    return rows.filter(one => one.person !== "" && typeof readAt[one.key] === "number")
        .map(one => ({ one, at: readAt[one.key] }));
}

// How many landings worth reading landed today, read or not, on this
// machine's clock, `now` in milliseconds since the epoch.
// Today runs from one midnight to the next, found once, since saying each
// of over a thousand landings' days to compare them was most of Landed's
// row at every opening (H6EDLC's profile, 2026-11-10).
function landedToday(kept, now) {
    const at = new Date(now);
    const begun = new Date(at.getFullYear(), at.getMonth(), at.getDate()).getTime();
    const ends = new Date(at.getFullYear(), at.getMonth(), at.getDate() + 1).getTime();
    return kept.filter(one => {
        const time = Date.parse(String(one.at ?? ""));
        return time >= begun && time < ends && worthReading(one);
    }).length;
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

// What of Joel's reading is still worth keeping: the keys of landings
// still kept or that a session still holds, so what's kept never grows
// past what's there.
function stillRead(kept, sessions, read) {
    const found = byId(sessions);
    const held = new Set(kept.map(one => landingKey(lander(found, one), one)).concat(
        ...sessions.map(session => (session.presence?.landings ?? [])
            .map(one => landingKey(session, one)))));
    return read.filter(key => held.has(key));
}
