pragma Singleton
pragma ComponentBehavior: Bound
// Joel's meetings, as the shell knows them, for the launcher's Today row
// and its Calendar program: what the google program keeps of his Google
// account (src/state.rs), read from the file it leaves beside its socket,
// google.json, which it replaces whole whenever the state changes. Until
// the program runs, as on a machine nobody signed in to Google on, there's
// no file, so the account is "none", there are no meetings and nothing
// shows; nothing is tried again and again, since the file is watched.
//
// `next` is the meeting he's about to go to, or the one going on, the same
// one for the Today row and Now's wants (Calendar.js says which), and
// `join` goes into a meeting's call through google-opens (opens.sh), which
// joins a Tuple link through Tuple and opens Google's own in a window of
// their own. `answer` answers an invitation on the program's socket
// (ask.rs), connecting only to ask, and the program says the meeting
// answered, in the file, as soon as Google takes it.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Calendar.js" as Said

Singleton {
    id: root

    readonly property string told: `${Quickshell.env("XDG_RUNTIME_DIR")}/google.json`
    readonly property string socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/google.sock`

    // "none", "signedOut", "unreachable" or "ready", as the program says.
    property string account: "none"
    // Each { id, title, start, end, allDay, answer, link, kind, page },
    // in milliseconds since 1970, from today's start to a week on.
    property var meetings: []
    // When Google last answered, or null.
    property var heard: null

    // The clock the next meeting is reckoned by, a minute at a time, as a
    // meeting only ever comes or goes on one.
    property real now: Date.now()

    Timer {
        interval: 60 * 1000 - Date.now() % (60 * 1000)
        running: true
        repeat: true
        onTriggered: {
            root.now = Date.now();
            interval = 60 * 1000 - root.now % (60 * 1000);
        }
    }

    readonly property var next: Said.next(meetings, now)

    // When a meeting is, as its row says it: "in 12 min", "at 16:00".
    function when(meeting) {
        return Said.when(meeting, now);
    }

    // Its times: "14:00 to 14:30", or "all day".
    function span(meeting) {
        return Said.span(meeting);
    }

    // Its times and, for an invitation, his answer: "14:00 to 14:30, maybe".
    function about(meeting) {
        return Said.about(meeting);
    }

    // His answer as he'd say it, "going", or nothing for one he holds.
    function answered(meeting) {
        return Said.answered(meeting);
    }

    function invited(meeting) {
        return Said.invited(meeting);
    }

    // The meeting `id` as it's held now, so what's said of one picked a
    // while ago is what the program last said.
    function meeting(id) {
        return meetings.find(each => each.id === id) ?? null;
    }

    // Answers the invitation yes, maybe or no, through the program, which
    // is running whenever there's a meeting to answer.
    function answer(meeting, said) {
        if (!meeting)
            return;
        asks = asks.concat([Said.answer(meeting, said)]);
        asking.connected = true;
    }

    // What's to be asked of the program, said once the socket connects.
    property var asks: []

    // Whether the next meeting is about to start, for Now's wants.
    readonly property bool starting: Said.aboutToStart(next, now)

    function join(meeting) {
        if (!meeting || !meeting.link)
            return;
        Niri.spawn(["google-opens", meeting.link]);
    }

    // The meeting's own page in Google Calendar.
    function open(meeting) {
        if (meeting && meeting.page)
            Niri.spawn(["google-opens", meeting.page]);
    }

    function hear(line) {
        const said = Said.heard(line);
        if (!said)
            return;
        account = said.account;
        meetings = said.meetings;
        heard = said.heard;
        now = Date.now();
    }

    // The program gone, as it is once it stops, is no account: what it held
    // may not be Joel's to see any more.
    function forget() {
        account = "none";
        meetings = [];
        heard = null;
    }

    // Connected only to ask: what's waiting is written, and the connection
    // let go, since what the program says back comes in the file. One that
    // won't connect lets what's waiting go, as the program isn't there to
    // take it.
    Socket {
        id: asking
        path: root.socket
        onConnectionStateChanged: {
            if (!connected)
                return;
            for (const ask of root.asks)
                write(ask);
            flush();
            root.asks = [];
            Qt.callLater(() => asking.connected = false);
        }
        onError: {
            root.asks = [];
            connected = false;
        }
    }

    FileView {
        path: root.told
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: root.hear(text())
        onLoadFailed: root.forget()
    }
}
