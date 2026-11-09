// For You's in-tray in the margin, the one mark for what waits on Joel:
// true while For You holds a to-do, a look, a landing to try or a critical
// notice, and nothing while it holds none, so a calm margin has no empty
// tray (Joel's pick, VKCF3Z, 2026-11-09). The bell and pi's want left the
// margin for it, since both are among what For You holds. What it holds
// is counted by Holding.js from the sources the shell already hears, so
// it costs nothing while the launcher is closed; the looks join once the
// to-do daemon streams them. Pressing it opens For You itself (`opens`).
// Now has no row of its own for it: it's For You's (launcher/Now.qml's
// above).
import qs
import QtQuick
import "Holding.js" as Holding

Piece {
    readonly property var sources: ({
        wanting: Sessions.wanting,
        toDo: WaitingOnYou.now,
        notices: Notices.waiting
    })
    readonly property var holds: Holding.held(sources)
    readonly property int count: Holding.count(holds)

    active: count > 0
    wants: active
    // While all it holds is sessions wanting Joel, pi's card says it, and
    // the mark waits for the card to go.
    app: Holding.saidBy(sources, holds)
    hue: "voice"
    opens: "forYou"
    // How many, as For You's row counts them.
    value: String(count)
    title: Words.name("on:waiting")
    actions: [{
        glyph: Words.glyph("on:waiting"),
        name: Words.name("on:waiting"),
        verb: Words.verb("on:waiting"),
        run: shell => shell.launcher.into("forYou")
    }]
}
