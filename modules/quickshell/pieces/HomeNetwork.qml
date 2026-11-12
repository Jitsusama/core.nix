// The home network, as the eero daemon keeps it (Eero.qml): true, in the
// warning colour, only while something ails it, the internet out from
// home, the provider down, the nodes' network ailing or a node off line
// (HomeNetwork.js), so it's in the margin and Now only then and says
// what, worst first. Its drawer is the Home Network's internet row's, as
// that place draws it. The daemon sends each change as it reads one, and
// this machine's own look every 10 seconds while eero's cloud is out of
// reach, so nothing is asked.
import qs
import QtQuick
import "HomeNetwork.js" as HomeNetwork

Piece {
    readonly property var said: Words.of("on:home-network").said ?? {}
    readonly property var ails: HomeNetwork.ails(Eero.model, said)

    drawnBy: ({ place: "eero", key: "eero:internet" })

    present: Eero.present
    active: ails.length > 0
    role: active ? "warning" : "text"
    glance: active ? ails[0] : ""
    value: active ? ails.join(" \u00b7 ") : said.well ?? ""
    title: Words.name("eero")
}
