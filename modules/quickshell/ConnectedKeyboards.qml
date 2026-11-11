// The keyboards connected now, the one list every surface that names a
// keyboard reads (VKCF3Z, 2026-11-07): the layout's map, the launcher's
// Keyboard collection and its Keyboards place. The machine daemon keeps
// it (modules/machine/src/keyboards.rs), reading the kernel's list of
// input devices again each time the keyboard-lights watcher says the
// keyboards it holds have changed, so a board plugged in or out is heard
// at once, nothing is read on a clock, and the shell reads no file for
// it. A board that isn't connected is in none of them (Joel, 2026-11-07:
// "we shouldn't be showing keyboard shortcuts for keyboards that are not
// plugged in"), and none is while the daemon isn't answering.
pragma Singleton
import Quickshell
import QtQuick
import "launcher/Keyboards.js" as Keyboards

Singleton {
    id: root

    // Each keyboard as the kernel says it, in its own order.
    readonly property var listed: daemon.revision >= 0
        ? daemon.held["keyboards:connected"]?.listed ?? [] : []
    // Joel's own board, ZSA's, or null while it isn't connected.
    readonly property var board: listed.find(Keyboards.zsa) ?? null

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "keyboards"
        keptThroughGap: false
        answersWhole: true
    }
}
