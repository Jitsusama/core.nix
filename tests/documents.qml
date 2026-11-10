// Drives the launcher's Documents (modules/google/Documents.qml) through
// Quickshell itself, with the real Drive.qml making each document's row
// and Google's account, the clock, the mail and the saves stood in. With
// the launcher shut, a minute passing and a document coming make no row,
// though each row says when it was opened against the clock; opened, it
// makes them once, holding every document; and shut again, it keeps the
// rows it made. It exits 0 once every case holds, and otherwise 1, saying
// which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    function document(id, title) {
        return { id: id, title: title, kind: "document",
            link: `https://docs.google.com/document/d/${id}/edit`,
            viewed: Calendar.now - 3600000, changed: Calendar.now - 7200000,
            owner: "Ada Lovelace" };
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
        return documents.things.map(row => row.name).join(", ");
    }

    Component.onCompleted: {
        try {
            Google.documents = [document("d1", "Quarterly Plan")];
            const shut = documents.tally.rows;
            // Every row says when it was opened, as minutes ago, so the
            // clock's minute remade them all with the launcher shut, 27 a
            // minute on Joel's shell (2026-11-10).
            Calendar.now += 60000;
            Expect.expect("shut, a minute passing makes no row", documents.tally.rows === shut,
                documents.tally.rows - shut);
            Google.documents = [document("d1", "Quarterly Plan"), document("d2", "Roadmap")];
            Expect.expect("nor a document coming", documents.tally.rows === shut,
                documents.tally.rows - shut);
            standIn.open = true;
            Expect.expect("opened, it holds every document", titles() === "Quarterly Plan, Roadmap",
                titles());
            Expect.expect("made once", documents.tally.rows - shut === 2, documents.tally.rows - shut);
            Expect.expect("and its program's Recent holds them",
                documents.recentOf("google-docs").map(row => row.name).join(", ")
                === "Quarterly Plan, Roadmap", "");
            standIn.open = false;
            const closed = documents.tally.rows;
            Calendar.now += 60000;
            Expect.expect("shut again, it keeps what it made", titles() === "Quarterly Plan, Roadmap",
                titles());
            Expect.expect("and makes none", documents.tally.rows === closed,
                documents.tally.rows - closed);
            Qt.callLater(() => Qt.exit(0));
        } catch (failed) {
            Expect.fail(failed.message);
        }
    }
}
