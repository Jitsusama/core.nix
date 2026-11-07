pragma Singleton
// What spend-watch (modules/spend-watch) writes for the launcher, its
// collection.json: the collection on Super+U and its answers (the launcher's
// Spend) and what each pi session spent today (its Agents). Every figure and
// word is the daemon's, so this only reads it: null with no file, or one of
// a version it doesn't know.
//
// The daemon replaces the file whole when something in it changed, which
// the watch hears, since it watches the folder as well, or the nearest one
// above it that's there (nixos.nix's patch), so it hears the daemon start
// after the shell too. A file from a daemon of another shape draws nothing rather than
// something wrong.
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    property var said: null
    // What each pi session spent today, by its whole id.
    readonly property var sessions: said?.sessions ?? {}
    // How each one's day went, by the same id: { graph, rebuilt }, its
    // hours as Graph's spark, carrying no dollars, and what rebuilding its
    // context cost, "" under a cent.
    readonly property var sessionsToday: said?.sessionsToday ?? {}
    // What each one's councils, judges and fleets cost today, costliest
    // first: [{ on, spent }].
    readonly property var helpers: said?.helpers ?? {}
    // What each quest spent today with every quest under it, by its id:
    // { all, own, under }, so an orchestrator's row says its whole
    // orchestration's without the shell adding anything up.
    readonly property var families: said?.families ?? {}

    function parsed(text) {
        try {
            const value = JSON.parse(text);
            return typeof value === "object" && value !== null && value.v === 2 ? value : null;
        } catch (error) {
            return null;
        }
    }

    FileView {
        id: file
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/spend-watch/collection.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        // What was read last, so a reading that changed nothing says nothing.
        property string read: ""
        onLoaded: {
            if (file.text() === file.read)
                return;
            file.read = file.text();
            root.said = root.parsed(file.read);
        }
        onLoadFailed: {
            file.read = "";
            root.said = null;
        }
    }
}
