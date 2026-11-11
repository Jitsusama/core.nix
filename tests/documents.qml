// Drives the launcher's Documents (modules/google/Documents.qml) through
// Quickshell itself, with the real Drive.qml making each row from the one
// the google daemon serves ready, and Google's account, the clock, the
// mail and the saves stood in. Its daemon is told records as the google
// daemon tells them (src/document_rows.rs), through Daemon.qml's own
// took, with no daemon to answer: every case runs in one turn, before a
// refused connection could let them go. With the launcher shut, rows
// told and a minute passing make no row, though each row says when it
// was opened against the clock; opened, it makes them once, in the
// daemon's order; opened again, it makes only those that changed; a row
// told ahead of the rest goes first, and one gone goes; and shut again,
// it keeps the rows it made. A copy of a row makes no drawer, and its
// drawer reads its record as it is now. It exits 0 once every case
// holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    // A document's ready row, Joel's, opened an hour ago, at `order`.
    function row(id, title, order) {
        const key = `row:documents:${id}`;
        const link = `https://docs.google.com/document/d/${id}/edit`;
        const viewed = Calendar.now - 3600000;
        return { key, kind: "row", row: {
            key, thing: `gdoc:${id}`, place: "documents", kind: "document", name: title,
            glyph: "googleDocument", hue: "program", depth: 0,
            side: [{ at: viewed, as: "opened", keep: 1 }], drawer: `drawer:documents:${id}`,
            acts: [{ key: "verb:docs:open", ask: { open: { link } } },
                { key: "verb:docs:link", ask: { copy: link } }],
            held: { link, viewed, mime: "" }, order
        } };
    }

    function drawer(id, title) {
        const key = `drawer:documents:${id}`;
        return { key, kind: "drawer", drawer: { eyebrow: "in My Drive \u00b7 Document", title,
            facts: [{ name: "Owner", said: "Ada Lovelace" }], body: "" } };
    }

    function tell(records, gone) {
        documents.daemon.took(records, gone ?? []);
    }

    QtObject {
        id: standIn

        property bool open: false
        // Whether Documents' rows are wanted (Provider.qml's wanted), as
        // the launcher says from where it stands: never while it's shut.
        function wants(source) {
            return open;
        }
    }

    Launcher.Documents {
        id: documents

        launcher: standIn
        settings: ({ name: "documents" })
    }

    function titles() {
        return documents.things.map(made => made.name).join(", ");
    }

    Component.onCompleted: {
        try {
            const shut = documents.tally.rows;
            tell([row("d1", "Quarterly Plan", 1024), drawer("d1", "Quarterly Plan")]);
            Expect.expect("shut, rows told make no row", documents.tally.rows === shut,
                documents.tally.rows - shut);
            // Every row says when it was opened, as minutes ago, so the
            // clock's minute remade them all with the launcher shut, 27 a
            // minute on Joel's shell (2026-11-10).
            Calendar.now += 60000;
            tell([row("d2", "Roadmap", 2048), drawer("d2", "Roadmap")]);
            Expect.expect("nor a minute passing and another told", documents.tally.rows === shut,
                documents.tally.rows - shut);
            standIn.open = true;
            Expect.expect("opened, it holds every document in the daemon's order",
                titles() === "Quarterly Plan, Roadmap", titles());
            Expect.expect("made once", documents.tally.rows - shut === 2, documents.tally.rows - shut);
            Expect.expect("each the thing it was, opening its link",
                documents.things[0].key === "gdoc:d1"
                && documents.things[0].verb === Words.verb("verb:docs:open"),
                JSON.stringify([documents.things[0].key, documents.things[0].verb]));
            Expect.expect("and its program's Recent holds them",
                documents.recentOf("google-docs").map(made => made.name).join(", ")
                === "Quarterly Plan, Roadmap", "");
            // Opened again, it makes only what changed: every opening made
            // all 27 of Joel's again (2026-11-10).
            standIn.open = false;
            const opened = documents.tally.rows;
            standIn.open = true;
            Expect.expect("opened again, it makes none that reads the same",
                documents.tally.rows === opened, documents.tally.rows - opened);
            tell([row("d2", "Roadmap, Revised", 2048), drawer("d2", "Roadmap, Revised")]);
            Expect.expect("and one changed, only it", documents.tally.rows - opened === 1,
                documents.tally.rows - opened);
            Expect.expect("as it reads now", titles() === "Quarterly Plan, Roadmap, Revised",
                titles());
            const ahead = documents.tally.rows;
            tell([row("d3", "Archive", 512)]);
            Expect.expect("one told ahead goes first",
                titles() === "Archive, Quarterly Plan, Roadmap, Revised", titles());
            Expect.expect("made alone", documents.tally.rows - ahead === 1,
                documents.tally.rows - ahead);
            tell([], ["row:documents:d3"]);
            Expect.expect("and gone, goes", titles() === "Quarterly Plan, Roadmap, Revised",
                titles());
            // A copy of a row reads all it holds, so a drawer it held as
            // a getter would be made with every copy (Y2YBTS, 2026-11-10).
            const copied = documents.things.map(made => Object.assign({}, made));
            Expect.expect("a copy of every row makes no drawer",
                copied.every(made => typeof made.drawer === "function"),
                copied.map(made => typeof made.drawer).join(", "));
            const shown = documents.things[1].drawer();
            Expect.expect("its drawer as it reads now",
                shown?.title === "Roadmap, Revised"
                && shown.facts[0].value === "Ada Lovelace", JSON.stringify(shown));
            standIn.open = false;
            const closed = documents.tally.rows;
            Calendar.now += 60000;
            Expect.expect("shut again, it keeps what it made",
                titles() === "Quarterly Plan, Roadmap, Revised", titles());
            Expect.expect("and makes none", documents.tally.rows === closed,
                documents.tally.rows - closed);
            Qt.callLater(() => Qt.exit(0));
        } catch (failed) {
            Expect.fail(failed.message);
        }
    }
}
