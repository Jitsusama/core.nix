// The shell's notices, for the launcher's Now (tests/now-moment.qml): the
// history the case gives, which Now counts and For You's Desktop holds the
// unread of, their rows served by a stand-in for the notices daemon
// (tests/lib/NoticesDaemon.qml), and no sender with a window.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property var history: []
    property var live: ({})
    property var resting: ({})
    property var waiting: []
    property bool gathered: false

    function markRead(ids) {}
    function seen() {}
    function mark(id, read) {}

    function senderWindow(app) {
        return null;
    }
}
