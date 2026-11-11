pragma Singleton
// Keeping the machine awake, so work left running goes on: two user services
// hold logind's locks, one against going idle and one against the lid, and
// each leaves a file in the runtime directory while it runs, which the
// machine daemon hears and says as `kept:idle` and `kept:lid` over the socket
// the shell already holds (modules/machine/src/kept.rs), so the shell
// watches no file of its own. Neither holds the lock against sleep,
// which logind's own handling of the power key honours, so a suspend Joel
// asks for with it still goes through (home.nix says why only that one
// would be held back). The lid is held on battery as on the charger, since
// Joel keeps it awake to run with no power and the lid shut (2026-11-10),
// and a battery run nearly flat still powers off. Where a shut lid never
// suspends, it's never held, since holding it would change nothing.
import Quickshell
import QtQuick

Singleton {
    id: root

    readonly property bool on: machine.revision >= 0 && machine.held["kept:idle"]?.held === true
    // Whether the lid is held: only where a shut lid would suspend
    // (Options.lidSleeps).
    readonly property bool lid: machine.revision >= 0
        && machine.held["kept:lid"]?.held === true
    readonly property bool lidHolds: Options.lidSleeps
    // Whether shutting the lid now leaves it running too: held, or where a
    // shut lid never suspends.
    readonly property bool lidShutToo: root.lidHolds ? root.lid : true

    function keep() {
        Niri.spawn(["systemctl", "--user", "start", "keep-awake.service"]
            .concat(root.lidHolds ? ["keep-awake-lid.service"] : []));
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

    // Watched always, as the files were: the daemon hears them as they're
    // written, so the watch wakes nothing until a lock's taken or let go.
    Daemon {
        id: machine

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "kept"
    }
}
