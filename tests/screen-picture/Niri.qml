// A niri that refuses every request, for tests/screen-picture/shell.qml,
// so the picture is read at its screen's own ratio, as it is when niri
// doesn't answer.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    function request(asked: var, answer: var, refused: var): void {
        Qt.callLater(refused);
    }
}
