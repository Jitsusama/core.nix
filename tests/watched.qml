// Drives a status file (modules/quickshell/pieces/StatusFile.qml) through
// Quickshell itself while its folder comes and goes, as a guard's runtime
// folder does when systemd restarts it: missing when the shell starts, then
// made with a status in it, then removed, then made again. Each status is
// heard as it's written, and none while the folder is gone. It exits 0 once
// every case holds, and otherwise 1, saying which case didn't.
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string folder: Quickshell.env("XDG_RUNTIME_DIR") + "/guard"

    StatusFile {
        id: status
        path: `${root.folder}/status.json`
    }

    // Each case: what's done to the folder (null: nothing), and what the
    // status file makes of it. A status is written beside the file and moved
    // over it, as the guards write theirs.
    function written(n) {
        const said = JSON.stringify({ heartbeat: Math.floor(Date.now() / 1000), n: n });
        return `mkdir -p ${folder} && echo '${said}' > ${folder}/.status && mv ${folder}/.status ${folder}/status.json`;
    }

    readonly property var cases: [{
        what: "nothing said while the folder isn't there",
        does: null,
        holds: () => status.said === null
    }, {
        what: "the first status heard once its folder is made",
        does: () => root.written(1),
        holds: () => status.said?.n === 1
    }, {
        what: "nothing said once the folder is removed",
        does: () => `rm -r ${root.folder}`,
        holds: () => status.said === null
    }, {
        what: "the next status heard once the folder is made again",
        does: () => root.written(2),
        holds: () => status.said?.n === 2
    }]

    property int at: 0
    property int left: 0

    Process {
        id: doing
        onExited: {
            root.left = 100;
            waiting.start();
        }
    }

    function begin() {
        const step = cases[at];
        if (step.does === null) {
            left = 100;
            waiting.start();
        } else {
            doing.command = ["sh", "-c", step.does()];
            doing.running = true;
        }
    }

    // Quickshell hears the files in the background, so each case waits for
    // the status to hold, for five seconds at most.
    Timer {
        id: waiting
        interval: 50
        repeat: true
        onTriggered: {
            const step = root.cases[root.at];
            if (step.holds()) {
                stop();
                if (++root.at === root.cases.length) {
                    console.log("PASS every case holds");
                    Qt.exit(0);
                } else {
                    root.begin();
                }
            } else if (--root.left === 0) {
                stop();
                console.warn(`FAIL ${step.what}: said ${JSON.stringify(status.said)}`);
                Qt.exit(1);
            }
        }
    }

    Component.onCompleted: begin()
}
