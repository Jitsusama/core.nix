// The link to work, as the warp daemon tells how WARP stands
// (WarpState.qml): true, in the warning colour, only while something ails
// it, down, degraded, connecting for long or WARP not answering
// (WorkNetwork.js), so it's in the margin and Now only then, and gone as
// soon as the daemon tells it's well again. Return steps into Network
// onto the Work Network's row, whose drawer says every figure WARP gives
// and whose parts step on into each (Joel, 2026-11-15: "someway to
// engage, link into more details"); the Work Network is an answer rather
// than a place's rows, so there's no drawer to borrow. Joel asked for the work network beside
// the home network (2026-11-11, "should we do the same for WARP and call
// it a work network"), and VKCF3Z's design has one Now line while it
// ails (123f40bf).
import qs
import QtQuick
import "WorkNetwork.js" as WorkNetwork

Piece {
    id: piece

    readonly property var said: Words.of("on:work-network").said ?? {}
    readonly property var tells: Words.of("on:work-network").tells ?? {}
    // The time it's judged at, which moves only while how long it has
    // stood decides whether it ails, as connecting does.
    property real judgedAt: Date.now()
    readonly property var ails: WorkNetwork.ails(WarpState.state, said, judgedAt,
        at => Qt.formatDateTime(new Date(at), "HH:mm"))

    present: WarpState.present
    active: ails.length > 0
    role: active ? "warning" : "text"
    glance: active ? ails[0] : ""
    value: active ? ails[0] : said.well ?? ""
    // WARP's evidence, the hour and what lies behind a degraded link, with
    // what Joel can do, beneath Now's line (WorkNetwork.js).
    context: active ? WorkNetwork.context(WarpState.state, said, tells) : ""
    title: Words.name("warp")
    actions: [{
        glyph: Words.glyph("warp"),
        name: Words.name("warp"),
        verb: "show",
        // Network is a place, not a collection, so it's walked into from
        // home as Machines' rows walk into Computer (goTo): opened into by
        // name as a collection is, it found none, and Return left Joel at
        // home (2026-11-16).
        run: shell => shell.launcher.goTo({ trail: ["category:network"], spot: "answer:warp" })
    }]
    // Said as it starts or stops ailing, so a test sees the piece apart
    // from what's made of it.
    onActiveChanged: Report.say(`work network ails ${active}`)

    Timer {
        interval: 5000
        repeat: true
        running: WorkNetwork.timed(WarpState.state)
        onRunningChanged: piece.judgedAt = Date.now()
        onTriggered: piece.judgedAt = Date.now()
    }
}
