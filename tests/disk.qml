// Drives the disk piece (modules/quickshell/pieces/Disk.qml) through
// Quickshell itself, over status files written where the test tells the
// piece the guards write theirs, with niri stood in for: healthy disks are
// shown but not true, a disk under pressure says how much is free and the
// pass that left it short but asks for nothing, an urgent one is a warning
// that still asks for nothing, a critical one wants Joel with nothing slow
// to run, the worse of the two guards' words wins for a disk both watch, a
// disk only the
// machine's guard watches counts, a writer frozen wants Joel with a way to let
// it go on, a guard that stopped writing is a warning
// of its own with a way to start it, as is one whose heartbeat ages past
// silence with nothing written after it, what holds the disks is one list
// from both guards, biggest first, and a guard that never wrote leaves no
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

    function status(disks, last, heartbeat, frozen) {
        return JSON.stringify({ version: 1, heartbeat: heartbeat ?? root.now,
            filesystems: disks, last: last ?? null, frozen: frozen ?? [] });
    }

    // A status with what holds the disks, measured at `at`.
    function held(said, holders, at) {
        return JSON.stringify(Object.assign(JSON.parse(said), { holders: holders,
            holders_at: at }));
    }

    function holder(path, size, kind, name) {
        return { path: path, bytes: size * gib, owner: { kind: kind, name: name },
            clearable: kind === "worktree", why: kind === "worktree" ? null : "not now",
            says: `${name} holds it.` };
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
            && piece.holders.length === 0 && piece.holdersAt === 0
    }, {
        what: "what holds the disks, from both guards",
        // A tree both measured is one holder, as the later measure says,
        // and every holder is listed biggest first.
        account: held(status([disk("/", "pressure", 120.5), disk("/boot", "healthy", 0.5)]), [
            holder("/home/joel/.cache/karst", 17, "cache", "karst"),
            holder("/home/joel/trees/lamp", 40, "worktree", "worktree-lamp")], root.now - 60),
        machine: held(status([disk("/", "healthy", 400)]), [
            holder("/var/lib/containers", 24, "containers", "podman"),
            holder("/home/joel/trees/lamp", 41, "worktree", "worktree-lamp")], root.now),
        holds: piece => piece.holders.map(one => one.path).join(" ")
                === "/home/joel/trees/lamp /var/lib/containers /home/joel/.cache/karst"
            && piece.holders[0].bytes === 41 * root.gib && piece.holdersAt === root.now
            && piece.holding("/home/joel/trees/lamp").path === "/"
            && piece.holding("/boot/efi").path === "/boot" && piece.holding("/bootstrap").path === "/"
    }, {
        what: "a disk filling slowly",
        // Nearly full in three days, or never: only the margin says so.
        account: status([disk("/", "pressure", 120.5, 3 * 24 * 3600)]),
        machine: status([disk("/", "pressure", 120.5)]),
        holds: piece => !piece.active && piece.role === "text" && !piece.wants
            && piece.value === "120.5 GiB free on /, filling"
            && piece.context === "nearly full in 3 days"
    }, {
        what: "a disk nearly full within a day",
        account: status([disk("/", "pressure", 120.5, 5400)],
            { at: root.now, path: "/", band: "pressure", end: "exhausted", freed: 0 }),
        machine: status([disk("/", "healthy", 400)]),
        holds: piece => piece.active && piece.role === "warning" && !piece.wants && !piece.now
            && piece.value === "120.5 GiB free on /, filling"
            && piece.context === "nearly full in 1:30; nothing more is safe to take"
    }, {
        what: "a critical disk",
        account: status([disk("/", "critical", 8.25)],
            { at: root.now, path: "/", band: "critical", end: "reached", freed: 3 * gib }),
        machine: status([]),
        holds: piece => piece.active && piece.role === "alert" && piece.wants && piece.now
            && piece.value === "8.2 GiB free on /, critically full"
            && piece.hint === "/ is short of room: 8.2 GiB free"
            && piece.context === "freed 3.0 GiB on /"
            && piece.actions.length === 0
    }, {
        what: "the worse of two words",
        account: status([disk("/", "healthy", 200)]),
        machine: status([disk("/", "urgent", 60)]),
        holds: piece => piece.active && piece.role === "warning" && !piece.wants
            && piece.value === "60.0 GiB free on /, nearly full"
    }, {
        what: "a disk only the machine watches",
        account: status([disk("/", "healthy", 400)]),
        machine: status([disk("/", "healthy", 400), disk("/boot", "critical", 0.01)]),
        holds: piece => piece.wants && piece.value === "10.2 MiB free on /boot, critically full"
    }, {
        what: "a writer frozen",
        // Thawed only above urgent, so frozen still at urgent.
        account: status([disk("/", "urgent", 50)], null, null, [{
            unit: "disk-guard-freeze-700.scope", pid: 700, start: 5, name: "dd",
            path: "/", at: root.now }]),
        machine: status([]),
        holds: piece => piece.active && piece.role === "alert" && piece.wants
            && piece.value === "froze dd"
            && piece.hint === "dd was frozen for filling /"
            && piece.actions[0].name === "Let it go on"
    }, {
        // After one that isn't silent, so only this one's heartbeat can
        // make it so: a second short of silence, and nothing after it, so
        // only the deadline it set can say so.
        what: "a guard that falls silent",
        account: () => status([disk("/", "healthy", 400)], null, Date.now() / 1000 - 119),
        machine: status([disk("/", "healthy", 400)]),
        holds: piece => piece.active && piece.value === "not watching"
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
            machine: `${root.runtime}/machine.json`
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
        // A guard's words may be reckoned as its case begins.
        write(account, typeof step.account === "function" ? step.account() : step.account);
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
                    + ` now ${piece.now}, hint "${piece.hint}", context "${piece.context}"`);
                Qt.exit(1);
            }
        }
    }

    Component.onCompleted: begin()
}
