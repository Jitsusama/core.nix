// The mail that wants Joel in the launcher, as parts of the Mail program
// (program:google-mail, the desktop entry home.nix gives), newest first as
// Mail.qml holds it, so Programs › Mail lists it after the program's own
// acts, and a subject finds its mail from anywhere. Each row is Mail.row's,
// the one every surface shows: Return opens it in Mail, the right arrow
// lists its acts, and its details show beside it. Mail's own Recent is the
// mail Joel sent or read in the last month (recentOf, which
// Applications.qml asks every provider that has it), a conversation once,
// as Google.qml gives it, each its one row too.
import qs
import QtQuick

Provider {
    id: provider

    what: "mail"

    things: Mail.mail.map(mail => Mail.row(mail, provider.thing))
    onThingsChanged: changedTo(things)

    // The desktop entry whose Recent this gives (home.nix).
    readonly property string program: "google-mail"

    // Mail's Recent: what he sent or read in the month, newest first, a
    // conversation once by its newest, each with when it came, so
    // Recent.js merges it by time. Made only as it's asked, as Mail is
    // stepped into.
    function recentOf(asked) {
        if (asked !== program)
            return [];
        return Google.hadMail()
            .map(one => Object.assign(Mail.row(one.mail, provider.thing), { at: one.at }));
    }

    // Whether a program's Recent is this one's to give, so Mail's stands,
    // saying "none", while it holds nothing.
    function keepsRecentOf(asked) {
        return asked === program;
    }

    // Mail's launcher, which a file it opens closes (Mail.qml).
    Binding {
        target: Mail
        property: "launcher"
        value: provider.launcher
    }
}
