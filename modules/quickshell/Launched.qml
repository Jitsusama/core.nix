// Whether the launcher's card is up, open or drawing its first frame
// (shell.qml sets it from Launcher.qml's cardUp), so what else lives on the
// screen beside it, as the margin's cat does (modules/margin-cat), keeps
// still while Joel's keys are in it without reaching into the launcher.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property bool up: false
}
