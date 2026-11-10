// The keyboard's lights. Whether caps lock is on, as the system's
// keyboard-lights watcher writes it (modules/quickshell/nixos.nix), told by
// the machine daemon as it's written (modules/machine/src/keyboards.rs), so
// nothing is asked and the shell reads no file for it. With no watcher, as
// on a Mac, or while the daemon isn't answering, it's never on.
//
// And the keyboard's backlight, whenever its firmware changes it, as the
// backlight key does on a Dell: the kernel writes brightness_hw_changed
// then and only then (LEDS_BRIGHTNESS_HW_CHANGED), so the light is heard
// changing by the key and never read on a clock. A kernel without it, or a
// keyboard light whose driver never says, is never heard.
pragma Singleton
import Quickshell
import Quickshell.Io
import Qt.labs.folderlistmodel
import QtQuick

Singleton {
    id: root

    readonly property bool caps: capslock.revision >= 0
        && capslock.held["capslock:keyboard"]?.on === true

    // Where the kernel lists its lights; a test lists its own.
    property string leds: "/sys/class/leds"
    // The keyboard's backlight, or "" without one.
    readonly property string backlight: {
        for (let i = 0; i < found.count; i++) {
            const name = found.get(i, "fileName");
            if (/kbd_backlight$/.test(name))
                return `${root.leds}/${name}`;
        }
        return "";
    }
    // How lit it was left, as a share, and whether it's off.
    property real lit: 0
    property bool dark: false
    // Says the firmware has just changed it.
    signal turned

    // The level last heard, and when. The file watcher can say one change
    // twice, and the second reads the same level within moments; a key is
    // never pressed that fast, so that's the same change, heard once.
    property int heardLevel: -1
    property real heardAt: 0
    readonly property int sameChange: 200

    Daemon {
        id: capslock

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "capslock"
        keptThroughGap: false
        answersWhole: true
    }

    FolderListModel {
        id: found
        folder: "file://" + root.leds
        showFiles: false
        showDotAndDotDot: false
    }

    FileView {
        id: most
        path: root.backlight === "" ? "" : `${root.backlight}/max_brightness`
        blockAllReads: true
        printErrors: false
    }

    // Read before the firmware has changed it, the file says nothing, which
    // is why it's only read once it changes. Every read blocks, since a
    // reload otherwise reads in the background and text() still says the
    // last.
    FileView {
        id: changed
        path: root.backlight === "" ? "" : `${root.backlight}/brightness_hw_changed`
        watchChanges: true
        blockAllReads: true
        printErrors: false
        onFileChanged: {
            reload();
            const now = parseInt(text());
            const steps = parseInt(most.text());
            if (isNaN(now) || isNaN(steps) || steps <= 0)
                return;
            const at = Date.now();
            if (now === root.heardLevel && at - root.heardAt < root.sameChange)
                return;
            root.heardLevel = now;
            root.heardAt = at;
            root.lit = Math.min(now, steps) / steps;
            root.dark = now === 0;
            root.turned();
        }
    }
}
