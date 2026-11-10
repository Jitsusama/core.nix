// What the stand-in Slack daemon holds (Daemon.qml beside it), set by
// tests/read-there.qml, by key as the shell's Daemon.qml holds records.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property var held: ({})
    property int revision: 0
}
