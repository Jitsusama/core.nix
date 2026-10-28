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
// as well when a guard has stopped writing, since then nothing watches the
// disks. Nothing else asks for him, since nothing else needs him. It also
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
    // Each guard writes its status at least every 30 seconds while it runs.
    readonly property bool silent: account.silent || machine.silent
    readonly property var last: account.said?.last ?? null
    // What the account's guard froze for filling a disk, if anything.
    readonly property var frozen: account.said?.frozen ?? []
    // What holds the disks, as each guard measured it, biggest first; a
    // folder both measured is one holder, as the later says. A guard
    // measures only while a disk is short of room, so it's empty until one
    // has: { path, bytes, owner: { kind, name }, clearable, why, says }.
    readonly property var holders: {
        const byPath = {};
        for (const status of said.slice().sort((a, b) => (a.holders_at ?? 0)
            - (b.holders_at ?? 0)))
            for (const holder of status.holders ?? [])
                byPath[holder.path] = holder;
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
        : "not watching"

    title: "Disks"
    hint: frozen.length > 0 ? `${frozen[0].name} was frozen for filling ${frozen[0].path}`
        : short ? `${worst.path} is short of room: ${shown(worst.free)} free`
        : silent ? "Nothing is watching the disks" : value
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
        if (silent && !short)
            parts.push("the guard's unit says why");
        return parts.join("; ");
    }
    actions: (frozen.length > 0 ? [{
        glyph: Theme.glyph.resume,
        name: "Thaw It",
        verb: "thaw",
        stays: true,
        run: () => Niri.spawn(["disk-guard", "thaw"])
    }] : []).concat(silent ? [{
        glyph: Theme.glyph.resume,
        name: "Start the Guard",
        stays: true,
        run: () => Niri.spawn(["systemctl", "--user", "restart", "disk-guard.service"])
    }] : [])

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
}
