// Google Drive's program in Programs, which holds Drive's own places: its
// Recent, the nine files of every kind Joel opened last, newest first
// (`recentOf`, which Applications.qml asks every provider that has it),
// and Starred, Shared with Me and Shared Drives, each saying how many it
// holds beside it and stepping in to them (`programActs`), a place
// holding none not listed, as an empty Recent isn't. Each file is the one
// row every surface draws a Drive file with (Drive.row): Return opens it
// in its own window, Drive's or, for one of Google's own kinds, its
// program's, and the right arrow lists Open and Copy the Link; a shared
// drive opens as Drive's page of it.
//
// The places come from the google program, which reads them every five
// minutes with the documents (src/watch.rs), so stepping in asks Google
// nothing. The places sit where Slack's Unread does, after the program's
// own acts and before its Recent, since Applications.qml lists what a
// provider gives a program there (controls).
import qs
import QtQuick

Provider {
    id: provider

    what: "document"

    // The desktop entry's id Drive's program row is made from.
    readonly property string program: "google-drive"

    // What the places say, so a program stepped into is drawn again only
    // as they change, never as a picker's listing or a download moves the
    // rest of Drive's state.
    readonly property string placed: JSON.stringify(Drive.places)
    onPlacedChanged: changed()

    function recentOf(asked) {
        return Drive.kindOf(asked) === Drive.every ? Drive.recent(provider) : [];
    }

    function programActs(asked) {
        return asked === program ? Drive.placeRows(provider) : [];
    }

    // What its drawer in Programs says it holds (Applications.qml asks
    // every provider's programFacts), from the places read with the
    // documents.
    function programFacts(asked) {
        return asked === program ? Drive.placeFacts() : [];
    }
}
