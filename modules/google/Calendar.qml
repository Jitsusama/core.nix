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
// their own.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Calendar.js" as Said

Singleton {
    id: root

    readonly property string told: `${Quickshell.env("XDG_RUNTIME_DIR")}/google.json`

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

    FileView {
        path: root.told
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: root.hear(text())
        onLoadFailed: root.forget()
    }
}
