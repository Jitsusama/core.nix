// The keyboards connected now, the one list every surface that names a
// keyboard reads (VKCF3Z, 2026-11-07): the layout's map, the launcher's
// Keyboard collection and its Keyboards place. It's the kernel's list of
// input devices, as Keyboards.js reads it, read again each time the
// keyboard-lights watcher says the keyboards it holds have changed
// (modules/quickshell/nixos.nix), so a board plugged in or out is heard at
// once and nothing is read on a clock. A board that isn't connected is in
// none of them (Joel, 2026-11-07: "we shouldn't be showing keyboard
// shortcuts for keyboards that are not plugged in").
pragma Singleton
import Quickshell
import Quickshell.Io
import QtQuick
import "launcher/Keyboards.js" as Keyboards

Singleton {
    id: root

    // Where the kernel lists its input devices and the watcher its
    // keyboards; a test lists its own.
    property string devices: "/proc/bus/input/devices"
    property string held: "/run/keyboard-lights/keyboards"

    // The kernel's list as last read.
    property string read: ""

    readonly property var listed: Keyboards.keyboards(read)
    // Joel's own board, ZSA's, or null while it isn't connected.
    readonly property var board: listed.find(Keyboards.zsa) ?? null

    function reread() {
        kernel.reload();
        kernel.waitForJob();
        root.read = kernel.text();
    }

    FileView {
        id: kernel
        path: root.devices
        blockLoading: true
        printErrors: false
    }

    // Read whenever the watcher's list changes, and once at the start, so a
    // machine without the watcher still lists what was there then.
    FileView {
        path: root.held
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: root.reread()
        onLoadFailed: root.reread()
    }
}
