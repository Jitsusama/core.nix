// pi-verbs and the clipboard daemon, for On the Selection
// (tests/on-selection-keys.qml): one Daemon for both, live, answering an
// ask at once with an explanation of the selection it was given, while
// what it's told or asked to find goes unanswered.
import QtQuick

QtObject {
    property string socket: ""
    property string kind: ""
    property bool connects: false
    property bool live: true
    readonly property var held: ({})
    property var wire: null
    signal told(var answer)
    function ask(said, then) {
        then({ ok: true, system: "Explain.", prompt: said.selection, use: "use-1" });
    }
    function tell(said) {}
    function find(said, then) {}
}
