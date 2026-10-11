// The keyboard's lights, as the system's keyboard-lights watcher writes them
// (modules/quickshell/nixos.nix): whether caps lock is on. The file changes
// only when the light does, so nothing is asked. With no watcher, as on a
// Mac, it's never on.
pragma Singleton
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    readonly property bool caps: file.loaded && file.text().trim() === "1"

    FileView {
        id: file
        path: "/run/keyboard-lights/capslock"
        printErrors: false
        watchChanges: true
        onFileChanged: reload()
    }
}
