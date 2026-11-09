// For You's in-tray in the margin, the one mark for what waits on Joel:
// true while For You holds a session's want, a to-do, a look, a landing to
// try or a critical notice unread, and nothing while it holds none, so a calm margin has no empty
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
        notices: Notices.history,
        // The notices with a card up, those still open and not resting.
        onScreen: Object.keys(Notices.live).filter(id => !(id in Notices.resting))
    })
    readonly property var holds: Holding.held(sources)
    readonly property int count: Holding.count(holds)

    // Set together as what it holds changes, whose card says it first, so
    // the margin never reads it lit before it knows to wait for that card:
    // two bindings would come true in whichever order Qt notifies them, and
    // lit with no card named it shows at once, beside the card (Margin.qml's
    // waits). While all it holds is sessions wanting Joel, pi's card says it.
    // Counted here, not read from count, which Qt may not have made again yet.
    function hold() {
        const any = Holding.count(holds) > 0;
        app = Holding.saidBy(sources, holds);
        wants = any;
        active = any;
    }
    onHoldsChanged: hold()
    Component.onCompleted: hold()
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
