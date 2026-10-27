pragma Singleton
pragma ComponentBehavior: Bound
// The mail that wants Joel, for the launcher's Mail program and Now's
// wants: unread, important and in his inbox, newest first, as the google
// program keeps it (src/mail.rs) and Google.qml reads it, with how many in
// the inbox are unread. With no account there's none, and nothing shows.
//
// `open` opens a piece of mail in Mail through google-opens (opens.sh).
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
        if (mail && mail.page)
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
