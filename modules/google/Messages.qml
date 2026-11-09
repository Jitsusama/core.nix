// The mail that wants Joel in the launcher, as parts of the Mail program
// (program:google-mail, the desktop entry home.nix gives), newest first as
// Mail.qml holds it, so Programs › Mail lists it after the program's own
// acts, and a subject finds its mail from anywhere. Each row is Mail.row's,
// the one every surface shows: Return opens it in Mail, the right arrow
// lists its acts, and its details show beside it.
//
// Mail's Recent (recentOf, which Applications.qml asks every provider that
// has it) is the mail Joel sent or read lately, newest first (Mail.lately),
// each that same row, who it's from and when at its side, Return opening
// it in Mail. Mail that still wants him is listed above it and never
// Recent. It stands, "none" while it holds nothing (keepsRecentOf), since
// the program keeps a month of it.
import qs
import QtQuick

Provider {
    id: provider

    what: "mail"

    things: Mail.mail.map(mail => Mail.row(mail, provider.thing))
    onThingsChanged: changedTo(things)

    function recentOf(program) {
        return program === "google-mail"
            ? Mail.lately.map(mail => Object.assign(Mail.row(mail, provider.thing),
                { at: mail.at }))
            : [];
    }

    function keepsRecentOf(program) {
        return program === "google-mail";
    }

    // So Programs › Mail, held as mail is sent or read, or as a side moves
    // on, draws its Recent again, and only then.
    readonly property string recentSaid: JSON.stringify(Mail.lately
        .map(mail => [mail.id, Mail.came(mail)]))
    onRecentSaidChanged: changed()

    // Mail's launcher, which a file it opens closes (Mail.qml).
    Binding {
        target: Mail
        property: "launcher"
        value: provider.launcher
    }
}
