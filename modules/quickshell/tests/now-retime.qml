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
        property bool cataloguing: false
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

    // The start's own placing is let through first: the cases begin only
    // once it has placed the time (timedMinute), however late a loaded
    // builder runs it. The cases read the wall clock's minute, so one begun
    // in a minute's last seconds would see it turn and rightly place the
    // time again: begun then, as the clock reads when they begin rather
    // than when this was made, they wait for the next minute's start
    // instead, and take the time as placed in that minute (intoAPlace). A
    // timer made with this file once fired after the minute it was meant
    // for had turned, on a loaded builder (gate af3, 2026-11-12). Nor do
    // they begin in a minute's first seconds, since the clock's turn may be
    // heard late and rightly makes the time stale: begun just past one,
    // the case placed it once more (2RCBOJ's gate, 2026-11-12).
    Timer {
        id: begin

        running: true
        interval: 100
        onTriggered: root.intoAPlace()
    }

    function intoAPlace() {
        const into = Date.now() % 60000;
        if (now.timedMinute < 0 || into < 5000 || into > 50000) {
            begin.interval = now.timedMinute < 0 ? 100 : (into < 5000 ? 5100 : 65100) - into;
            begin.start();
            return;
        }
        try {
            // As retime leaves it: placed this minute, and not stale from a
            // turn heard while the case waited.
            now.timedMinute = now.minuteNow();
            now.timeStale = false;
            theLauncher.placed = 0;
            theLauncher.open = true;
            Expect.same("opened into a place, it places no time", theLauncher.placed, 0);
            // The clock's turn to this minute heard only now, late, as a
            // loaded builder hears it.
            now.minuteTurned();
            Expect.same("a turn heard after the time was placed leaves it fresh",
                now.timeStale, false);
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
                Expect.expect("come to Now's rows within the minute, it places no time",
                    theLauncher.placed === 0, `placed ${theLauncher.placed}, stale `
                        + `${now.timeStale}, minute ${now.timedMinute} of ${now.minuteNow()}`);
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
                // A turn heard while it isn't wanted, placed in the minute
                // before, leaves it stale.
                theLauncher.wanting = false;
                now.minuteTurned();
                Expect.same("a turn heard after the minute it was placed in leaves it stale",
                    now.timeStale, true);
                theLauncher.wanting = true;
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
