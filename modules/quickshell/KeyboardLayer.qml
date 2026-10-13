// The keyboard's layer, as a keyboard's watcher writes it to keyboard-layer
// in the runtime directory (modules/ergodox/nixos.nix): the layer's name,
// and whether it's locked there rather than held by a key. The file says "-"
// while no board is plugged in, and with no watcher, as on a Mac, there's
// no file and no layer.
pragma Singleton
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    readonly property string line: file.loaded ? file.text().split("\n")[0].trim() : ""
    readonly property bool present: line !== "" && line !== "-"
    readonly property bool locked: present && line.endsWith(" locked")
    readonly property string name: locked ? line.slice(0, -" locked".length) : present ? line : ""

    FileView {
        id: file
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/keyboard-layer`
        printErrors: false
        watchChanges: true
        onFileChanged: reload()
    }
}
