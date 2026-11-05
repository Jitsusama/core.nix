// What day it is in the Chinese, Japanese, Hebrew and Islamic calendars,
// as timekeeping reckons it with ICU4X when asked (timekeeping/src/
// calendar.rs), over varlink on the socket systemd holds for it. Date asks
// for a day as it's shown, and the socket is held only until that day's
// answer comes, so it's asked once a day it's looked at and nothing is
// held at rest. A machine without the reader has no socket, and Date's
// Calendars says it isn't answering.
pragma Singleton
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // The day asked for, as an ISO date, or "" before any is.
    property string asked: ""

    // The last answer, { day, calendars }, kept until another day's comes.
    property var reading: null

    // Whether the reader failed to answer the day asked: refused, gone,
    // or the day not one it reckons. Asking again clears it.
    property bool refused: false

    readonly property string path: `${Quickshell.env("XDG_RUNTIME_DIR")}/io.jitsusama.Timekeeping`

    // Whether the day asked still waits on its answer.
    readonly property bool waiting: asked !== "" && !refused && reading?.day !== asked

    // Ask for a day, as an ISO date: answered at once if it's the day
    // last answered, and otherwise asked once.
    function ask(day) {
        if (day === asked && !refused)
            return;
        refused = false;
        asked = day;
    }

    function unanswered() {
        if (waiting)
            refused = true;
    }

    // The reader's one reply. A line that isn't JSON is passed over.
    function heard(data) {
        let reply;
        try {
            reply = JSON.parse(data);
        } catch (error) {
            console.warn("timekeeping said something unreadable:", error);
            return;
        }
        if (reply.error || !reply.parameters) {
            unanswered();
            return;
        }
        reading = reply.parameters;
    }

    LazyLoader {
        active: root.waiting

        Socket {
            path: root.path
            connected: true
            onConnectionStateChanged: {
                if (connected) {
                    write(JSON.stringify({
                        method: "io.jitsusama.Timekeeping.Calendars",
                        parameters: { day: root.asked }
                    }) + "\u0000");
                    flush();
                } else {
                    root.unanswered();
                }
            }
            onError: root.unanswered()
            parser: SplitParser {
                splitMarker: "\u0000"
                onRead: data => root.heard(data)
            }
        }
    }
}
