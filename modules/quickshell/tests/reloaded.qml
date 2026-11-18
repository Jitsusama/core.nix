// Drives a FileView reloaded while it's still reading, through Quickshell
// itself (modules/quickshell/fileview-reads-again.patch). A case reloads
// it, writes the file at once and reloads it again, all in one turn of the
// event loop. A read lands only once the turn is over, so the second
// reload always finds the first's read under way, whenever that read
// opened the file. What the view holds once its reads land is what was
// written last, read by one read more and no others, as a change heard
// mid-read must be. It exits 0 once every case holds, and otherwise 1,
// saying which case didn't.
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string path: Quickshell.env("XDG_RUNTIME_DIR") + "/status.json"

    // How many reads the view has landed, and how many when a case began.
    property int reads: 0
    property int readsBefore: 0

    // Watches nothing, so only a case's own reloads read it.
    FileView {
        id: view
        path: root.path
        printErrors: false
        onLoaded: root.reads += 1
        onLoadFailed: root.reads += 1
    }

    // Writes as it's told to, before the turn is over.
    FileView {
        id: writer
        path: root.path
        blockWrites: true
        printErrors: false
    }

    function written(text) {
        writer.setText(text);
    }

    readonly property var cases: [{
        what: "a file read with nothing under way is read once",
        does: () => {
            root.written("one");
            view.reload();
        },
        holds: () => view.text() === "one" && root.reads === root.readsBefore + 1
    }, {
        what: "a reload mid-read is read after the write it follows",
        does: () => {
            view.reload();
            root.written("two");
            view.reload();
        },
        holds: () => view.text() === "two" && root.reads === root.readsBefore + 2
    }, {
        what: "however many reloads come mid-read, they ask one read",
        does: () => {
            view.reload();
            root.written("three");
            view.reload();
            view.reload();
            view.reload();
        },
        holds: () => view.text() === "three" && root.reads === root.readsBefore + 2
    }, {
        what: "and a reload with nothing under way still reads once",
        does: () => {
            root.written("four");
            view.reload();
        },
        holds: () => view.text() === "four" && root.reads === root.readsBefore + 1
    }]

    property int at: 0
    property int left: 0

    function begin() {
        readsBefore = reads;
        cases[at].does();
        left = 100;
        waiting.start();
    }

    // Quickshell reads the file in the background, so each case waits for
    // the view to hold, for five seconds at most.
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
                console.warn(`FAIL ${step.what}: holds "${view.text()}", `
                    + `read ${root.reads - root.readsBefore} times since`);
                Qt.exit(1);
            }
        }
    }

    // The view's own first read lands before the first case counts.
    Timer {
        id: settling
        interval: 50
        repeat: true
        onTriggered: {
            if (root.reads > 0) {
                stop();
                root.begin();
            }
        }
    }

    Component.onCompleted: settling.start()
}
