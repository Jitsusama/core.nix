// The disks the guard keeps from filling (modules/disk-guard): how much room
// the worst of them has left and how the guard stands. It reads the status
// files the account's guard and the machine's write in their runtime
// directories, so it asks nothing, and a disk both watch is as the worse of
// them says. While a disk is filling the guard is only doing its routine
// work, so the piece just says how much is free, unless the disk will be
// nearly full within a day, when it's true and Now shows it with when
// (Joel, 2026-10-26: Now shows a filling disk with its forecast). From
// nearly full it's true and takes the warning colour, and at critically
// full the alert colour, when it wants Joel now:
// the guard is taking all it safely can. A writer the guard froze for
// filling a disk wants him now too, with Thaw It to let it carry on. It's true
// as well when a guard has stopped, since then nothing watches the disks: its
// status going quiet is only the cue to ask its unit, once, and only a unit
// that says it isn't running shows, since every switch restarts the guards
// (Joel, 2026-10-28: "why do i sometimes see disks not watching, wtf does
// that mean?"). Nothing else asks for him, since nothing else needs him. It also
// holds what the guards measured filling the disks, for the launcher's
// Disk to list.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

Piece {
    id: piece

    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")
    readonly property var bands: ["healthy", "pressure", "urgent", "critical"]
    readonly property var said: [account.said, machine.said].filter(status => status !== null)
    readonly property var disks: {
        const byPath = {};
        for (const status of said)
            for (const disk of status.filesystems ?? [])
                if (!(disk.path in byPath) || worse(disk, byPath[disk.path]))
                    byPath[disk.path] = disk;
        return Object.values(byPath);
    }
    readonly property var worst: disks.reduce(
        (worst, disk) => worst === null || worse(disk, worst) ? disk : worst, null)
    readonly property bool short: worst !== null && worst.band !== "healthy"
    readonly property bool pressed: short && worst.band !== "pressure"
    // A filling disk that will be nearly full within a day.
    readonly property real day: 24 * 60 * 60
    readonly property bool soon: short && worst.band === "pressure"
        && typeof worst.forecast === "number" && worst.forecast < day
    readonly property bool critical: short && worst.band === "critical"
    // Each guard writes its status at least every 30 seconds while it runs,
    // so a quiet one is asked about, and counts only once its unit says it
    // stopped.
    readonly property bool accountStopped: account.silent && accountUnit.stopped
    readonly property bool machineStopped: machine.silent && machineUnit.stopped
    readonly property bool silent: accountStopped || machineStopped
    // What each guard's unit said when last asked, for its tests.
    readonly property var units: ({ account: accountUnit, machine: machineUnit })
    readonly property var last: account.said?.last ?? null
    // What the account's guard froze for filling a disk, if anything.
    readonly property var frozen: account.said?.frozen ?? []
    // What holds the disks, as each guard measured it, biggest first; a
    // folder both measured is one holder, as the later says. A guard
    // measures only while a disk is short of room, so it's empty until one
    // has: { path, name, bytes, clears, owner: { kind, name }, clearable,
    // why, says, kept }, and `ours` where the account's guard measured it,
    // since only that guard keeps what Joel asks it to.
    readonly property var holders: {
        const byPath = {};
        for (const status of said.slice().sort((a, b) => (a.holders_at ?? 0)
            - (b.holders_at ?? 0)))
            for (const holder of status.holders ?? [])
                byPath[holder.path] = Object.assign({}, holder,
                    { ours: status === account.said });
        return Object.values(byPath).sort((a, b) => b.bytes - a.bytes);
    }
    // When they were last measured, in wall seconds, or 0.
    readonly property real holdersAt: said.reduce((latest, status) => Math.max(latest,
        status.holders_at ?? 0), 0)

    present: said.length > 0
    active: pressed || soon || silent || frozen.length > 0
    role: critical || frozen.length > 0 ? "alert" : active ? "warning" : "text"
    // Only critical or frozen wants Joel, and then at once.
    wants: critical || frozen.length > 0
    now: wants
    value: frozen.length > 0 ? "froze " + frozen.map(writer => writer.name).join(", ")
        : short || !silent
        ? (worst === null ? "" : `${shown(worst.free)} free on ${worst.path}`
            + (short ? `, ${banded(worst.band)}` : ""))
        : "guard stopped"

    title: "Disks"
    hint: frozen.length > 0 ? `${frozen[0].name} was frozen for filling ${frozen[0].path}`
        : short ? `${worst.path} is short of room: ${shown(worst.free)} free`
        : accountStopped ? "The disk guard stopped, so nothing keeps the disks from filling"
        : machineStopped ? "The machine's disk guard stopped, so nothing keeps the disks"
            + " from filling; restarting the machine starts it again"
        : value
    context: {
        const parts = [];
        const next = short ? bands[bands.indexOf(worst.band) + 1] : undefined;
        if (next && typeof worst.forecast === "number")
            parts.push(`${banded(next)} in ${duration(worst.forecast)}`);
        if (last !== null && last.band !== "healthy") {
            parts.push(last.end === "reached" ? `freed ${shown(last.freed)} on ${last.path}`
                : last.end === "exhausted" ? "nothing more is safe to take"
                : last.end === "stalled" ? "deleting stopped freeing space"
                : `its last pass ${last.end}`);
        }
        if (silent)
            parts.push(stoppedFor((accountStopped ? accountUnit : machineUnit).result));
        return parts.join("; ");
    }
    actions: (frozen.length > 0 ? [{
        glyph: Theme.glyph.resume,
        name: "Thaw It",
        verb: "thaw",
        stays: true,
        run: () => Niri.spawn(["disk-guard", "thaw"])
    }] : []).concat(accountStopped ? [{
        glyph: Theme.glyph.resume,
        name: "Start the Guard",
        stays: true,
        run: () => Niri.spawn(["systemctl", "--user", "restart", "disk-guard.service"])
    }] : [])

    // What the guard said of the last Keep or Release, for the foot: the
    // one sentence it prints, or why it couldn't.
    property string told: ""

    // Keeps a folder from the guard's clearing, for Joel's reason, or lets
    // it go again; the account's guard rewrites its status before it
    // answers, so the row has flipped by the time the foot says so.
    function keep(path, reason) {
        tell(["keep", path, reason]);
    }

    function release(path) {
        tell(["unkeep", path]);
    }

    // Asks the account's guard to clear what it would take of a folder
    // now, which it starts on its next step, marking the holder clearing.
    function clear(path) {
        tell(["clear", path]);
    }

    function tell(words) {
        if (keeping.running)
            return;
        told = "\u2026";
        keeping.command = ["sh", "-c", "exec \"$0\" \"$@\" 2>&1", "disk-guard"].concat(words);
        keeping.running = true;
    }

    Process {
        id: keeping
        stdout: StdioCollector {
            id: keepingSaid
            onStreamFinished: piece.told = keepingSaid.text.trim()
        }
    }

    // Why a guard stopped, in a phrase, from what its unit's Result says.
    function stoppedFor(result) {
        return result === "success" ? "it was turned off"
            : result === "oom-kill" ? "it ran out of memory"
            : result === "timeout" ? "it took too long to start or stop"
            : result === "start-limit-hit" ? "it failed too often to start again"
            : "it failed";
    }

    // A band in Joel's words, the glossary's: filling, nearly full and
    // critically full, never the guard's own names for them.
    function banded(band) {
        return (Words.of("machine:disk").said ?? {})[band] ?? band;
    }

    // The watched disk a path is on: the one whose path holds it longest.
    function holding(path) {
        return disks.filter(disk => path === disk.path || path.startsWith(disk.path === "/" ? "/"
            : `${disk.path}/`)).reduce((best, disk) => best === null
            || disk.path.length > best.path.length ? disk : best, null);
    }

    function worse(one, other) {
        return bands.indexOf(one.band) > bands.indexOf(other.band);
    }

    // Sizes as the guard says them, but never more than there is.
    function shown(bytes) {
        const units = [[40, "TiB"], [30, "GiB"], [20, "MiB"], [10, "KiB"]];
        const [shift, unit] = units.find(([shift]) => bytes >= 2 ** shift) ?? [0, "B"];
        return shift === 0 ? `${bytes} B`
            : `${(Math.floor(bytes / 2 ** shift * 10) / 10).toFixed(1)} ${unit}`;
    }

    // How long, as the battery's "full in" says it, and past a day in days.
    function duration(seconds) {
        if (seconds >= day) {
            const days = Math.round(seconds / day);
            return days === 1 ? "a day" : `${days} days`;
        }
        const minutes = Math.round(seconds / 60);
        return Math.floor(minutes / 60) + ":" + String(minutes % 60).padStart(2, "0");
    }

    // Each guard's status, the account's and the machine's.
    StatusFile {
        id: account
        path: piece.settings.account ?? `${piece.runtime}/disk-guard/status.json`
        silence: piece.settings.silence ?? 120
    }

    StatusFile {
        id: machine
        path: piece.settings.machine ?? "/run/disk-guard/status.json"
        silence: piece.settings.silence ?? 120
    }

    // Each guard's unit, asked once each time its status goes quiet.
    Unit {
        id: accountUnit
        quiet: account.silent
        command: piece.settings.accountUnit ?? ["systemctl", "--user", "show", "-p",
            "ActiveState,Result", "disk-guard.service"]
    }

    Unit {
        id: machineUnit
        quiet: machine.silent
        command: piece.settings.machineUnit ?? ["systemctl", "show", "-p",
            "ActiveState,Result", "disk-guard.service"]
    }

    // What a guard's unit says of it, asked as its status goes quiet, or as
    // the piece starts on one already quiet, and never again until it's
    // heard from: stopped once the unit says it isn't running or starting,
    // or can't be asked, with the Result that says why.
    component Unit: Process {
        id: unit

        property bool quiet: false
        property bool asked: false
        property string state: ""
        property string result: ""
        readonly property bool stopped: asked
            && !["active", "activating", "deactivating", "reloading", "refreshing"]
                .includes(state)

        function ask() {
            asked = false;
            state = "";
            result = "";
            running = true;
        }

        onQuietChanged: quiet ? ask() : asked = false
        Component.onCompleted: if (quiet) ask()
        onExited: code => {
            if (code !== 0)
                asked = true;
        }
        stdout: StdioCollector {
            onStreamFinished: {
                for (const line of text.split("\n")) {
                    const [key, value] = line.split("=");
                    if (key === "ActiveState")
                        unit.state = value;
                    else if (key === "Result")
                        unit.result = value;
                }
                unit.asked = true;
            }
        }
    }
}
