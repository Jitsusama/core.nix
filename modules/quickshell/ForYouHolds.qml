pragma Singleton
// What For You holds that waits on Joel, counted once (pieces/Holding.js)
// for the two that say it: For You's own row (launcher/Now.qml's
// waitsNow) and the margin's tray (pieces/ForYou.qml), so they never say
// different numbers (Y2YBTS, 2026-11-09). The sessions, the to-dos and
// the notices it hears itself; the looks and what the launcher's sources
// give, Now sets as it reads them, since only it lists them, while the
// launcher is open, and what it read last stands while it's closed.
import qs
import Quickshell
import QtQuick
import "pieces/Holding.js" as Holding

Singleton {
    // What the agents left Joel to look at that waits, and what the
    // launcher's other sources give him to do, as Now last read them.
    property var looks: []
    property var given: []
    readonly property var holds: Holding.held({
        wanting: Sessions.wanting,
        toDo: WaitingOnYou.now,
        looks: looks,
        given: given,
        notices: Notices.history,
        // The notices with a card up, those still open and not resting.
        onScreen: Object.keys(Notices.live).filter(id => !(id in Notices.resting))
    })
    readonly property int count: Holding.count(holds)
}
