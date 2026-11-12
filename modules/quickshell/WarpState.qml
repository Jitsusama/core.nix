pragma Singleton
// How WARP stands, as the warp daemon tells it (modules/warp/src/serve.rs):
// its `warp:state` record, WARP's status and reason or the error warp-cli
// gives, and when that began where the daemon saw it begin. The margin's
// Work Network piece says from it when the work network ails
// (pieces/WorkNetwork.qml). Null until the daemon says one, so a machine
// with no warp-cli, and no daemon, has nothing to say.
//
// The shell holds the daemon's socket from the moment it starts, as it
// holds every daemon's (Daemon.qml), and watches this kind always. That
// costs nothing while all is well: the daemon only holds a warp-cli
// listening, which writes a line when WARP's state changes, and reads
// none of the link's figures for it, which it does only while the
// launcher shows the Work Network (launcher/Warp.qml).
import Quickshell
import QtQuick

Singleton {
    id: root

    // The record as the daemon last told it, or null.
    property var state: null
    readonly property bool present: state !== null

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/warp.sock`
        kind: "warp-state"
        // A daemon that isn't answering says nothing of how WARP stands,
        // so no old state is held as current.
        keptThroughGap: false
        onHeard: (changed, gone) => {
            if (!changed.includes("warp:state") && !gone.includes("warp:state"))
                return;
            root.state = daemon.held["warp:state"] ?? null;
            // What it heard, so a test sees the margin's half apart from
            // the daemon's.
            Report.say(`warp stands ${JSON.stringify(root.state?.said ?? null)}`);
        }
    }
}
