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

// The session a kept landing names, by its own id, among those there are,
// or one standing for it, by its address, once it has ended.
function lander(sessions, one) {
    const id = String(one.session ?? "");
    return sessions.find(session => session.presence?.session === id)
        ?? { address: String(one.address ?? ""), title: "", presence: { session: id } };
}

// What sessions landed, newest first, each worth Joel's reading: those the
// harness keeps (`kept`, the node's `landing` records), which outlive
// their session, and those the sessions there are still carry from before
// the harness kept them, each once. Each is { key, at, ref, what, person,
// url, quest, repo, machine, session }, `url` only a web link.
function all(kept, sessions) {
    const rows = kept.filter(worthReading).map(one => landingOf(lander(sessions, one), one));
    const keys = rows.map(one => one.key);
    for (const session of sessions) {
        for (const one of (session.presence?.landings ?? []).filter(worthReading)) {
            const row = landingOf(session, one);
            if (!keys.includes(row.key)) {
                rows.push(row);
                keys.push(row.key);
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
function landedToday(kept, now) {
    const today = new Date(now).toDateString();
    return kept.filter(one => worthReading(one)
        && new Date(Date.parse(String(one.at ?? ""))).toDateString() === today).length;
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

// A landing's drawer: who landed it and when as its eyebrow, what landed
// as its title, its ref, session, quest, repository and machine as facts,
// and what to try as its words, since a sentence never fits beside the
// row's ref. `said` gives when (`at`), the session's title and how a fact
// is named; what isn't known isn't said.
function landingDrawer(one, said) {
    const eyebrow = [one.session.address, said.at]
        .filter(part => (part ?? "") !== "").join(" \u00b7 ");
    const facts = [
        { label: said.named("landed"), value: one.ref },
        { label: said.named("session"), value: said.session },
        { label: said.named("task"), value: one.quest },
        { label: said.named("repo"), value: one.repo },
        { label: said.named("machine"), value: one.machine }
    ].filter(fact => (fact.value ?? "") !== "");
    return { eyebrow, eyebrowNames: true, title: one.what, facts, body: one.person };
}

// What of Joel's reading is still worth keeping: the keys of landings
// still kept or that a session still holds, so what's kept never grows
// past what's there.
function stillRead(kept, sessions, read) {
    const held = kept.map(one => landingKey(lander(sessions, one), one)).concat(
        ...sessions.map(session => (session.presence?.landings ?? [])
            .map(one => landingKey(session, one))));
    return read.filter(key => held.includes(key));
}
