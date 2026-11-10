// Slack's daemon, for tests/read-there.qml: what it holds is what the
// case says through SlackSays, always answering, and each change is heard
// with the keys it changed and let go, as the shell's Daemon.qml serves a
// daemon's records.
import qs
import QtQuick

QtObject {
    id: daemon

    required property string socket
    required property var kind
    property bool keptThroughGap: true
    readonly property string state: "answering"
    readonly property var held: SlackSays.held
    readonly property int revision: SlackSays.revision

    signal heard(var changed, var gone)

    Component.onCompleted: SlackSays.heard.connect((changed, gone) => daemon.heard(changed, gone))
}
