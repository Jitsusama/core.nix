// The documents Joel looked at lately in the launcher: each found by its
// title from anywhere, and each program's own, Docs', Sheets' or Slides',
// as that program's Recent (recentOf, which Applications.qml asks every
// provider that has it), the same row in both (Drive.row). It also gives
// the launcher's picker Google Drive as a place (chooserPlaces), and gives
// Saves.qml the launcher, whose picker a save Drive couldn't take is kept
// through instead, which also has Saves made as the shell starts.
import qs
import QtQuick

Provider {
    id: provider

    what: "document"

    // Its rows, made only while the launcher is open or its rows are
    // wanted (Provider.qml's wanted), as a walk of every row is. Each row
    // says when it was opened against the clock (Drive.row reads
    // Calendar.now), so every minute remade them all with the launcher
    // shut, 27 a minute on Joel's shell (2026-11-10). Shut, it gives the
    // rows it made last and reads nothing else, so no change wakes it, and
    // it makes them once as the launcher opens. Open is wider than wanted,
    // since a program's Recent and Programs' counts read them from
    // Applications' steps (recentOf).
    readonly property bool rowsWanted: provider.launcher?.open === true || provider.wanted
    readonly property var madeRows: ({ rows: [] })
    // Each row is kept while its document and what it says at its right
    // read the same, so an opening makes only those that changed: every
    // opening made all 27 again on Joel's shell (2026-11-10). A stopgap
    // until the google daemon serves Documents' rows ready (RTOIMQ's
    // ready rows, due 2026-11-11 18:00), which streams only those that
    // changed, and keeping them by key stays.
    readonly property var keeping: provider.keeper()
    things: {
        if (!provider.rowsWanted)
            return provider.madeRows.rows;
        provider.keepingBegun(provider.keeping);
        const rows = Drive.documents.map(document => provider.keptOr(provider.keeping,
            document.id, JSON.stringify([document, Drive.side(document)]),
            () => Drive.row(document, provider)));
        provider.keptOnly(provider.keeping);
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
