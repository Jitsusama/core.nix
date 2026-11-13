pragma Singleton
// What For You holds that waits on Joel, counted once (pieces/Holding.js)
// for the two that say it: For You's own row (launcher/Now.qml's
// waitsNow) and the margin's tray (pieces/ForYou.qml), so they never say
// different numbers (Y2YBTS, 2026-11-09). The sessions, the to-dos and
// the notices it hears itself. The looks are the to-do daemon's stream
// (modules/to-do), which it holds here, so the tray lights for one left
// while the launcher is closed, as WCJQSG asked (2026-11-09): one Rust
// daemon per domain, heard, never a folder watched in the shell. Now
// takes them from it, with Joel's acts laid over them until the daemon
// says them too, and sets those that wait here, as it sets what the
// launcher's sources give, since only it holds those.
import qs
import Quickshell
import QtQuick
import "pieces/Holding.js" as Holding

Singleton {
    // What the agents left Joel to look at that waits, and what the
    // launcher's other sources give him to do, as Now last took them.
    property var looks: []
    property var given: []
    // The looks' stream, which Now takes them from and asks to settle one.
    readonly property var lookStream: toDo

    // The to-do daemon's looks, each a look file whole, watched while the
    // shell runs; a change to the looks folder is told as it happens, so
    // nothing is polled and nothing wakes while none changes.
    Daemon {
        id: toDo

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/to-do.sock`
        kind: "look"
        // Its watch is every look there is, so one held that it no longer
        // says went while the shell wasn't hearing.
        answersWhole: true
    }
    readonly property var holds: Holding.held({
        wanting: Sessions.wanting,
        toDo: WaitingOnYou.now,
        looks: looks,
        given: given,
        toReopen: Sessions.toReopen,
        notices: Notices.history,
        // The notices with a card up, those still open and not resting.
        onScreen: Object.keys(Notices.live).filter(id => !(id in Notices.resting))
    })
    readonly property int count: Holding.count(holds)
}
