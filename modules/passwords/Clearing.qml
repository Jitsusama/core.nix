// The secret the Passwords place last copied, which is the one on the
// clipboard until the passwords daemon clears it: shared by the place
// (launcher/Passwords.qml), whose foot says how the copy stands when the
// launcher opens again, and the margin's mark (Copied.qml), which stands
// until the copy clears, since the launcher goes as a copy does and so
// can't say it (VKCF3Z's ruling). It holds what the foot says, never the
// secret, which goes from the manager to the clipboard through the daemon.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    id: clearing

    // The last copy asked of the daemon: { key, title, detail, clears,
    // lapses, period, why }, as Passwords.js's told reads it, and the time
    // it's counted down by, moved each second only while it counts.
    property var copied: null
    property real now: Date.now()

    // Whether a copy is on the clipboard, waiting to clear.
    readonly property bool waiting: copied !== null && (copied.why ?? "") === ""
        && (copied.clears ?? 0) > now

    Timer {
        interval: 1000
        repeat: true
        running: clearing.waiting
        onTriggered: clearing.now = Date.now()
    }
}
