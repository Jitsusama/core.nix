// Slack's daemon, for tests/read-there.qml: what it holds is what the
// case says through SlackSays, always answering, as the shell's
// Daemon.qml serves a daemon's records.
import qs
import QtQuick

QtObject {
    required property string socket
    required property var kind
    property bool keptThroughGap: true
    readonly property string state: "answering"
    readonly property var held: SlackSays.held
    readonly property int revision: SlackSays.revision
}
