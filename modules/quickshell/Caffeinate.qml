pragma Singleton
// Keeping the machine awake, so work left running goes on: two user services
// hold logind's locks, one against going idle and one against the lid, and
// each leaves a file in the runtime directory while it runs, which is how the
// shell knows without asking anything. Neither holds the lock against sleep,
// so a suspend Joel asks for with the power key or the launcher still goes
// through. The lid is held only while there's
// power, since a machine shut in a bag on its battery would cook: when the
// power goes the lid is let go, and taken again when it comes back. Where
// a shut lid on the charger never suspends, as with the lid module, it's
// never held, since holding it would change nothing (M4RL61, 2026-11-03).
import Quickshell
import Quickshell.Io
import Quickshell.Services.UPower
import QtQuick

Singleton {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property bool on: awake.present
    // Whether the lid is held: only where a shut lid on the charger would
    // suspend (Options.lidSleepsOnPower), and only on the charger.
    readonly property bool lid: shut.present
    readonly property bool plugged: !UPower.onBattery
    readonly property bool lidHolds: Options.lidSleepsOnPower
    // Whether shutting the lid now leaves it running too: held, or on the
    // charger where a shut lid never suspends.
    readonly property bool lidShutToo: root.lidHolds ? root.lid : root.plugged

    function keep() {
        Niri.spawn(["systemctl", "--user", "start", "caffeinate.service"]
            .concat(root.plugged && root.lidHolds ? ["caffeinate-lid.service"] : []));
    }

    // Keeps it awake, but lets shutting the lid sleep it.
    function open() {
        Niri.spawn(["systemctl", "--user", "stop", "caffeinate-lid.service"]);
    }

    function release() {
        Niri.spawn(["systemctl", "--user", "stop", "caffeinate.service"]);
    }

    onPluggedChanged: {
        if (root.on && root.lidHolds)
            Niri.spawn(["systemctl", "--user", plugged ? "start" : "stop",
                "caffeinate-lid.service"]);
    }

    FileView {
        id: awake
        property bool present: false
        path: `${root.runtime}/caffeinated`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: present = true
        onLoadFailed: present = false
    }

    FileView {
        id: shut
        property bool present: false
        path: `${root.runtime}/caffeinated-lid`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: present = true
        onLoadFailed: present = false
    }
}
