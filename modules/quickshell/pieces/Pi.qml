// pi at a glance: how many sessions run and how many are working, for the
// launcher's foot, where the time and the battery are; and while any wants
// Joel (Sessions.qml: a question, an approval, or a run stuck without him),
// a way to each one's window in What's On. The margin says a want by For
// You's tray (ForYou.qml), which holds it.
// Now has no row of its own for it: For You lists each session that wants
// Joel, as what it asks (launcher/Now.qml's wantRow). A session stopped on
// his keys, the newest the bell rang for (Sessions.qml's ringing), is said
// as a notice too, Go To its window first, so Super+N and Return take him
// to it; it's taken back once he's there or the want ends, so it never
// takes him to a session that has moved on (WCJQSG's pivot, 2026-11-12).
// With no pi running there's nothing to show.
import qs
import QtQuick

Piece {
    readonly property var wanting: Sessions.wanting
    readonly property var first: wanting.length > 0 ? wanting[0] : null
    readonly property var ringing: Sessions.ringing

    // A parked one wanting Joel is shown too, with none running.
    present: Sessions.running.length > 0 || wanting.length > 0
    active: wanting.length > 0
    wants: active
    // The harness's own notices are pi's, so the mark waits while one of
    // its cards is up.
    app: "pi"
    // A program's, as the agents' glyph is on their notices and in Agents.
    hue: "program"
    // In the vocabulary's words, "needs you" as everywhere Joel is waited on.
    readonly property var said: Words.of("agents").said ?? {}
    function counted(key, count) {
        return String(said[key] ?? "").replace("{count}", count);
    }
    readonly property string needing: wanting.length === 0 ? ""
        : wanting.length === 1 ? String(said.needsCount ?? "")
        : counted("needCount", wanting.length)
    readonly property string working: Sessions.working > 0
        ? counted("workingCount", Sessions.working) : ""
    value: [counted("running", Sessions.running.length), working, needing]
        .filter(part => part !== "").join(", ")
    // What's moving: those needing Joel, else those working, else the
    // glyph alone while sessions sit idle.
    glance: needing !== "" ? needing : working
    title: "pi"
    hint: !first ? value : `${first.title} ${Sessions.wanted(first.presence.wants)}`
    context: !first ? "" : wanting.length === 1 ? (first.presence.wants.said ?? "")
        : `and ${wanting.length - 1} more`
    // Each one's window, the oldest want first, in What's On.
    function going(session) {
        return {
            glyph: Theme.glyph.windows,
            // Go To, where it goes at its side, as going to anything open
            // is said everywhere, so the word never changes shape.
            name: Words.name("verb:agent:switch"),
            verb: Words.verb("verb:agent:switch"),
            side: session.title,
            // One parked is woken first, as its row in For You wakes it.
            run: () => {
                const resting = Sessions.parkedAs(session.address);
                if (resting)
                    Sessions.wake(resting);
                Sessions.goTo(session);
            }
        };
    }
    readonly property var every: wanting.map(session => going(session))
    // The one rung for leads, so its notice's first act goes to it.
    actions: ringing === null ? every.slice(0, 3)
        : [going(ringing)].concat(wanting.filter(session => session !== ringing)
            .map(session => going(session))).slice(0, 3)

    // Its notice says the one rung for, by its session and what it asks,
    // once for each, and goes as that one stops ringing.
    arrives: ringing !== null
    withdraws: true
    telling: ringing === null ? "" : Sessions.key(ringing)
    summary: ringing === null ? hint : `${ringing.title} ${Sessions.wanted(ringing.presence.wants)}`
    message: ringing === null ? context : String(ringing.presence.wants.said ?? "")
}
