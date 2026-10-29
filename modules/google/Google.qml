pragma Singleton
pragma ComponentBehavior: Bound
// Joel's Google account, as the shell knows it: what the google program
// keeps of it (src/state.rs), read from the file it leaves beside its
// socket, google.json, which it replaces whole whenever the state changes.
// Calendar.qml reads the meetings from here and the mail provider the mail.
// Until the program runs, as on a machine nobody signed in to Google on,
// there's no file, so the account is "none", there's nothing held and
// nothing shows; nothing is tried again and again, since the file is
// watched.
//
// `ask` hands the program a line on its socket (src/ask.rs), connecting
// only to ask; what changes comes back in the file.
import Quickshell
import Quickshell.Io
import QtQuick
import "Google.js" as Said

Singleton {
    id: root

    readonly property string told: `${Quickshell.env("XDG_RUNTIME_DIR")}/google.json`
    readonly property string socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/google.sock`

    // "none", "signedOut", "unreachable" or "ready", as the program says.
    property string account: "none"
    // Each { id, title, start, end, allDay, answer, link, kind, page },
    // in milliseconds since 1970, from today's start to a week on.
    property var meetings: []
    // The mail that wants Joel, newest first: each { id, from, subject,
    // at, page }.
    property var mail: []
    // How many in the inbox are unread, or null until Gmail says.
    property var unread: null
    // The documents Joel looked at lately, each kind's last first: each
    // { id, title, kind, link, viewed, changed, owner }.
    property var documents: []
    // When Google last answered, or null.
    property var heard: null
    // The file mail carries last asked for, as it stands: { mail, file,
    // how, asked, path, failed }, or null.
    property var fetched: null
    // Making a Meet meeting: whether Joel's sign-in grants it, and the one
    // last asked for, { asked, link, failed }, or null.
    property var meet: ({ granted: false, made: null })

    // How the meeting asked for at `since` stands (Google.js).
    function making(since) {
        return Said.making(meet, since);
    }

    // Asks the program to make a new Meet meeting.
    function make() {
        ask(Said.make());
    }

    function hear(line) {
        const said = Said.heard(line);
        if (!said)
            return;
        account = said.account;
        meetings = said.meetings;
        mail = said.mail;
        unread = said.unread;
        documents = said.documents;
        heard = said.heard;
        fetched = said.fetched;
        meet = said.meet;
    }

    // Everyone the meetings and mail name, by address, for People
    // (GoogleSource.qml), each saying when what's with them is.
    function people(meetings, mail) {
        return Said.people(meetings, mail);
    }

    // Whether a meeting or a piece of mail is with someone by one of their
    // addresses.
    function withThem(thing, emails) {
        return Said.withThem(thing, emails);
    }

    // The program gone, as it is once it stops, is no account: what it held
    // may not be Joel's to see any more.
    function forget() {
        account = "none";
        meetings = [];
        mail = [];
        unread = null;
        documents = [];
        heard = null;
        fetched = null;
        meet = { granted: false, made: null };
    }

    // Asks the program one thing, a line its ask.rs reads. It's running
    // whenever there's something to ask about.
    function ask(line) {
        asks = asks.concat([line]);
        asking.connected = true;
    }

    // What's to be asked of the program, said once the socket connects.
    property var asks: []

    FileView {
        path: root.told
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: root.hear(text())
        onLoadFailed: root.forget()
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
}
