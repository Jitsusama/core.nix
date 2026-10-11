pragma Singleton
// Keeping the machine awake, so work left running goes on: two user services
// hold logind's locks, one against sleeping and one against the lid, and each
// leaves a file in the runtime directory while it runs, which is how the
// shell knows without asking anything. The lid is held only while there's
// power, since a machine shut in a bag on its battery would cook: when the
// power goes the lid is let go, and taken again when it comes back.
import Quickshell
import Quickshell.Io
import Quickshell.Services.UPower
import QtQuick

Singleton {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property bool on: awake.present
    // Whether shutting the lid leaves it running too.
    readonly property bool lid: shut.present
    readonly property bool plugged: !UPower.onBattery

    function keep() {
        Niri.spawn(["systemctl", "--user", "start", "caffeinate.service"]
            .concat(plugged ? ["caffeinate-lid.service"] : []));
    }

    function release() {
        Niri.spawn(["systemctl", "--user", "stop", "caffeinate.service"]);
    }

    onPluggedChanged: {
        if (on)
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
