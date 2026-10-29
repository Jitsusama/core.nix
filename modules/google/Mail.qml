pragma Singleton
pragma ComponentBehavior: Bound
// The mail that wants Joel, for the launcher's Mail program and Now's
// wants: unread, important and in his inbox, newest first, as the google
// program keeps it (src/mail.rs) and Google.qml reads it, with how many in
// the inbox are unread. With no account there's none, and nothing shows.
//
// A piece of mail is one thing wherever the launcher shows it: `row`
// gives its whole row, which the Mail place (Messages.qml), Now's mail
// (NewMail.qml) and search all take as it is, with its acts in one order,
// Open, Reply, Archive, Read (a switch) and Copy the Link, and its
// details in the drawer.
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

    // Each { id, from, address, to, subject, at, page, body }.
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
    // asked, or "unread" once it's back; nothing while it's as it was.
    function became(mail, how) {
        if (asked[mail.id] !== how)
            return "";
        return (how === "unread") === held(mail.id) ? Said.tidied(how) : "";
    }

    // A piece of mail's row, made with the launcher's `thing` of the
    // surface showing it, so every surface shows the same one: its subject,
    // who it's from and when, and its details beside it. Return opens it in
    // Mail; the right arrow lists every act.
    function row(mail, thing) {
        return thing({
            key: `mail:${mail.id}`,
            what: "mail",
            component: "mail",
            of: "program:google-mail",
            part: titled(mail),
            name: titled(mail),
            verb: Words.verb("verb:mail:open"),
            glyph: Words.glyph("mail:message"),
            hue: "program",
            side: Said.beside(mail, Calendar.now),
            drawer: Said.details(mail, Words.name("mail:message", "untitled")),
            later: true,
            run: () => root.open(mail),
            verbs: () => root.acts(mail, thing)
        });
    }

    function titled(mail) {
        return mail.subject !== "" ? mail.subject : Words.name("mail:message", "untitled");
    }

    // Its acts, in the one order every surface lists them.
    function acts(mail, thing) {
        const act = (key, run) => thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            later: true,
            run: run
        });
        const listed = [act("verb:mail:open", () => root.open(mail))];
        if (Said.replying(mail) !== "")
            listed.push(act("verb:mail:reply", () => root.reply(mail)));
        return listed.concat([tidying(mail, "archive", thing), reading(mail, thing),
            linking(mail, thing)]);
    }

    // Whether it's been marked read: asked here, and no longer held as
    // wanting Joel. All the mail held is unread, since that's what wants
    // him.
    function isRead(mail) {
        return asked[mail.id] === "read" && !held(mail.id);
    }

    // Read is a switch, one row whose name never changes (VKCF3Z's design,
    // RSCH-20261028-2NK1E5, rule 8): it says yes or no, Return flips it
    // and keeps the launcher, and the foot says what became of it.
    function reading(mail, thing) {
        const key = "verb:mail:read";
        const row = thing({
            key: key,
            name: Words.name(key),
            glyph: Words.glyph(key),
            hue: "action",
            stays: true,
            state: () => root.became(mail, "read") || root.became(mail, "unread"),
            run: () => root.tidy(mail, root.isRead(mail) ? "unread" : "read")
        });
        return Object.defineProperties(row, {
            side: { enumerable: true, get: () => root.isRead(mail) ? "yes" : "no" },
            verb: {
                enumerable: true,
                get: () => Words.verb(key, root.isRead(mail) ? "read" : "")
            }
        });
    }

    // Copy the Link copies its conversation's link in Gmail, for pasting
    // where someone else can open it.
    function linking(mail, thing) {
        const key = "verb:mail:link";
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            now: true,
            run: () => {
                Quickshell.clipboardText = mail.page;
            }
        });
    }

    // Archiving it or marking it read keeps the launcher, since he's going
    // through what wants him, and the foot says what became of it.
    function tidying(mail, how, thing) {
        const key = `verb:mail:${how}`;
        return thing({
            key: key,
            name: Words.name(key),
            verb: Words.verb(key),
            glyph: Words.glyph(key),
            hue: "action",
            stays: true,
            state: () => root.became(mail, how),
            run: () => root.tidy(mail, how)
        });
    }

    // Gmail's link to write someone a new mail, or nothing for no address.
    function writing(address) {
        return Said.writing(address);
    }

    // A reply is written in a Mail window of its own, beside the mail it
    // answers, so neither closes the other.
    function reply(mail) {
        const link = Said.replying(mail);
        if (link !== "")
            Niri.spawn(["google-opens", link]);
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
