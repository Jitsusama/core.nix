// What day it is in the Chinese, Japanese, Hebrew and Islamic calendars,
// as timekeeping reckons it with ICU4X when asked (timekeeping/src/
// calendar.rs), over varlink on the socket systemd holds for it. Date asks
// for a day as it's shown, and the socket is held only until that day's
// answer comes, so it's asked once a day it's looked at and nothing is
// held at rest. A machine without the reader has no socket, and Date's
// Calendars says it isn't answering. Asked from where the laptop is, the
// answer says when that day's sunset begins the Hebrew and Islamic days
// there, reckoned with the day, so nothing waits on the sun.
pragma Singleton
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // The day asked for, as an ISO date, or "" before any is.
    property string asked: ""

    // Where it's asked from, { latitude, longitude }, or null from nowhere.
    property var from: null

    // The day and place asked, as one, and those the reading answers.
    readonly property string question: JSON.stringify({ day: asked,
        latitude: from?.latitude, longitude: from?.longitude })
    property string answered: ""

    // The last answer, { day, calendars, evening }, kept until another
    // day's comes.
    property var reading: null

    // Whether the reader failed to answer the day asked: refused, gone,
    // or the day not one it reckons. Asking again clears it.
    property bool refused: false

    readonly property string path: `${Quickshell.env("XDG_RUNTIME_DIR")}/io.jitsusama.Timekeeping`

    // Whether the day asked still waits on its answer.
    readonly property bool waiting: asked !== "" && !refused && answered !== question

    // Ask for a day, as an ISO date, from where the laptop is, as Location
    // says it: answered at once if it's the day and place last answered,
    // and otherwise asked once.
    function ask(day, here) {
        const place = typeof here?.latitude === "number" && typeof here?.longitude === "number"
            ? { latitude: here.latitude, longitude: here.longitude } : null;
        if (day === asked && JSON.stringify(place) === JSON.stringify(from) && !refused)
            return;
        refused = false;
        from = place;
        asked = day;
    }

    function unanswered() {
        if (waiting)
            refused = true;
    }

    // The reader's one reply to a question. A line that isn't JSON is
    // passed over.
    function heard(data, question) {
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
        answered = question;
    }

    // A refusal is said once the loader has made its Socket, never while
    // it makes it: a Unix socket refuses within connectToServer, and the
    // loader turned off before its item is made keeps the item anyway, so
    // the refused Socket would stand and the next day asked never connect.
    LazyLoader {
        active: root.waiting

        Socket {
            id: socket

            // What this connection last asked, so each answer is known
            // for what it answers, and a day or place asked while it's
            // open is asked on it too.
            property var sent: []

            // What's asked now, asked on it once the day and place are
            // both set, as asking sets one then the other.
            readonly property string asking: root.question
            onAskingChanged: Qt.callLater(socket.resend)

            function send() {
                sent = sent.concat([root.question]);
                write(JSON.stringify({
                    method: "io.jitsusama.Timekeeping.Calendars",
                    parameters: Object.assign({ day: root.asked }, root.from ?? {})
                }) + "\u0000");
                flush();
            }

            function resend() {
                if (socket.connected && root.waiting && !socket.sent.includes(root.question))
                    socket.send();
            }

            path: root.path
            connected: true
            onConnectionStateChanged: {
                if (connected)
                    send();
                else
                    Qt.callLater(root.unanswered);
            }
            onError: Qt.callLater(root.unanswered)
            parser: SplitParser {
                splitMarker: "\u0000"
                onRead: data => {
                    const question = socket.sent[0];
                    socket.sent = socket.sent.slice(1);
                    root.heard(data, question);
                }
            }
        }
    }
}
