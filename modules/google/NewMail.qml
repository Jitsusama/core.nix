// The mail that wants Joel (Mail.qml), as one of what wants him in Now and
// the margin: how many, and the newest's sender. It's never a notice, since
// Gmail's own says each one as it comes. Return lists them in Now, each
// Mail.row's, the row every surface shows, with its details beside it;
// the right arrow also shows them in Programs › Mail, or opens the newest
// in Mail. It goes once none want him.
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
    }, {
        glyph: Words.glyph("verb:mail:open"),
        name: "Open the Newest in Mail",
        verb: "open",
        run: () => Mail.open(newest)
    }]
}
