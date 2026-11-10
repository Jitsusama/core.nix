// The shell's notices, for the launcher's Now (tests/now-moment.qml): the
// history the case gives, For You's Desktop holding the unread as the
// notices' own rows say them (launcher/PastNotices.qml), and no sender with
// a window, so a notice's row goes nowhere but its program.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    property var history: []
    property var live: ({})
    property var resting: ({})
    property var waiting: []

    function markRead(ids) {}
    function seen() {}
    function mark(id, read) {}

    function pictureOf(notice) {
        return "";
    }

    function sender(name) {
        return { glyph: 0xf0f3, hue: "program", name: name };
    }

    function senderWindow(app) {
        return null;
    }
}
