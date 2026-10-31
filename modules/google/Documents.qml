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

    things: Drive.documents.map(document => Drive.row(document, provider))
    onThingsChanged: changedTo(things)

    // A program's Recent: its kind's documents, the one he opened last
    // first, or none for a program that opens no document.
    function recentOf(program) {
        const kind = Drive.kindOf(program);
        return kind === "" ? [] : things.filter(row => row.kind === kind);
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
