// What the desktop's watchers say of themselves: lag-watch, of whether
// what Joel uses waits (modules/lag-watch), and the disk guards, the
// account's and the machine's, of the disks (modules/disk-guard). The
// machine daemon reads each one's status file as it's renamed into place
// (modules/machine/src/status.rs), leaves out the heartbeat that says
// only that the watcher still runs, and says once it's gone silent, so
// nothing here reads a file or keeps a timer, and a write that changes
// nothing else reaches no binding. A guard gone silent comes with how
// its unit stands, asked by the daemon of systemd once, so Disk spawns no
// systemctl, and what Disk asks of the account's guard, Keep, Release
// and Clear, goes over the same connection as acts the daemon carries
// out. One connection holds all three, watched always, since Lag and
// Disk are in the margin whatever's shown.
pragma Singleton
import Quickshell
import QtQuick

Singleton {
    id: root

    // What each says, or null while it says nothing that reads: no file,
    // one that doesn't parse, or a daemon not answering.
    property var lag: null
    property var guard: null
    property var machineGuard: null
    // Whether each has gone silent, no heartbeat for as long as it beats.
    property bool lagSilent: false
    property bool guardSilent: false
    property bool machineGuardSilent: false
    // How each guard's unit stood as it fell silent, { stopped, state,
    // result }, or null while it isn't silent.
    property var guardUnit: null
    property var machineGuardUnit: null

    // Each watcher's property, by its record's key.
    readonly property var named: ({
        "status:lag-watch": "lag",
        "status:disk-guard": "guard",
        "status:disk-guard-machine": "machineGuard"
    })

    // Only the watchers a message said are set, so a guard's write leaves
    // Lag's bindings be.
    function heard(changed, gone) {
        for (const key of changed.concat(gone)) {
            const name = named[key];
            if (name === undefined)
                continue;
            const told = daemon.held[key] ?? null;
            root[name] = told?.said ?? null;
            root[`${name}Silent`] = told?.silent === true;
            if (name !== "lag")
                root[`${name}Unit`] = told?.unit ?? null;
        }
    }

    // Asks the daemon to act, as Disk's Keep does; `then` is given its
    // answer, or null when it isn't there to ask.
    function act(said, then) {
        daemon.ask(Object.assign({ ask: "act" }, said), then);
    }

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "status"
        keptThroughGap: false
        answersWhole: true
        onHeard: (changed, gone) => root.heard(changed, gone)
    }
}
