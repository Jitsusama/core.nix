// niri, for the launcher's Screens (tests/displays.qml): the screens come
// from the windows daemon, which the test stands in, so niri here is only
// what Screens still asks of it itself, a change to a screen, answered as
// applied and counted, and the workspace in front.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property int revision: 0
    readonly property bool powersOffEach: false
    // The changes asked of niri, as it was asked them.
    property var asked: []

    function workspace() {
        return { output: "eDP-1" };
    }

    function act(request) {}

    function spawn(command) {}

    function request(change, answer) {
        asked = asked.concat([change]);
        answer({ OutputConfigChanged: "Applied" });
    }
}
