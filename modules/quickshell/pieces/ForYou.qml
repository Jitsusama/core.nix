// For You's in-tray in the margin, the one mark for what waits on Joel:
// true while For You holds anything that waits on him now, a session's
// want, a to-do, a look, what a source gives him to do or a critical
// notice unread, and nothing while it holds none, so a calm margin has no
// empty tray (Joel's pick, VKCF3Z, 2026-11-09). The bell and pi's want
// left the margin for it, since both are among what For You holds. It
// reads the count For You's own row reads (ForYouHolds.qml), so the two
// say one number; landings to try join once the to-do daemon streams
// them. Pressing it opens For You itself (`opens`). Now has no row of its
// own for it: it's For You's (launcher/Now.qml's above).
import qs
import QtQuick
import "Holding.js" as Holding

Piece {
    readonly property var holds: ForYouHolds.holds

    // Set together as what it holds changes, so the margin never reads it
    // wanting and not lit, or lit and not wanting: two bindings would come
    // true in whichever order Qt notifies them. No card says what it holds,
    // since a session's want posts none (Sessions.qml), so it waits for
    // none. Counted here, not read from the count, which Qt may not have
    // made again yet.
    function hold() {
        const any = Holding.count(holds) > 0;
        wants = any;
        active = any;
    }
    onHoldsChanged: hold()
    Component.onCompleted: hold()
    hue: "voice"
    opens: "forYou"
    // How many, as For You's row counts them.
    value: String(ForYouHolds.count)
    title: Words.name("on:waiting")
    actions: [{
        glyph: Words.glyph("on:waiting"),
        name: Words.name("on:waiting"),
        verb: Words.verb("on:waiting"),
        run: shell => shell.launcher.into("forYou")
    }]
}
