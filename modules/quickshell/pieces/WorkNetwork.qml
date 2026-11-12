// The link to work, as the warp daemon tells how WARP stands
// (WarpState.qml): true, in the warning colour, only while something ails
// it, down, degraded, connecting for long or WARP not answering
// (WorkNetwork.js), so it's in the margin and Now only then, and gone as
// soon as the daemon tells it's well again. It has no drawer yet: the
// Work Network is an answer rather than a place's rows, so there's none
// to borrow, and typing work steps in. Joel asked for the work network beside
// the home network (2026-11-11, "should we do the same for WARP and call
// it a work network"), and VKCF3Z's design has one Now line while it
// ails (123f40bf).
import qs
import QtQuick
import "WorkNetwork.js" as WorkNetwork

Piece {
    id: piece

    readonly property var said: Words.of("on:work-network").said ?? {}
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
    title: Words.name("warp")

    Timer {
        interval: 5000
        repeat: true
        running: WorkNetwork.timed(WarpState.state)
        onRunningChanged: piece.judgedAt = Date.now()
        onTriggered: piece.judgedAt = Date.now()
    }
}
