// A meter's moment (modules/quickshell/Moment.qml), timed in Qt's own
// engine. The `a-meter-stays-its-moment-once-seen` check copies Moment.qml
// and tests/lib/Expect.js beside this file and runs it offscreen, its moment
// a tenth of a meter's so the cases take a second or so. Each case shows a
// moment, does what it says at its times, and is held to when it's spent.
import QtQuick
import "Expect.js" as Expect

Item {
    id: root

    // A tenth of a meter's: 150 ms once seen, and 1 s at most.
    readonly property int lasts: 150
    readonly property int atMost: 1000
    // How far either side of its time a spending may come, a busy builder's
    // timers firing late.
    readonly property int slack: 60

    // Each case: what's done when, in ms from showing, and when it's spent,
    // or -1 for never within the case's span, with whether it was seen.
    readonly property var cases: [
        { name: "never shown a frame, it's spent at the most it lasts, unseen",
          does: [], spent: 1000, seen: false, span: 1300 },
        { name: "shown a frame 300 ms on, it lasts from then, not from showing",
          does: [[300, "shown"]], spent: 450, seen: true, span: 800 },
        { name: "changed before its frame, it waits for its frame",
          does: [[50, "again"], [400, "shown"]], spent: 550, seen: true, span: 900 },
        { name: "changed once seen, it lasts from the change",
          does: [[20, "shown"], [120, "again"]], spent: 270, seen: true, span: 600 },
        { name: "hidden before it's spent, it's never spent",
          does: [[20, "shown"], [80, "hide"]], spent: -1, seen: true, span: 1300 },
        { name: "shown again once hidden, it waits for a frame anew",
          does: [[20, "shown"], [80, "hide"], [100, "show"], [600, "shown"]], spent: 750,
          seen: true, span: 1100 }
    ]

    property int at: 0
    property double began: 0
    property var heard: null
    property int done: 0

    function fail(why) {
        console.warn(`FAIL ${why}`);
        Qt.callLater(Qt.exit, 1);
    }

    function begin() {
        if (at === cases.length) {
            Qt.exit(0);
            return;
        }
        heard = null;
        done = 0;
        moment.showing = false;
        began = Date.now();
        moment.showing = true;
        tick.start();
    }

    function act(what) {
        if (what === "shown")
            moment.shown();
        else if (what === "again")
            moment.again();
        else
            moment.showing = what === "show";
    }

    function judge() {
        const one = cases[at];
        try {
            if (one.spent < 0)
                Expect.expect(one.name, heard === null, `spent at ${heard?.after} ms`);
            else {
                Expect.expect(one.name, heard !== null, "never spent");
                const off = Math.abs(heard.after - one.spent);
                Expect.expect(one.name, off <= slack, `spent at ${heard.after} ms`);
                Expect.same(`${one.name}, seen`, heard.seen, one.seen);
            }
        } catch (error) {
            fail(error.message);
            return;
        }
        at += 1;
        begin();
    }

    Moment {
        id: moment
        lasts: root.lasts
        atMost: root.atMost
        onSpent: wasSeen => {
            if (root.heard === null)
                root.heard = { after: Date.now() - root.began, seen: wasSeen };
        }
    }

    Timer {
        id: tick
        interval: 5
        repeat: true
        onTriggered: {
            const one = root.cases[root.at];
            const now = Date.now() - root.began;
            while (root.done < one.does.length && one.does[root.done][0] <= now) {
                root.act(one.does[root.done][1]);
                root.done += 1;
            }
            if (now >= one.span) {
                stop();
                root.judge();
            }
        }
    }

    Component.onCompleted: begin()
}
