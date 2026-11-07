pragma Singleton
// The home network as the eero daemon keeps it (modules/eero), its whole
// record in $XDG_RUNTIME_DIR/eero/home-network.json, each key documented
// in modules/eero/src/model.rs: the internet, the nodes, the devices on
// them, the profiles and what happened. The launcher's Home Network place
// draws from it (launcher/HomeNetwork.qml). Every figure is eero's, so
// this only reads it: null with no file, or with one that isn't the
// record, so a half-written or foreign file draws nothing rather than
// something wrong.
//
// The daemon replaces the file whole when what eero says changes, which
// the watch hears, since it watches the folder too, or the nearest one
// above it that's there (nixos.nix's patch), so it hears the daemon start
// after the shell. Nothing here polls.
//
// While the Home Network is open (`watching`), the shell also holds the
// daemon's socket, $XDG_RUNTIME_DIR/eero/socket, which makes the daemon
// read eero every 30 seconds rather than every five minutes: it says the
// whole record as one line once connected and again on each change, and
// `read` asks it to read now. The socket's word wins while it's held, and
// the file's comes back once it's let go. A daemon with no socket leaves
// the file to say it all.
import Quickshell
import Quickshell.Io
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

    function parsed(text) {
        try {
            const value = JSON.parse(text);
            const whole = typeof value === "object" && value !== null
                && typeof value.internet === "object" && Array.isArray(value.nodes)
                && Array.isArray(value.devices);
            return whole ? value : null;
        } catch (error) {
            return null;
        }
    }

    // Asks the daemon to read eero now, as after a speed test or a change
    // of setting, where it's held; nothing where it isn't.
    function readNow() {
        if (!socket.connected)
            return;
        socket.write("read\n");
        socket.flush();
    }

    Socket {
        id: socket
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/eero/socket`
        connected: root.watching
        parser: SplitParser {
            onRead: line => {
                const said = root.parsed(line);
                if (said !== null)
                    root.model = said;
            }
        }
        // Let go, the record is the file's again, which the daemon keeps as
        // current as what it told the socket.
        onConnectionStateChanged: {
            if (!connected)
                file.reload();
        }
    }

    FileView {
        id: file
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/eero/home-network.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        // What was read last, so a reading that changed nothing says nothing.
        property string read: ""
        onLoaded: {
            if (file.text() === file.read && root.model !== null)
                return;
            file.read = file.text();
            root.model = root.parsed(file.read);
        }
        onLoadFailed: {
            file.read = "";
            root.model = null;
        }
    }
}
