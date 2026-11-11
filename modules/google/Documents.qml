// The documents Joel looked at lately in the launcher: each found by its
// title from anywhere, and each program's own, Docs', Sheets' or Slides',
// as that program's Recent (recentOf, which Applications.qml asks every
// provider that has it), the same row in both. Each is the row the google
// daemon serves ready (src/document_rows.rs), worded, ordered and with its
// acts, so the shell only places it and words its time (Drive.qml's
// readyRow). It also gives the launcher's picker Google Drive as a place
// (chooserPlaces), and gives Saves.qml the launcher, whose picker a save
// Drive couldn't take is kept through instead, which also has Saves made
// as the shell starts.
import qs
import Quickshell
import QtQuick
import "Placed.js" as Placed

Provider {
    id: provider

    what: "document"

    // Documents' rows and their drawers as the daemon serves them, each
    // record `{ key, kind, row }` or `{ key, kind, drawer }` held by key.
    // Every drawer is held, a few dozen records with no clock in them, so
    // one is there in the frame its row is selected rather than asked for
    // then; a row carries none of what its drawer alone says. Watched
    // whether or not the launcher's open, so it opens on rows already
    // held, and a change while shut only keeps its record.
    Daemon {
        id: served

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/google.sock`
        kind: ["row", "drawer"]
        watching: ({ place: "documents" })
        answersWhole: true
        // What Google held may not be Joel's to see once it answers again.
        keptThroughGap: false
        onHeard: (changed, gone) => provider.take(changed, gone)
    }
    // Named, so tests/documents.qml tells it records as the daemon would.
    readonly property alias daemon: served

    // The rows' keys in the order the daemon says (Placed.js), and a count
    // of the changes taken, since the daemon's revision moves before it's
    // heard.
    property var placed: []
    property int taken: 0

    function take(changed, gone) {
        const rows = {};
        for (const key of placed.concat(changed)) {
            const row = served.held[key]?.row;
            if (row)
                rows[key] = row;
        }
        placed = Placed.placed(placed, rows, changed.filter(key => rows[key]), gone);
        taken += 1;
    }

    // Its rows, made only while the launcher is open or its rows are
    // wanted (Provider.qml's wanted), as a walk of every row is. Each says
    // when it was opened against the clock, so shut it gives the rows it
    // made last and reads nothing else, so no change wakes it. Open is
    // wider than wanted, since a program's Recent and Programs' counts
    // read them from Applications' steps (recentOf). A row is made again
    // only once its record or what it says at its right changed, so an
    // opening makes only those.
    readonly property bool rowsWanted: provider.launcher?.open === true || provider.wanted
    readonly property var madeRows: ({ rows: [] })
    readonly property var keeping: provider.keeper()
    things: {
        if (!provider.rowsWanted)
            return provider.madeRows.rows;
        void provider.taken;
        provider.keepingBegun(provider.keeping);
        const rows = provider.placed.map(key => {
            const ready = served.held[key]?.row;
            return provider.keptOr(provider.keeping, key,
                JSON.stringify([ready, Drive.readySide(ready)]),
                () => Drive.readyRow(ready, provider, drawerKey => provider.drawerOf(
                    drawerKey)));
        });
        provider.keptOnly(provider.keeping);
        provider.madeRows.rows = rows;
        return rows;
    }
    onThingsChanged: changedTo(things)

    // A row's drawer as the launcher shows it, from the record the daemon
    // made ready, worded again only once the clock or the record moved.
    readonly property var drawn: ({})
    function drawerOf(key) {
        void provider.taken;
        const drawer = served.held[key]?.drawer ?? null;
        if (drawer === null)
            return null;
        const was = drawn[key];
        if (was && was.drawer === drawer && was.now === Calendar.now)
            return was.shown;
        const shown = Drive.readyDrawer(drawer);
        drawn[key] = { drawer, now: Calendar.now, shown };
        return shown;
    }

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
