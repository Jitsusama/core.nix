// The documents Joel looked at lately in the launcher: each found by its
// title from anywhere, and each program's own, Docs', Sheets' or Slides',
// as that program's Recent (recentOf, which Applications.qml asks every
// provider that has it), the same row in both. Each is the row the google
// daemon serves ready (src/document_rows.rs), worded, ordered and with its
// acts, held and made as every ready place's are (ReadyRows.qml), so the
// shell only draws it and words its time. It also gives the launcher's
// picker Google Drive as a place (chooserPlaces), and gives Saves.qml the
// launcher, whose picker a save Drive couldn't take is kept through
// instead, which also has Saves made as the shell starts.
import qs
import Quickshell
import QtQuick

Provider {
    id: provider

    what: "document"

    // Documents' rows and their drawers as the daemon serves them, each
    // drawer a record of its own (`drawer:documents:<id>`), so a row
    // carries none of what its drawer alone says. Every drawer is held, a
    // few dozen records with no clock in them, so one is there in the
    // frame its row is selected rather than asked for then. Watched
    // whether or not the launcher's open, so it opens on rows already
    // held, and a change while shut only keeps its record.
    ReadyRows {
        id: ready

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/google.sock`
        place: "documents"
        beside: ["drawer"]
        // What Google held may not be Joel's to see once it answers again.
        keptThroughGap: false
        maker: provider
        drawn: provider.launcher?.open ?? false
        // A document, of its kind, so a program's Recent finds its own;
        // opened once the launcher's gone, so its window takes the keys.
        make: (row, key) => ready.thing(row, key, {
            what: "document",
            component: "document",
            kind: String(row.kind ?? ""),
            hue: row.hue || "program",
            later: true
        })
    }
    // Named, so tests/documents.qml tells it records as the daemon would.
    readonly property alias served: ready

    // A document another place serves as a row of its own, as a meeting's
    // attachment, is this one's row (ReadyRows.qml's lentOf), made as it's
    // asked for, so it's one row with its drawer wherever it shows; null
    // for one this doesn't hold. `lentRevision` moves as its rows do.
    function lent(thing) {
        const id = String(thing).startsWith("gdoc:") ? String(thing).slice(5) : "";
        const key = `row:documents:${id}`;
        return id !== "" && ready.served(key) ? ready.rowAt(key) : null;
    }
    readonly property int lentRevision: ready.placings

    // Its rows, made only while the launcher is open or its rows are
    // wanted (Provider.qml's wanted), as a walk of every row is, so shut
    // it gives the rows it made last and a change wakes nothing. Open is
    // wider than wanted, since a program's Recent and Programs' counts
    // read them from Applications' steps (recentOf).
    readonly property bool rowsWanted: provider.launcher?.open === true || provider.wanted
    readonly property var madeRows: ({ rows: [] })
    things: {
        if (!provider.rowsWanted)
            return provider.madeRows.rows;
        const rows = ready.rows();
        provider.madeRows.rows = rows;
        return rows;
    }
    onThingsChanged: changedTo(things)

    // A program's Recent: its kind's documents, the one he opened last
    // first, or none for a program that opens no document.
    function recentOf(program) {
        const kind = Drive.kindOf(program);
        return kind === "" ? [] : things.filter(row => row.kind === kind);
    }

    // Whether a program's Recent is this one's to give, so it stands while
    // Drive hasn't answered or holds none of its kind.
    function keepsRecentOf(program) {
        return Drive.kindOf(program) !== "";
    }

    // Google Drive as a place in the launcher's picker (Drive.qml).
    function chooserPlaces(chooser) {
        return Drive.chooserPlaces(chooser, provider);
    }

    Binding {
        target: Saves
        property: "launcher"
        value: provider.launcher
    }
}
