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
    // Drive's folders: the one last listed, { folder, asked, files, more,
    // failed }, or null, and the downloads asked lately, newest first,
    // each { file, how, asked, path, failed }.
    property var drive: ({ listed: null, downloads: [] })

    // The person an address is, as People's row for them, from the
    // launcher's provider that knows people (People.qml's personOf), so a
    // mail's sender and a meeting's guests step through to who they are;
    // null on a machine without People, or for someone no source knows.
    function personOf(launcher, email) {
        const people = (launcher?.sources ?? []).find(source => typeof source.personOf
            === "function");
        return people ? people.personOf(email) : null;
    }

    // The launcher's chooser of files (open-with's FileChooser.qml), which
    // asks where to save, or null on a machine without it.
    function chooserOf(launcher) {
        return (launcher?.sources ?? []).find(source => typeof source.chooseToSave
            === "function") ?? null;
    }

    // How the meeting asked for at `since` stands (Google.js).
    function making(since) {
        return Said.making(meet, since);
    }

    // Asks the program to make a new Meet meeting.
    function make() {
        ask(Said.make());
    }

    // Asks the program what a Drive folder holds, which comes in
    // `drive.listed`.
    function list(folder) {
        ask(Said.list(folder));
    }

    // Asks the program to download a Drive file, which comes in
    // `drive.downloads`.
    function download(file, how, to) {
        ask(Said.download(file, how, to));
    }

    function downloading(file, since) {
        return Said.downloading(drive, file, since);
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
        drive = said.drive;
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
        drive = { listed: null, downloads: [] };
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
