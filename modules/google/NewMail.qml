// The mail that wants Joel (Mail.qml), as one of what wants him in Now and
// the margin: how many, and the newest's sender. It's never a notice, since
// Gmail's own says each one as it comes. Return lists them in Now, each
// Mail.row's, the row every surface shows, with its details beside it,
// and after them Show the Mail, which goes to Programs › Mail. It goes
// once none want him. In Now it's For You's Mail row, among what came for
// him (Now.qml's Lately), and the margin shows it as it is. Mail done
// with, read or archived in Gmail or here, leaves it by itself, and For
// You's History says how (Google.js's finished).
import qs
import QtQuick

Piece {
    readonly property var newest: Mail.mail.length > 0 ? Mail.mail[0] : null

    active: newest !== null
    kind: "wants"
    wants: true
    hue: "person"
    title: "Mail"
    value: `${Mail.mail.length} to read`
    hint: Mail.mail.length === 1 ? "A piece of mail wants you"
        : `${Mail.mail.length} pieces of mail want you`
    context: newest === null || newest.from === "" ? "" : `newest from ${newest.from}`
    rows: thing => Mail.mail.map(mail => Mail.row(mail, thing))
    lately: "mail"
    came: Mail.mail.map(mail => ({ who: mail.from, at: mail.at }))
    finished: Google.finished.map(mail => Object.assign({}, mail, {
        row: thing => Mail.row(mail, thing)
    }))
    actions: [{
        glyph: Theme.glyph.mail,
        name: "Show the Mail",
        verb: "show",
        run: shell => {
            shell.launcher.open = true;
            shell.launcher.goTo({
                trail: ["collection:applications", "program:google-mail"],
                spot: ""
            });
        }
    }]
}
