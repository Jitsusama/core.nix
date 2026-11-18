// Which window a notice's sender comes forward in, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `a-notice-comes-forward` check copies Forward.js beside this file and
// runs `qml -platform offscreen notice-comes-forward.qml`. The windows are
// niri's as it gave them on 2026-10-26, cut to what's read.
import QtQuick
import "Forward.js" as Forward

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    readonly property var windows: ({
        "2": { id: 2, app_id: "kitty", focus_timestamp: { secs: 9005, nanos: 0 } },
        "22": { id: 22, app_id: "slack", focus_timestamp: { secs: 8990, nanos: 500 } },
        "23": { id: 23, app_id: "google-chrome", focus_timestamp: { secs: 9000, nanos: 0 } },
        "24": { id: 24, app_id: "google-chrome", focus_timestamp: { secs: 7000, nanos: 0 } },
        "25": { id: 25, app_id: "google-chrome", focus_timestamp: null },
        "26": { id: 26, app_id: null, focus_timestamp: null }
    })
    readonly property var slack: ["slack"]
    readonly property var chrome: ["google chrome", "google-chrome"]

    function run() {
        const failures = [];
        const expect = (name, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${name}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };

        // Slack's notice opened while a terminal was in front, and Slack
        // didn't raise itself: its window comes forward.
        expect("a sender that stayed behind comes forward",
            Forward.toFocus(windows, 2, slack), 22);
        expect("as it does when nothing was focused",
            Forward.toFocus(windows, null, slack), 22);
        // One that raised itself is left alone.
        expect("one that came forward by itself is left alone",
            Forward.toFocus(windows, 22, slack), null);
        expect("in any of its windows",
            Forward.toFocus(windows, 24, chrome), null);
        // Of several, the one used last, which is where its program opens
        // what it was asked to.
        expect("of several, the one used last", Forward.toFocus(windows, 2, chrome), 23);
        // A sender with no window, such as pi's, is focused by its own
        // means, or opens one.
        expect("one with no window open isn't focused",
            Forward.toFocus(windows, 2, ["pi"]), null);
        expect("nor is a window with no program named",
            Forward.senderWindow(windows, [""]), null);
        expect("a window is a sender's by its program's name, case aside",
            Forward.sends({ app_id: "Slack" }, slack), true);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
