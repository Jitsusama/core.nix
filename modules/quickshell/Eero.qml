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
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // The record as eero last answered, or null.
    property var model: null
    readonly property bool present: model !== null

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

    FileView {
        id: file
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/eero/home-network.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        // What was read last, so a reading that changed nothing says nothing.
        property string read: ""
        onLoaded: {
            if (file.text() === file.read)
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
