// Drives what's said as a notice when it comes true (modules/quickshell/
// Arrivals.qml) through Quickshell itself, with the facts, the notices and
// notify-send stood in; the stand-in notify-send writes down each notice
// it's asked for, a line each. A meeting already telling "10 min" as the
// shell starts is no news, so nothing is said of it; a camera coming on,
// which tells nothing, is said once, as before; the meeting telling
// something new, "start", is said again, its earlier notice taken back
// first and no other mark's, while the camera isn't said again; and the
// meeting going takes back whichever of its notices is up, by its mark.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    property string waiting: "the arrivals settling"
    // The notices dismissed, by id, in the order they went.
    property var dismissed: []

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    // Nothing here waits forever: each case that never comes fails, by
    // name, half a minute after the test starts waiting for it.
    onWaitingChanged: deadline.restart()

    Timer {
        id: deadline
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}; said: ${root.said().join(" | ")}`)
    }

    // A notice as the shell holds one, which says when it's dismissed and
    // then goes, as one does.
    function notice(id, mark) {
        return {
            id: id,
            hints: mark === "" ? {} : { "x-arrival": mark },
            dismiss: () => {
                root.dismissed = root.dismissed.concat([id]);
                const live = Object.assign({}, Notices.live);
                delete live[String(id)];
                Notices.live = live;
            }
        };
    }

    // Each notice notify-send was asked for, a line each.
    function said() {
        saidFile.reload();
        return saidFile.text().split("\n").filter(line => line !== "");
    }

    QtObject {
        id: camera

        property bool present: true
        property bool active: false
        property bool arrives: true
        property bool now: false
        property bool withdraws: false
        property string telling: ""
        property string app: "Camera"
        property string title: "Camera"
        property string hint: "Camera on"
        property string context: ""
        property var actions: []
    }

    QtObject {
        id: meeting

        property bool present: true
        property bool active: true
        property bool arrives: true
        property bool now: false
        property bool withdraws: true
        property string telling: "10 min"
        property string app: "Calendar"
        property string title: "Standup"
        property string hint: "Standup in 10 min"
        property string context: ""
        property var actions: []
    }

    Component.onCompleted: {
        Facts.all = [
            { entry: { mark: "camera" }, piece: camera },
            { entry: { mark: "meeting" }, piece: meeting }
        ];
    }

    Arrivals {
        id: arrivals

        launcher: null

        onSettledChanged: {
            if (settled)
                quiet.start();
        }
    }

    FileView {
        id: saidFile

        path: `${root.runtime}/said`
        blockLoading: true
        printErrors: false
    }

    // What notify-send writes down is looked at often, since a file only
    // added to isn't always heard changing.
    Timer {
        id: looking
        interval: 200
        repeat: true
        onTriggered: root.heard()
    }

    // Long enough for anything said of what stood as it settled to be said.
    Timer {
        id: quiet
        interval: 1000
        onTriggered: {
            try {
                root.expect("nothing said of what stood as the shell started",
                    root.said().length === 0, root.said().join(" | "));
                root.waiting = "the camera said once";
                camera.active = true;
                looking.start();
            } catch (error) {
                root.fail(error.message);
            }
        }
    }

    function heard() {
        try {
            const lines = root.said();
            if (root.waiting === "the camera said once" && lines.length === 1) {
                expect("the camera is said as before", lines[0].includes("--app-name=Camera")
                    && lines[0].includes("Camera on") && !lines[0].includes("x-arrival"), lines[0]);
                // The meeting's earlier notice, one of nobody's, and one of
                // the camera's, which still stands.
                Notices.live = {
                    "7": notice(7, "meeting"),
                    "8": notice(8, ""),
                    "9": notice(9, "camera")
                };
                root.waiting = "the meeting said again";
                meeting.hint = "Standup now";
                meeting.telling = "start";
            } else if (root.waiting === "the meeting said again" && lines.length === 2) {
                expect("the meeting is said anew", lines[1].includes("--app-name=Calendar")
                    && lines[1].includes("Standup now")
                    && lines[1].includes("x-arrival:meeting"), lines[1]);
                expect("its earlier notice was taken back, read, and no other",
                    JSON.stringify(root.dismissed) === "[7]"
                    && JSON.stringify(Notices.marked) === "[7]",
                    `${JSON.stringify(root.dismissed)} ${JSON.stringify(Notices.marked)}`);
                Notices.live = Object.assign({}, Notices.live, { "10": notice(10, "meeting") });
                root.waiting = "the meeting's notice taken back as it goes";
                meeting.active = false;
                looking.stop();
                gone.start();
            }
        } catch (error) {
            fail(error.message);
        }
    }

    // Arrivals takes a notice back a second after what it said stops being
    // true; this looks a moment after that.
    Timer {
        id: gone
        interval: 1500
        onTriggered: {
            try {
                root.expect("the camera isn't said again", root.said().length === 2,
                    root.said().join(" | "));
                root.expect("the one up is taken back by its mark, and no other",
                    JSON.stringify(root.dismissed) === "[7,10]", JSON.stringify(root.dismissed));
                console.log("every case held");
                Qt.exit(0);
            } catch (error) {
                root.fail(error.message);
            }
        }
    }
}
