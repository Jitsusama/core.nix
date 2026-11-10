pragma Singleton
// Keeping the machine awake, so work left running goes on: two user services
// hold logind's locks, one against going idle and one against the lid, and
// each leaves a file in the runtime directory while it runs, which the
// machine daemon hears and says as `kept:idle` and `kept:lid` over the socket
// the shell already holds (modules/machine/src/kept.rs), so the shell
// watches no file of its own. Neither holds the lock against sleep,
// which logind's own handling of the power key honours, so a suspend Joel
// asks for with it still goes through (home.nix says why only that one
// would be held back). The lid is held only while there's
// power, since a machine shut in a bag on its battery would cook: when the
// power goes the lid is let go, and taken again when it comes back. Where
// a shut lid on the charger never suspends, as with the lid module, it's
// never held, since holding it would change nothing (M4RL61, 2026-11-03).
import Quickshell
import Quickshell.Services.UPower
import QtQuick

Singleton {
    id: root

    readonly property bool on: machine.revision >= 0 && machine.held["kept:idle"]?.held === true
    // Whether the lid is held: only where a shut lid on the charger would
    // suspend (Options.lidSleepsOnPower), and only on the charger.
    readonly property bool lid: machine.revision >= 0
        && machine.held["kept:lid"]?.held === true
    readonly property bool plugged: !UPower.onBattery
    readonly property bool lidHolds: Options.lidSleepsOnPower
    // Whether shutting the lid now leaves it running too: held, or on the
    // charger where a shut lid never suspends.
    readonly property bool lidShutToo: root.lidHolds ? root.lid : root.plugged

    function keep() {
        Niri.spawn(["systemctl", "--user", "start", "keep-awake.service"]
            .concat(root.plugged && root.lidHolds ? ["keep-awake-lid.service"] : []));
    }

    // Keeps it awake, but lets shutting the lid sleep it.
    function open() {
        Niri.spawn(["systemctl", "--user", "stop", "keep-awake-lid.service"]);
    }

    // Keeps it awake from asleep or awake alike, only while the lid is open,
    // as :set awake=open does.
    function keepOpen() {
        Niri.spawn(["systemctl", "--user", "start", "keep-awake.service"]);
        open();
    }

    function release() {
        Niri.spawn(["systemctl", "--user", "stop", "keep-awake.service"]);
    }

    onPluggedChanged: {
        if (root.on && root.lidHolds)
            Niri.spawn(["systemctl", "--user", plugged ? "start" : "stop",
                "keep-awake-lid.service"]);
    }

    // Watched always, as the files were: the daemon hears them as they're
    // written, so the watch wakes nothing until a lock's taken or let go.
    Daemon {
        id: machine

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "kept"
    }
}
