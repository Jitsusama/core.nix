// The mail that wants Joel in the launcher, as parts of the Mail program
// (program:google-mail, the desktop entry home.nix gives), newest first as
// Mail.qml holds it, so Programs › Mail lists it after the program's own
// acts, and a subject finds its mail from anywhere. Return opens it in
// Mail, which takes Joel out of the launcher; the right arrow lists Open
// in Mail, Mark as Read and Archive, the last two keeping the launcher,
// its foot saying what became of it once Gmail has done it. Each row says
// when it came at the right and who it's from below; its names and words
// are the vocabulary's (mail:*, Words.qml).
import qs
import QtQuick

Provider {
    id: provider

    what: "mail"

    things: Mail.mail.map(mail => made(mail))
    onThingsChanged: changedTo(things)

    function made(mail) {
        const name = mail.subject !== "" ? mail.subject
            : Words.name("mail:message", "untitled");
        return thing({
            key: `mail:message:${mail.id}`,
            of: "program:google-mail",
            part: name,
            name: name,
            verb: Words.verb("verb:mail:open"),
            glyph: Words.glyph("mail:message"),
            hue: "program",
            side: Mail.came(mail),
            note: mail.from,
            later: true,
            run: () => Mail.open(mail),
            verbs: () => provider.verbs(mail)
        });
    }

    function verbs(mail) {
        return [thing({
            key: "verb:mail:open",
            name: Words.name("verb:mail:open"),
            verb: Words.verb("verb:mail:open"),
            glyph: Words.glyph("verb:mail:open"),
            hue: "action",
            later: true,
            run: () => Mail.open(mail)
        }), tidying(mail, "read"), tidying(mail, "archive")];
    }

    // Marking it read or archiving it keeps the launcher, since he's going
    // through what wants him, and the foot says what became of it.
    function tidying(mail, how) {
        const key = `verb:mail:${how}`;
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            stays: true,
            state: () => Mail.became(mail, how),
            run: () => how === "read" ? Mail.read(mail) : Mail.archive(mail)
        });
    }
}
