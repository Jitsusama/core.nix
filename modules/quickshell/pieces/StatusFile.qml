// A status file a watcher writes whole, with its heartbeat in it at least
// every so often while it runs: what it says, read as `parse` reads it, or
// null when there's none that reads, and whether the watcher has gone
// silent, no heartbeat for `silence` seconds.
//
// Nothing ticks. The file's watch hears each new one, since it watches the
// folder too, and each heartbeat sets a deadline that long after it. The
// deadline counts the machine's running time, which a suspend stops as it
// stops the watcher, so waking gives the watcher as long again.
import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: status

    required property string path
    property real silence: 120
    property var parse: text => {
        try {
            const said = JSON.parse(text);
            return typeof said === "object" && said !== null ? said : null;
        } catch (error) {
            return null;
        }
    }
    property var said: null
    property bool silent: false

    function heard() {
        const age = Date.now() / 1000 - (said?.heartbeat ?? 0);
        quiet.stop();
        silent = said !== null && age > silence;
        if (said !== null && !silent) {
            quiet.interval = Math.max(1, (silence - age) * 1000);
            quiet.start();
        }
    }

    Timer {
        id: quiet
        onTriggered: status.silent = true
    }

    FileView {
        id: file
        path: status.path
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: {
            status.said = status.parse(file.text());
            status.heard();
        }
        onLoadFailed: {
            status.said = null;
            status.heard();
        }
    }
}
