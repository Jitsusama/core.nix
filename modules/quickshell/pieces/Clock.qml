// The time or the date, in the format its settings give, as Qt spells one:
// https://doc.qt.io/qt-6/qml-qtqml-qt.html#formatDateTime-method
import Quickshell
import QtQuick

Piece {
    id: piece

    value: Qt.formatDateTime(clock.date, settings.format ?? "HH:mm")

    SystemClock {
        id: clock
        precision: (piece.settings.format ?? "").includes("s") ? SystemClock.Seconds : SystemClock.Minutes
    }
}
