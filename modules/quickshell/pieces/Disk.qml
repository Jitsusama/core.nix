// The disks the guard keeps from filling (modules/disk-guard): how much room
// the worst of them has left and how the guard stands. It reads the status
// files the account's guard and the machine's write in their runtime
// directories, so it asks nothing, and a disk both watch is as the worse of
// them says. It's true while a disk is short of room, below healthy, and
// takes the warning colour then and wants a look, and the alert colour at
// critical, when it wants Joel now: the guard is taking all it safely can. A
// writer the guard froze for filling a disk wants him now too, with a way to
// let it go on. It's true as
// well when a guard has stopped writing, since then nothing watches the disks.
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
    readonly property bool critical: short && worst.band === "critical"
    // Each guard writes its status at least every 30 seconds while it runs.
    readonly property bool silent: account.silent || machine.silent
    readonly property var last: account.said?.last ?? null
    // What the account's guard froze for filling a disk, if anything.
    readonly property var frozen: account.said?.frozen ?? []

    present: said.length > 0
    active: short || silent || frozen.length > 0
    role: critical || frozen.length > 0 ? "alert" : active ? "warning" : "text"
    // Short of room, it wants a look; critical or frozen, it wants Joel now.
    wants: short || frozen.length > 0
    now: critical || frozen.length > 0
    value: frozen.length > 0 ? "froze " + frozen.map(writer => writer.name).join(", ")
        : short || !silent
        ? (worst === null ? "" : `${shown(worst.free)} free on ${worst.path}`
            + (short ? `, ${worst.band}` : ""))
        : "not watching"

    title: "Disks"
    hint: frozen.length > 0 ? `${frozen[0].name} was frozen for filling ${frozen[0].path}`
        : short ? `${worst.path} is short of room: ${shown(worst.free)} free`
        : silent ? "Nothing is watching the disks" : value
    context: {
        const parts = [];
        if (short && worst.forecast !== null && worst.forecast !== undefined)
            parts.push(`the next band in ${duration(worst.forecast)}`);
        if (last !== null && last.band !== "healthy") {
            parts.push(last.end === "reached" ? `freed ${shown(last.freed)} on ${last.path}`
                : last.end === "exhausted" ? "nothing more is safe to take"
                : last.end === "stalled" ? "deleting stopped freeing space"
                : `its last pass ${last.end}`);
        }
        if (silent && !short)
            parts.push("the guard's unit says why");
        return parts.join("; ");
    }
    actions: (frozen.length > 0 ? [{
        glyph: String.fromCodePoint(0xf040a),
        name: "Let it go on",
        verb: "resume",
        run: () => Niri.spawn(["disk-guard", "thaw"])
    }] : []).concat([{
        glyph: String.fromCodePoint(0xf0349),
        name: "See what it would take",
        verb: "see",
        run: () => Niri.spawn(["kitty", "--hold", "disk-guard", "plan"])
    }]).concat(silent ? [{
        glyph: String.fromCodePoint(0xf040a),
        name: "Start the guard",
        run: () => Niri.spawn(["systemctl", "--user", "restart", "disk-guard.service"])
    }] : [])

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

    function duration(seconds) {
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
}
