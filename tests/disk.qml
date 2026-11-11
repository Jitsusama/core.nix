// Drives the disk piece (modules/quickshell/pieces/Disk.qml) through
// Quickshell itself, over status files written where the guards write
// theirs on a machine of the check's own (MACHINE_ROOT), which the machine
// daemon tells the piece, with niri stood in for: healthy disks are
// shown but not true, a filling disk says how much is free and the pass
// that left it short but asks for nothing, a nearly full one is a warning
// that still asks for nothing, a critically full one wants Joel with
// nothing slow to run, the worse of the two guards' words wins for a disk
// both watch, a disk only the machine's guard watches counts, a disk full
// within a few days says when and how fast whatever its band, and is a
// warning within a day, with one forecast when it's the worst disk too, a
// writer frozen wants Joel with a way to let it go on, a guard gone quiet
// whose unit says it's starting again shows nothing, one whose heartbeat
// ages past silence with nothing written after it and whose unit failed is
// a warning of its own saying why, with a way to start it, and the
// machine's guard stopped says a restart starts it, with no way of its
// own, what holds the disks is one list from both guards, biggest first,
// and a guard that never wrote leaves no piece. It exits 0 once every case
// holds, and otherwise 1, saying which
// case didn't.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "pieces"

ShellRoot {
    id: root

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property string machineRoot: Quickshell.env("MACHINE_ROOT")
    readonly property real gib: 2 ** 30
    readonly property real now: Date.now() / 1000

    function disk(path, band, free, forecast) {
        return { path: path, band: band, free: free * gib, total: 950 * gib,
            forecast: forecast ?? null };
    }

    // A disk filling at `perHour` GiB an hour, full in `fullIn` seconds.
    function filling(said, fullIn, perHour) {
        return Object.assign(said, { full_at: root.now + fullIn,
            fill_per_hour: perHour * gib });
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
        account: held(status([disk("/", "filling", 120.5), disk("/boot", "healthy", 0.5)]), [
            holder("/home/joel/.cache/karst", 17, "cache", "karst"),
            holder("/home/joel/trees/lamp", 40, "worktree", "worktree-lamp")], root.now - 60),
        machine: held(status([disk("/", "healthy", 400)]), [
            holder("/var/lib/containers", 24, "containers", "podman"),
            holder("/home/joel/trees/lamp", 41, "worktree", "worktree-lamp")], root.now),
        holds: piece => piece.holders.map(one => one.path).join(" ")
                === "/home/joel/trees/lamp /var/lib/containers /home/joel/.cache/karst"
            && piece.holders[0].bytes === 41 * root.gib && piece.holdersAt === root.now
            && piece.holding("/home/joel/trees/lamp").path === "/"
            && piece.holding("/boot/efi").path === "/boot"
            && piece.holding("/bootstrap").path === "/"
    }, {
        what: "a disk filling slowly",
        // Nearly full in three days, or never: only the margin says so.
        account: status([disk("/", "filling", 120.5, 3 * 24 * 3600)]),
        machine: status([disk("/", "filling", 120.5)]),
        holds: piece => !piece.active && piece.role === "text" && !piece.wants
            && piece.value === "filling \u00b7 129\u00a0GB free"
            && piece.valueItems.length === 2
            && piece.context === "nearly full in 3 days"
    }, {
        what: "a disk nearly full within a day",
        account: status([disk("/", "filling", 120.5, 5400)],
            { at: root.now, path: "/", band: "filling", end: "exhausted", freed: 0 }),
        machine: status([disk("/", "healthy", 400)]),
        holds: piece => piece.active && piece.role === "warning" && !piece.wants && !piece.now
            && piece.value === "filling \u00b7 129\u00a0GB free"
            && piece.context === "nearly full in 1 h 30 min; nothing more is safe to take"
    }, {
        what: "a healthy disk full within a day",
        // cargo's builds on 7 November: healthy, 21 GiB free, full in 6.8
        // hours, and nothing said.
        account: status([disk("/", "healthy", 286),
            filling(disk("/var/cache/cargo-builds", "healthy", 21), 24500, 3)]),
        machine: status([disk("/", "healthy", 286)]),
        holds: piece => piece.active && piece.role === "warning" && !piece.wants
            && piece.value === "307\u00a0GB free"
            && piece.context
                === "/var/cache/cargo-builds full in 6 h 48 min at 3.22\u00a0GB an hour"
    }, {
        what: "a disk full in a few days",
        account: status([filling(disk("/", "healthy", 286), 2 * 24 * 3600 + 600, 4.4)]),
        machine: status([disk("/", "healthy", 286)]),
        holds: piece => !piece.active && piece.role === "text"
            && piece.context === "/ full in 2 days at 4.72\u00a0GB an hour"
    }, {
        what: "the worst disk filling",
        // Its band's forecast and its full one would be two forecasts of
        // one disk, so it says the nearer, with how fast it fills.
        account: status([filling(disk("/", "filling", 120.5, 5400), 20000, 4)]),
        machine: status([disk("/", "healthy", 400)]),
        holds: piece => piece.active && piece.role === "warning"
            && piece.context === "nearly full in 1 h 30 min at 4.29\u00a0GB an hour"
    }, {
        what: "a disk full past a few days",
        account: status([filling(disk("/", "healthy", 286), 5 * 24 * 3600, 2)]),
        machine: status([disk("/", "healthy", 286)]),
        holds: piece => !piece.active && piece.context === ""
    }, {
        what: "a critically full disk",
        account: status([disk("/", "critically-full", 8.25)],
            { at: root.now, path: "/", band: "critically-full", end: "reached", freed: 3 * gib }),
        machine: status([]),
        holds: piece => piece.active && piece.role === "alert" && piece.wants && piece.now
            && piece.value === "critically full \u00b7 8.86\u00a0GB free"
            && piece.hint === "/ is short of room: 8.86\u00a0GB free"
            && piece.context === "freed 3.22\u00a0GB on /"
            && piece.actions.length === 0
    }, {
        what: "the worse of two words",
        account: status([disk("/", "healthy", 200)]),
        machine: status([disk("/", "nearly-full", 60)]),
        holds: piece => piece.active && piece.role === "warning" && !piece.wants
            && piece.value === "nearly full \u00b7 64.4\u00a0GB free"
    }, {
        what: "a disk only the machine watches",
        account: status([disk("/", "healthy", 400)]),
        machine: status([disk("/", "healthy", 400), disk("/boot", "critically-full", 0.01)]),
        holds: piece => piece.wants && piece.value === "critically full \u00b7 10.7\u00a0MB free"
    }, {
        what: "a writer frozen",
        // Thawed only once no longer nearly full, so frozen still while it is.
        account: status([disk("/", "nearly-full", 50)], null, null, [{
            unit: "disk-guard-freeze-700.scope", pid: 700, start: 5, name: "dd",
            path: "/", at: root.now }]),
        machine: status([]),
        holds: piece => piece.active && piece.role === "alert" && piece.wants
            && piece.value === "froze dd"
            && piece.hint === "dd was frozen for filling /"
            && piece.actions[0].name === "Thaw It" && piece.actions[0].verb === "thaw"
    }, {
        // Quiet as each switch leaves it while its unit restarts it.
        what: "a guard being restarted",
        accountUnit: "ActiveState=activating\nResult=success\n",
        account: status([disk("/", "healthy", 400)], null, root.now - 600),
        machine: status([disk("/", "healthy", 400)]),
        holds: piece => piece.units.account.asked
            && piece.units.account.state === "activating"
            && !piece.active && piece.value === "429\u00a0GB free"
            && piece.valueItems.length === 1
    }, {
        // After one that isn't silent, so only this one's heartbeat can
        // make it so: a second short of silence, and nothing after it, so
        // only the deadline it set can say so.
        what: "a guard that falls silent and failed",
        accountUnit: "ActiveState=failed\nResult=exit-code\n",
        account: () => status([disk("/", "healthy", 400)], null, Date.now() / 1000 - 119),
        machine: status([disk("/", "healthy", 400)]),
        holds: piece => piece.active && piece.role === "warning" && !piece.wants
            && piece.value === "guard stopped"
            && piece.hint === "The disk guard stopped, so nothing keeps the disks from filling"
            && piece.context === "it failed"
            && piece.actions.some(action => action.name === "Start the Guard")
    }, {
        what: "the machine's guard turned off",
        machineUnit: "ActiveState=inactive\nResult=success\n",
        account: status([disk("/", "healthy", 400)]),
        machine: status([disk("/", "healthy", 400)], null, root.now - 600),
        holds: piece => piece.active && piece.value === "guard stopped"
            && piece.hint.endsWith("restarting the machine starts it again")
            && piece.context === "it was turned off"
            && piece.actions.length === 0
    }]

    property int at: 0
    property int left: 0

    Disk {
        id: piece
        settings: ({
            accountUnit: ["cat", `${root.runtime}/account-unit`],
            machineUnit: ["cat", `${root.runtime}/machine-unit`]
        })
    }

    // What each guard's unit says when asked, written before its status.
    FileView {
        id: accountUnit
        path: `${root.runtime}/account-unit`
        blockWrites: true
        printErrors: false
    }

    FileView {
        id: machineUnit
        path: `${root.runtime}/machine-unit`
        blockWrites: true
        printErrors: false
    }

    // Says what each guard said, or leaves a file nothing parses where it
    // was, written beside and renamed into place as the guards write
    // theirs: a FileView's write can make a file new by linking it in,
    // which says only that it was made, and the daemon, as it should,
    // waits for one written whole. The case is waited on once both are.
    Process {
        id: writer

        onExited: {
            root.left = 100;
            waiting.start();
        }
    }

    function write(account, machine) {
        writer.command = ["sh", "-c", 'printf %s "$1" > "$2.writing" && mv "$2.writing" "$2"'
            + ' && printf %s "$3" > "$4.writing" && mv "$4.writing" "$4"', "write",
            account ?? "", `${root.machineRoot}/run/user/1000/disk-guard/status.json`,
            machine ?? "", `${root.machineRoot}/run/disk-guard/status.json`];
        writer.running = true;
    }

    function begin() {
        const step = cases[at];
        // A unit is running unless the case says otherwise.
        const running = "ActiveState=active\nResult=success\n";
        accountUnit.setText(step.accountUnit ?? running);
        machineUnit.setText(step.machineUnit ?? running);
        // A guard's words may be reckoned as its case begins.
        write(typeof step.account === "function" ? step.account() : step.account, step.machine);
    }

    // The daemon tells the piece what was written over its socket, so each
    // case waits for the piece to hold, for five seconds at most.
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
