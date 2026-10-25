// pi at a glance: how many sessions run and how many are working, for the
// launcher's foot, where the time and the battery are; and a mark in the
// margin while any wants Joel (Sessions.qml: a question, an approval, or a
// run stuck without him), What's on saying which and what, with a way to
// each one's window. With no pi running there's nothing to show.
import qs
import QtQuick

Piece {
    readonly property var wanting: Sessions.wanting
    readonly property var first: wanting.length > 0 ? wanting[0] : null

    present: Sessions.running.length > 0
    active: wanting.length > 0
    wants: active
    // Its want is said as a notice of pi's (Sessions.qml) while it's new.
    app: "pi"
    // A program's, as the agents' glyph is on their notices and in Agents.
    hue: "program"
    value: [`${Sessions.running.length} running`,
        Sessions.working > 0 ? `${Sessions.working} working` : "",
        wanting.length > 0 ? `${wanting.length} ${wanting.length === 1 ? "wants" : "want"} you` : ""]
        .filter(part => part !== "").join(", ")
    // What's moving: those wanting Joel, else those working, else the
    // glyph alone while sessions sit idle.
    glance: wanting.length > 0 ? `${wanting.length} ${wanting.length === 1 ? "wants" : "want"} you`
        : Sessions.working > 0 ? `${Sessions.working} working` : ""
    title: "pi"
    hint: !first ? value : `${first.title} ${Sessions.wanted(first.presence.wants)}`
    context: !first ? "" : wanting.length === 1 ? (first.presence.wants.said ?? "")
        : `and ${wanting.length - 1} more`
    // Each one's window, the oldest want first, as many as fit.
    actions: wanting.slice(0, 3).map(session => ({
        glyph: Theme.glyph.windows,
        name: `Go to ${session.title}`,
        run: () => Niri.act({ FocusWindow: { id: session.window.id } })
    }))
}
