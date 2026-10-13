// Drives the disk piece (modules/quickshell/pieces/Disk.qml) through
// Quickshell itself, over status files written where the test tells the
// piece the guards write theirs, with niri stood in for: healthy disks are
// shown but not true, a disk under pressure is a warning with how much is
// free and the pass that left it short, a critical one wants Joel, the worse
// of the two guards' words wins for a disk both watch, a disk only the
// machine's guard watches counts, a guard that stopped writing is a warning
// of its own with a way to start it, and a guard that never wrote leaves no
// piece. It exits 0 once every case holds, and otherwise 1, saying which
// case didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property real gib: 2 ** 30
    readonly property real now: Date.now() / 1000

    function disk(path, band, free, forecast) {
        return { path: path, band: band, free: free * gib, total: 950 * gib,
            forecast: forecast ?? null };
    }

    function status(disks, last, heartbeat) {
        return JSON.stringify({ version: 1, heartbeat: heartbeat ?? root.now,
            filesystems: disks, last: last ?? null });
    }

    // Each case: what each guard says (null: nothing written), and what
    // the piece makes of it.
    readonly property var cases: [{
        what: "a guard that never wrote",
        account: null,
        machine: null,
        holds: piece => !piece.present && !piece.active
    }, {
        what: "healthy disks",
        account: status([disk("/", "healthy", 400)]),
        machine: status([disk("/", "healthy", 400), disk("/boot", "healthy", 0.5)]),
        holds: piece => piece.present && !piece.active && piece.role === "text"
    }, {
        what: "a disk under pressure",
        account: status([disk("/", "pressure", 120.5, 5400)],
            { at: root.now, path: "/", band: "pressure", end: "exhausted", freed: 0 }),
        machine: status([disk("/", "healthy", 400)]),
        holds: piece => piece.active && piece.role === "warning" && !piece.wants
            && piece.value === "120.5 GiB free on /, pressure"
            && piece.badge === "120G"
            && piece.context === "the next band in 1:30; nothing more is safe to take"
    }, {
        what: "a critical disk",
        account: status([disk("/", "critical", 8.25)],
            { at: root.now, path: "/", band: "critical", end: "reached", freed: 3 * gib }),
        machine: status([]),
        holds: piece => piece.active && piece.role === "alert" && piece.wants
            && piece.value === "8.2 GiB free on /, critical" && piece.badge === "8.2G"
            && piece.hint === "/ is short of room: 8.2 GiB free"
            && piece.context === "freed 3.0 GiB on /"
    }, {
        what: "the worse of two words",
        account: status([disk("/", "healthy", 200)]),
        machine: status([disk("/", "urgent", 60)]),
        holds: piece => piece.active && piece.value === "60.0 GiB free on /, urgent"
    }, {
        what: "a disk only the machine watches",
        account: status([disk("/", "healthy", 400)]),
        machine: status([disk("/", "healthy", 400), disk("/boot", "critical", 0.01)]),
        holds: piece => piece.wants && piece.value === "10.2 MiB free on /boot, critical"
    }, {
        what: "a guard that stopped",
        account: status([disk("/", "healthy", 400)], null, root.now - 600),
        machine: status([disk("/", "healthy", 400)]),
        holds: piece => piece.active && piece.role === "warning" && !piece.wants
            && piece.value === "not watching"
            && piece.actions.some(action => action.name === "Start the guard")
    }]

    property int at: 0
    property int left: 0

    Disk {
        id: piece
        settings: ({
            account: `${root.runtime}/account.json`,
            machine: `${root.runtime}/machine.json`,
            every: 50
        })
    }

    FileView {
        id: account
        path: `${root.runtime}/account.json`
        printErrors: false
    }

    FileView {
        id: machine
        path: `${root.runtime}/machine.json`
        printErrors: false
    }

    // Says what a guard said, or leaves a file nothing parses where it was.
    function write(file, said) {
        file.setText(said ?? "");
    }

    function begin() {
        const step = cases[at];
        write(account, step.account);
        write(machine, step.machine);
        left = 100;
        waiting.start();
    }

    // Quickshell reads the files in the background, so each case waits for
    // the piece to hold, for five seconds at most.
    Timer {
        id: waiting
        interval: 50
        repeat: true
        onTriggered: {
            const step = root.cases[root.at];
            if (step.holds(piece)) {
                stop();
                if (++root.at === root.cases.length) {
                    console.log("PASS every case holds");
                    Qt.exit(0);
                } else {
                    root.begin();
                }
            } else if (--root.left === 0) {
                stop();
                console.warn(`FAIL ${step.what}: present ${piece.present}, active ${piece.active},`
                    + ` role ${piece.role}, wants ${piece.wants}, value "${piece.value}",`
                    + ` badge "${piece.badge}", hint "${piece.hint}", context "${piece.context}"`);
                Qt.exit(1);
            }
        }
    }

    Component.onCompleted: begin()
}
