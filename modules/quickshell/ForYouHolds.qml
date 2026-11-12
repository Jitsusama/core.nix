pragma Singleton
// What For You holds that waits on Joel, counted once (pieces/Holding.js)
// for the two that say it: For You's own row (launcher/Now.qml's
// waitsNow) and the margin's tray (pieces/ForYou.qml), so they never say
// different numbers (Y2YBTS, 2026-11-09). The sessions, the to-dos and
// the notices it hears itself. The looks it hears from the to-do daemon's
// stream (modules/to-do), so the tray lights for one left while the
// launcher is closed, as WCJQSG asked (2026-11-09): one Rust daemon per
// domain, heard, never a folder watched in the shell. Where the daemon
// isn't there, as on a machine without it, the looks are Now's, as it
// last listed their folder while open. What the launcher's sources give,
// Now sets as it reads them, since only it holds those sources.
import qs
import Quickshell
import QtQuick
import "launcher/Looks.js" as Looks
import "pieces/Holding.js" as Holding

Singleton {
    // What the agents left Joel to look at that waits, and what the
    // launcher's other sources give him to do, as Now last read them.
    property var looks: []
    property var given: []

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
    // The looks that wait, as the stream says them, weighed as For You
    // weighs the folder's, or Now's while the daemon hasn't answered. A
    // daemon that stopped answering keeps what it last held, so that's
    // set aside for Now's reading rather than counted as current: the tray
    // fails to the folder, never to stale looks (Y2YBTS, as in #2707). One
    // that has waited three days goes quiet as the stream next changes,
    // which is many times an hour, so nothing wakes to time it.
    readonly property bool streamed: toDo.answered && toDo.state !== "not answering"
    readonly property var waitingLooks: {
        void toDo.revision;
        return streamed ? Looks.looks(Holding.lookFiles(toDo.held), {}, Date.now()).waiting : looks;
    }

    readonly property var holds: Holding.held({
        wanting: Sessions.wanting,
        toDo: WaitingOnYou.now,
        looks: waitingLooks,
        given: given,
        toReopen: Sessions.toReopen,
        notices: Notices.history,
        // The notices with a card up, those still open and not resting.
        onScreen: Object.keys(Notices.live).filter(id => !(id in Notices.resting))
    })
    readonly property int count: Holding.count(holds)
}
