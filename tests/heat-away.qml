pragma ComponentBehavior: Bound
// Drives the launcher's Heat (modules/quickshell/launcher/Heat.qml)
// through Quickshell itself with nothing at the machine daemon's socket.
// Heat's row stands while nothing has been served: before its daemon is
// watched, as a terminal's show finds it with the launcher shut, it's
// held in its place, still being read, so the show finds it and watches;
// and once the daemon is found not answering, it says so, and is still
// there for a show to find, rather than leaving Machine's head naming a
// row that isn't there (VKCF3Z, 058bd4b0). It exits 0 once every case
// holds, and otherwise 1, saying which didn't.
import Quickshell
import QtQuick
import "Expect.js" as Expect
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: "the daemon found away"

    // The launcher as Heat reads it: shut at first, as a terminal's show
    // finds it, then open with "heat" typed, so its daemon is watched.
    QtObject {
        id: theLauncher

        property bool open: false
        property string typed: "heat"
        property var answer: null
        property var held: []
        property var matches: []
        property var placed: []
        property var entered: null

        function enter(row) {
            entered = row;
        }

        function afterFrame(then) {
            then();
        }
    }

    // An answer as a failure says it, without its provider, which holds
    // the launcher and can't be said as JSON.
    function said(answer) {
        return JSON.stringify(Object.assign({}, answer, { live: answer.live === theHeat }));
    }

    Timer {
        running: true
        interval: 30000
        onTriggered: Expect.fail(`never came: ${root.waiting}`)
    }

    Launcher.Heat {
        id: theHeat

        launcher: theLauncher
        settings: ({ name: "heat" })
    }

    // Nothing watched yet: Heat's row held in its place, being read.
    Component.onCompleted: {
        try {
            const held = theHeat.answer("heat");
            Expect.expect("before its daemon is asked, Heat's row stands", held !== null,
                "no answer");
            Expect.expect("held in its place, being read", held.headline === "\u2026"
                && held.asking === true && held.gone !== true && held.rows.length === 0,
                root.said(held));
            Expect.expect("it says it has parts, so a place lists it as it will be",
                held.parted === true && held.live === theHeat, root.said(held));
        } catch (error) {
            root.waiting = "a failure";
            Expect.fail(error.message);
            return;
        }
        theLauncher.open = true;
    }

    // The daemon found away: Heat's row says it isn't answering, and asks
    // the launcher to say it again so.
    Connections {
        target: theHeat.served

        function onStateChanged() {
            if (theHeat.served.state !== "not answering"
                || root.waiting !== "the daemon found away")
                return;
            root.waiting = "the end";
            try {
                const away = theHeat.answer("heat");
                Expect.expect("with its daemon away, Heat's row stands", away !== null,
                    "no answer");
                Expect.expect("saying its daemon isn't answering", away.note === "not answering"
                    && away.headline === "", root.said(away));
                Expect.expect("no longer being read, and still there to find",
                    away.gone !== true && away.asking !== true && away.parted === true,
                    root.said(away));
                Expect.expect("a terminal's show isn't kept waiting on it",
                    theHeat.stillReading === false, "still reading");
            } catch (error) {
                Expect.fail(error.message);
            }
            Qt.callLater(() => Qt.exit(0));
        }
    }
}
