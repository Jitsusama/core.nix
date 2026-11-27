// Drives On the Selection's place (modules/quickshell/launcher/OnSelection.qml)
// through Quickshell itself, over a pi-verbs record keyed as one of the
// launcher's own kinds, with pi-verbs, the clipboard daemon and Ask's warm
// pi stood in. Its row is kept under the node's row:, never as the
// launcher's own kind, while what it posed is held by the record's own key,
// and once Ask answers, the answer standing in the row's place keeps the
// row's key.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    readonly property var same: Expect.same

    // Ask's place as On the Selection finds it among the launcher's
    // sources, answering what it's posed only as a case says.
    QtObject {
        id: asker

        readonly property var settings: ({ name: "ask" })
        property string posed: ""
        property string stage: ""
        property string said: ""
        signal answered

        function pose(message) {
            posed = message;
        }
    }

    // The launcher open, standing in the place.
    QtObject {
        id: theLauncher

        property bool open: true
        readonly property var held: [{ row: { from: selection } }]
        readonly property var sources: [asker]

        function wants(source) {
            return true;
        }
    }

    Launcher.OnSelection {
        id: selection

        launcher: theLauncher
        settings: ({ name: "selection" })
    }

    Component.onCompleted: Qt.callLater(() => Expect.run(root.cases))

    function cases() {
        selection.selected = "the words";
        selection.listed = [{ key: "place:x", kind: "verb", id: "explain", words: "Explain It",
            says: "says what it means", result: "say" }];
        const row = selection.everything()[0];
        same("a record claiming a place is kept under the node's row:", row.key, "row:place:x");

        // Return poses it to Ask, held by the record's own key.
        row.run();
        same("what it posed is held by the record's key", Object.keys(selection.posed),
            ["place:x"]);
        same("and posed to Ask", asker.posed, selection.posed["place:x"].message);

        // Answered, the answer in its row's place keeps the row's key.
        asker.stage = "answered";
        asker.said = "It means this. And more.";
        const answered = selection.everything()[0];
        same("the answer in its place", answered.name, "It means this.");
        same("keeps the row's key", answered.key, "row:place:x");
    }
}
