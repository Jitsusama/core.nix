pragma Singleton
pragma ComponentBehavior: Bound
// Joel's Google account, as the shell knows it: what the google program
// keeps of it (src/state.rs), held over one connection to its daemon
// (src/daemon.rs), which says each meeting, piece of mail and document,
// and how the account stands, as a record of its own, and each as it
// changes. Calendar.qml reads the meetings from here and the mail
// provider the mail. On a machine nobody signed in to Google on there's
// no socket, so the account is "none", there's nothing held and nothing
// shows; Daemon.qml waits for the socket rather than trying it again and
// again.
//
// `ask` hands the program a line it reads (src/ask.rs), as the daemon's
// act; what changes comes back in the records.
import Quickshell
import QtQuick
import "Google.js" as Said

Singleton {
    id: root

    Daemon {
        id: google

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/google.sock`
        kind: ["meeting", "mail", "file", "held", "lately", "call", "account"]
        // What Google held may not be Joel's to see once it answers again,
        // and a meeting cancelled in the gap would otherwise stay.
        keptThroughGap: false
    }

    // What's kept of the daemon's records, each kind's by key (Google.js's
    // keep), so a change remakes only the parts of the kinds it names: the
    // account told again each minute remakes the account's own parts and
    // never a list. A plain object, so keeping one notifies nothing.
    readonly property var kept: Said.keeping()
    // Whether there was an account to say at the last change, so the
    // change that first says one remakes every part.
    property bool ready: false

    Connections {
        target: google
        function onHeard(changed, gone) {
            root.took(Said.keep(root.kept, google.held, changed.concat(gone)));
        }
    }

    // Takes the parts the kinds moved make (Google.js's partsOf), each
    // only if its text changed (hear); none while the daemon isn't
    // answering.
    function took(moved) {
        const now = Said.ready(kept, answering);
        const parts = Said.partsOf(kept, now && !ready ? Said.everyKind() : moved, answering);
        ready = now;
        hear(parts);
    }

    // "none", "signedOut", "unreachable" or "ready", as the program says.
    property string account: "none"
    // Each { id, title, start, end, allDay, answer, link, kind, page,
    // done }, in milliseconds since 1970, from today's start to a week on;
    // `done` is { how, at } once it has ended, else null (src/done.rs).
    // One Joel declined, or its organiser cancelled, is never here.
    property var meetings: []
    // The mail that wants Joel, newest first: each { id, from, subject,
    // at, page }.
    property var mail: []
    // The mail that wanted Joel and no longer does, the latest done first,
    // each as `mail` says it with a `done` of { how, at }: read or archived
    // in Gmail or here, no longer important, or gone (src/done.rs).
    property var finished: []
    // How many in the inbox are unread, or null until Gmail says.
    property var unread: null
    // The documents Joel looked at lately, each kind's last first: each
    // { id, title, kind, link, viewed, changed, owner }.
    property var documents: []
    // The meetings held in the month before today, by start, as `meetings`
    // says each, and the mail Joel sent or read in it, newest first, as
    // `mail` says each: what a person's Recent holds (GoogleSource.qml).
    property var held: []
    property var lately: []
    // The Meet calls followed, as Meet last said each: { code, granted,
    // started, people: [{ name, joined, left }] } (src/meet.rs).
    property var calls: []
    // When Google last answered, or null.
    property var heard: null
    // The file mail carries last asked for, as it stands: { mail, file,
    // how, asked, path, failed }, or null.
    property var fetched: null
    // Making a Meet meeting: whether Joel's sign-in grants it, and the one
    // last asked for, { asked, link, failed }, or null.
    property var meet: ({ granted: false, made: null })
    // Drive's folders, downloads and saves lately, each newest first: each
    // folder's last listing { folder, asked, files, more, failed }, each
    // download { file, how, asked, path, failed }, and each save the picker
    // staged { path, folder, name, asked, id, link, failed }; and Drive's
    // own places, { recent, starred, shared, drives } (Google.js's
    // placesOf).
    property var drive: Said.stateOf({ account: "none", meetings: [] }).drive

    // The person an address is, as People's row for them, from the
    // launcher's provider that knows people (People.qml's personOf), so a
    // mail's sender and a meeting's guests step through to who they are;
    // null on a machine without People, or for someone no source knows.
    function personOf(launcher, email) {
        const people = (launcher?.sources ?? []).find(source => typeof source.personOf
            === "function");
        return people ? people.personOf(email) : null;
    }

    // The launcher's provider that knows people, asked who an address is
    // with no row made (People.qml's whoIs), as a meeting's guests are
    // counted; null on a machine without People.
    function peopleOf(launcher) {
        return (launcher?.sources ?? []).find(source => typeof source.whoIs
            === "function") ?? null;
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

    // Asks the program to follow the Meet calls of these codes, the ones
    // open in windows of their own (Calendar.qml's calls), so Meet is
    // asked who's in each while they are.
    function follow(codes) {
        following = codes;
        ask(Said.follow(codes));
    }

    // The codes last followed, asked again whenever the daemon answers
    // anew, since a program started again follows nothing.
    property var following: []
    readonly property bool answering: google.state === "answering"
    onAnsweringChanged: {
        took({});
        if (answering && following.length > 0)
            ask(Said.follow(following));
    }

    // Asks the program what a Drive folder holds, which comes in
    // `drive.listings`.
    function list(folder) {
        ask(Said.list(folder));
    }

    // Asks for a save the picker staged to be sent up again, as Try Again
    // does once Drive couldn't be reached (Saves.qml).
    function upload(path) {
        ask(Said.upload(path));
    }

    function listing(folder, since) {
        return Said.listing(drive, folder, since);
    }

    // Asks the program to download a Drive file, which comes in
    // `drive.downloads`.
    function download(file, how, to) {
        ask(Said.download(file, how, to));
    }

    function downloading(file, since) {
        return Said.downloading(drive, file, since);
    }

    function hear(said) {
        if (!said)
            return;
        // Only what changed, since a part set again remakes all that reads
        // it, and most lines only say when Google last answered.
        const news = Said.changed(texts, said);
        texts = news.texts;
        for (const part of news.parts)
            root[part] = said[part];
    }

    // The text each part was last taken from (Google.js's changed).
    property var texts: ({})

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

    // Whether mail Joel sent or read is with someone: from them, or to
    // them or copied to them.
    function mailedWith(mail, emails) {
        return Said.mailedWith(mail, emails);
    }

    // Asks the program one thing, a line its ask.rs reads, as the daemon's
    // act. It's running whenever there's something to ask about; while the
    // daemon isn't answering, what's asked is let go, as nothing is there
    // to take it.
    function ask(line) {
        google.ask({ ask: "act", act: JSON.parse(line) }, null);
    }
}
