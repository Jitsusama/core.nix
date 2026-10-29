// The mail that wants Joel in the launcher, as parts of the Mail program
// (program:google-mail, the desktop entry home.nix gives), newest first as
// Mail.qml holds it, so Programs › Mail lists it after the program's own
// acts, and a subject finds its mail from anywhere. Each row is Mail.row's,
// the one every surface shows: Return opens it in Mail, the right arrow
// lists its acts, and its details show beside it.
import qs
import QtQuick

Provider {
    id: provider

    what: "mail"

    things: Mail.mail.map(mail => Mail.row(mail, provider.thing))
    onThingsChanged: changedTo(things)
}
