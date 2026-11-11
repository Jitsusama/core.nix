// The keyboard's layer, as a board's watcher writes it to keyboard-layer in
// the runtime directory (modules/ergodox/nixos.nix), told by the machine
// daemon as it's written (modules/machine/src/keyboards.rs), so the shell
// reads no file for it: the layer's name, and whether it's locked there
// rather than held by a key. There's no layer while no board is plugged
// in, with no watcher, as on a Mac, or while the daemon isn't answering.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    readonly property var said: daemon.revision >= 0 ? daemon.held["layer:keyboard"] ?? null
        : null
    readonly property bool present: said !== null
    readonly property bool locked: present && said.locked === true
    readonly property string name: present ? String(said.name ?? "") : ""

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "layer"
        keptThroughGap: false
        answersWhole: true
    }
}
