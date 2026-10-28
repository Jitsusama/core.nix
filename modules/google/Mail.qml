pragma Singleton
pragma ComponentBehavior: Bound
// The mail that wants Joel, for the launcher's Mail program and Now's
// wants: unread, important and in his inbox, newest first, as the google
// program keeps it (src/mail.rs) and Google.qml reads it, with how many in
// the inbox are unread. With no account there's none, and nothing shows.
//
// `open` opens a piece of mail in Mail through google-opens (opens.sh),
// keeping one Mail window: with Mail open, it brings that window forward,
// so the new one opens beside it, and closes it once the new one shows.
// `read` and `archive` ask the program, which takes it out of the mail
// that wants him as soon as Gmail has done it.
import qs
import Quickshell
import QtQuick
import "Mail.js" as Said

Singleton {
    id: root

    // Each { id, from, subject, at, page }.
    readonly property var mail: Google.mail
    // How many in the inbox are unread, or null until Gmail says.
    readonly property var unread: Google.unread

    // How each piece asked about was tidied, "read" or "archive", by its
    // id, so its row says what became of it once it's gone.
    property var asked: ({})

    // Mail's windows open now, by id; niri's revision says when a window
    // opens or closes.
    readonly property var windows: {
        Niri.revision;
        return Said.windows(Object.values(Niri.windows));
    }
    // The Mail windows open when mail was last opened, to close once its
    // own window shows. Chrome may never open one, so they're let be after
    // half a minute.
    property var replacing: []

    onWindowsChanged: {
        const going = Said.superseded(replacing, windows);
        if (going.length === 0)
            return;
        replacing = [];
        letBe.stop();
        for (const id of going)
            Niri.act({ CloseWindow: { id: id } });
    }

    Timer {
        id: letBe
        interval: 30000
        onTriggered: root.replacing = []
    }

    // When it came, as its row says it: "14:02", "yesterday".
    function came(mail) {
        return Said.came(mail.at, Calendar.now);
    }

    function held(id) {
        return root.mail.some(each => each.id === id);
    }

    // What became of it, "read" or "archived", once it went the way `how`
    // asked; nothing while it still wants him.
    function became(mail, how) {
        return !held(mail.id) && asked[mail.id] === how ? Said.tidied(how) : "";
    }

    function open(mail) {
        if (!mail || !mail.page)
            return;
        if (windows.length > 0) {
            Niri.act({ FocusWindow: { id: windows[0] } });
            replacing = windows;
            letBe.restart();
        }
        Niri.spawn(["google-opens", mail.page]);
    }

    function read(mail) {
        tidy(mail, "read");
    }

    function archive(mail) {
        tidy(mail, "archive");
    }

    function tidy(mail, how) {
        if (!mail)
            return;
        asked = Object.assign({}, asked, { [mail.id]: how });
        Google.ask(Said.tidy(mail, how));
    }
}
