pragma Singleton
// The home network as the eero daemon keeps it (modules/eero), its whole
// record, each key documented in modules/eero/src/model.rs: the internet,
// the nodes, the devices on them, the profiles and what happened. The
// launcher's Home Network place draws from it (launcher/HomeNetwork.qml).
// Every figure is eero's, so this only holds it: null until the daemon
// says one, or when what it says isn't the record, so a foreign one
// draws nothing rather than something wrong.
//
// The shell holds the daemon's socket, $XDG_RUNTIME_DIR/eero/socket, from
// the moment it starts, as it holds every daemon's (Daemon.qml), and is
// told the record as one `home-network` record on each change; nothing
// here reads a file or polls. While the Home Network is open (`watching`),
// it also watches the record ahead, which makes the daemon read eero
// every 30 seconds rather than every five minutes, and `read` asks it to
// read now. Through a gap the record held stays, as the daemon's file
// would have.
import Quickshell
import QtQuick
import "launcher/HomeNetwork.js" as Home

Singleton {
    id: root

    // The record as eero last answered, or null.
    property var model: null
    readonly property bool present: model !== null
    // Each device's signal and speed as each record said them, since the
    // shell started, for the drawers' sparks (HomeNetwork.js's
    // remembered): twenty readings at most, Graph's whole width.
    property var history: ({})
    readonly property int remembers: 20
    onModelChanged: history = model === null ? history : Home.remembered(history, model,
        remembers)
    // Whether the Home Network is open, so the daemon is held to read
    // often; the place sets it.
    property bool watching: false

    // The record, if it's whole: the internet, the nodes and the devices.
    function whole(value) {
        return typeof value === "object" && value !== null
            && typeof value.internet === "object" && Array.isArray(value.nodes)
            && Array.isArray(value.devices) ? value : null;
    }

    // Asks the daemon to read eero now, as after a speed test or a change
    // of setting; nothing while it isn't answering.
    function readNow() {
        daemon.ask({ ask: "read" }, null);
    }

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/eero/socket`
        kind: "home-network"
        ahead: root.watching ? ["home-network"] : []
        onHeard: changed => {
            if (changed.includes("home-network"))
                root.model = root.whole(daemon.held["home-network"].network);
        }
    }
}
