// Drives when For You (modules/quickshell/launcher/Now.qml) places the time
// and the date, through Quickshell itself. Placing them makes Time's and
// Today's rows, so an opening into a place that shows none of Now's rows
// leaves them until Now is wanted, where every opening once made all 17
// as it began (H6EDLC, 2026-11-11), and Now's rows, opened onto or come
// to, place them once, and only once a minute has turned since they were
// placed: an opening into For You that wasn't yet asleep made all 17
// again as Now's rows were come to, with no minute turned (2026-11-12).
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    // The launcher as Now reads it, shut, wanting Now's rows only as said,
    // counting each time it's asked to place the time.
    QtObject {
        id: theLauncher

        property bool open: false
        property bool wanting: false
        property var held: []
        readonly property var sources: []
        property int placed: 0

        function wants(source) {
            return wanting;
        }
        function resolved(id) {
            if (id === "time")
                placed += 1;
            return null;
        }
    }

    Launcher.Now {
        id: now

        launcher: theLauncher
        settings: ({ name: "on" })
    }

    // The start's own placing is let through first.
    Timer {
        running: true
        interval: 100
        onTriggered: root.intoAPlace()
    }

    function intoAPlace() {
        try {
            theLauncher.placed = 0;
            theLauncher.open = true;
            Expect.same("opened into a place, it places no time", theLauncher.placed, 0);
            theLauncher.wanting = true;
            cameToNow.start();
        } catch (error) {
            Expect.fail(error.message ?? error);
        }
    }

    // Wanted within the minute the start placed it in, nothing is stale.
    Timer {
        id: cameToNow

        interval: 50
        onTriggered: {
            try {
                Expect.same("come to Now's rows within the minute, it places no time",
                    theLauncher.placed, 0);
                theLauncher.wanting = false;
                theLauncher.open = false;
                theLauncher.open = true;
                now.timedMinute -= 1;
                theLauncher.wanting = true;
                cameOn.start();
            } catch (error) {
                Expect.fail(error.message ?? error);
            }
        }
    }

    // Wanted a minute after it was placed, it places the time on its next
    // turn.
    Timer {
        id: cameOn

        interval: 50
        onTriggered: {
            try {
                Expect.expect("come to Now's rows a minute on, it places the time",
                    theLauncher.placed >= 1, `placed ${theLauncher.placed}`);
                theLauncher.open = false;
                theLauncher.placed = 0;
                theLauncher.open = true;
                Expect.same("opened onto Now's rows within the minute, it places no time",
                    theLauncher.placed, 0);
                theLauncher.open = false;
                // As if it were placed in the minute before this one.
                now.timedMinute -= 1;
                theLauncher.open = true;
                Expect.same("opened onto Now's rows a minute on, it places the time once",
                    theLauncher.placed, 1);
                Qt.exit(0);
            } catch (error) {
                Expect.fail(error.message ?? error);
            }
        }
    }
}
