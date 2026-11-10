// pi's sessions, for the launcher's Now (tests/now-moment.qml): where an
// item is done names its session, and For You lists the sessions wanting
// Joel, none of them parked, and those to reopen, each as the case gives.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property var running: []
    property var wanting: []
    property var toReopen: []

    function wanted(wants) {
        return "needs you";
    }

    function goTo(session) {}

    function parkedAs(address) {
        return null;
    }
}
