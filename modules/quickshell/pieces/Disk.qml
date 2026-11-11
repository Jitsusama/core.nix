// The disks the guard keeps from filling (modules/disk-guard): how much room
// the worst of them has left and how the guard stands. It hears what the
// account's guard and the machine's write in their status files from the
// machine daemon (Watchers.qml), so it reads no file and asks nothing, and a
// disk both watch is as the worse of them says. While a disk is filling the
// guard is only doing its routine
// work, so the piece just says how much is free, unless the disk will be
// nearly full within a day, when it's true and Now shows it with when
// (Joel, 2026-10-26: Now shows a filling disk with its forecast). Any disk
// that will be full within a few days says so, how fast it fills and when,
// whatever its band, and within a day it's true: on 7 November cargo's
// builds were healthy with 21 GiB free, filling 3 GiB an hour and full in
// 6.8 hours, and nothing said so. From
// nearly full it's true and takes the warning colour, and at critically
// full the alert colour, when it wants Joel now:
// the guard is taking all it safely can. A writer the guard froze for
// filling a disk wants him now too, with Thaw It to let it carry on. It's true
// as well when a guard has stopped, since then nothing watches the disks: its
// status going quiet is only the cue for the machine daemon to ask its unit,
// once, and only a unit that says it isn't running shows, since every switch
// restarts the guards
// (Joel, 2026-10-28: "why do i sometimes see disks not watching, wtf does
// that mean?"). Nothing else asks for him, since nothing else needs him. It also
// holds what the guards measured filling the disks, for the launcher's
// Disk to list. Keep, Release and Clear are acts the machine daemon carries
// out with the account's guard, so the piece spawns nothing.
import qs
import QtQuick
import "../FileFacts.js" as FileFacts
import "Drawers.js" as Drawers
import "../launcher/ReadyRows.js" as Ready

Piece {
    id: piece

    readonly property var bands: ["healthy", "filling", "nearly-full", "critically-full"]
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
    readonly property bool pressed: short && worst.band !== "filling"
    // A filling disk that will be nearly full within a day.
    readonly property real day: 24 * 60 * 60
    readonly property bool soon: short && worst.band === "filling"
        && typeof worst.forecast === "number" && worst.forecast < day
    readonly property bool criticallyFull: short && worst.band === "critically-full"
    // The disk soonest full, of those either guard says will be within
    // `few` seconds at the fill's trend, or null: { path, full_at,
    // fill_per_hour }. Each guard's own figure counts, since each watches
    // over its own two hours.
    readonly property real few: 3 * day
    readonly property var filling: {
        const now = Date.now() / 1000;
        return said.reduce((all, status) => all.concat(status.filesystems ?? []), [])
            .filter(disk => typeof disk.full_at === "number" && disk.fill_per_hour > 0
                && disk.full_at - now <= few)
            .reduce((soonest, disk) => soonest === null || disk.full_at < soonest.full_at
                ? disk : soonest, null);
    }
    readonly property bool fullSoon: filling !== null
        && filling.full_at - Date.now() / 1000 < day
    // Each guard writes its status at least every 30 seconds while it runs,
    // so a quiet one's unit is asked about, and it counts only once its unit
    // says it stopped.
    readonly property bool accountStopped: account.silent && account.unit?.stopped === true
    readonly property bool machineStopped: machine.silent && machine.unit?.stopped === true
    readonly property bool silent: accountStopped || machineStopped
    // What each guard's unit said as it fell silent, for its tests.
    readonly property var units: ({ account: account.unit, machine: machine.unit })
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
    active: pressed || soon || fullSoon || silent || frozen.length > 0
    role: criticallyFull || frozen.length > 0 ? "alert" : active ? "warning" : "text"
    // Only critically full or frozen wants Joel, and then at once.
    wants: criticallyFull || frozen.length > 0
    now: wants
    value: frozen.length > 0 ? "froze " + frozen.map(writer => writer.name).join(", ")
        : short || !silent ? valueItems.map(item => item.said).join(" \u00b7 ")
        : "guard stopped"
    // How the worst disk stands, then its room, "nearly full \u00b7 8.59 GB
    // free": a row gives way by whole items, its room first, never cut, and
    // which disk is its drawer's to say (VKCF3Z, 2026-11-08).
    valueItems: worst === null || frozen.length > 0 || !(short || !silent) ? []
        : (short ? [{ said: banded(worst.band), keep: 2 }] : []).concat([{
            said: Drawers.filled((Words.of("on:disk").said ?? {}).free,
                { amount: shown(worst.free) }),
            keep: 1
        }])

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
        const banding = next && typeof worst.forecast === "number";
        // The worst disk filling says one forecast, its band's, which is the
        // nearer, with how fast it fills.
        const same = banding && filling !== null && filling.path === worst.path;
        if (banding)
            parts.push(`${banded(next)} in ${duration(worst.forecast)}`
                + (same ? ` at ${shown(filling.fill_per_hour)} an hour` : ""));
        if (filling !== null && !same)
            parts.push(`${filling.path} full in ${duration(Math.max(0, filling.full_at
                - Date.now() / 1000))} at ${shown(filling.fill_per_hour)} an hour`);
        if (last !== null && last.band !== "healthy") {
            parts.push(last.end === "reached" ? `freed ${shown(last.freed)} on ${last.path}`
                : last.end === "exhausted" ? "nothing more is safe to take"
                : last.end === "stalled" ? "deleting stopped freeing space"
                : `its last pass ${last.end}`);
        }
        if (silent)
            parts.push(stoppedFor((accountStopped ? account : machine).unit.result));
        return parts.join("; ");
    }
    actions: (frozen.length > 0 ? [{
        glyph: Theme.glyph.resume,
        name: "Thaw It",
        verb: "thaw",
        stays: true,
        // The guard's status says it's thawed, which takes the row away.
        run: () => Watchers.act({ act: "disk-thaw" }, null)
    }] : []).concat(accountStopped ? [{
        glyph: Theme.glyph.resume,
        name: "Start the Guard",
        stays: true,
        // Its status beating again is what says it started.
        run: () => Watchers.act({ act: "restart-unit", unit: "disk-guard.service" }, null)
    }] : [])

    // Everything the guards say of the disk shortest of room, under Now's
    // row (Drawers.js), each figure said as the Disks place says it, and
    // what either guard cleared last.
    drawer: worst === null ? null : () => Drawers.disk({
        worst: piece.worst,
        holders: piece.holders,
        cleared: piece.said.reduce((all, status) => all.concat(status.cleared ?? []), [])
            .sort((one, other) => other.at - one.at)
    }, {
        now: Date.now() / 1000,
        named: id => Words.name(id),
        said: Words.of("on:disk").said,
        shown: bytes => piece.shown(bytes),
        duration: seconds => piece.duration(seconds),
        within: path => Files.within(path),
        time: at => Qt.formatDateTime(new Date(at * 1000), "HH:mm")
    })

    // What the guard said of the last Keep or Release, for the foot: the
    // one sentence it prints, or why it couldn't.
    property string told: ""

    // Keeps a folder from the guard's clearing, for Joel's reason, or lets
    // it go again; the account's guard rewrites its status before it
    // answers, so the row has flipped by the time the foot says so.
    function keep(path, reason) {
        tell("disk-keep", { path: path, reason: reason });
    }

    function release(path) {
        tell("disk-unkeep", { path: path });
    }

    // Asks the account's guard to clear what it would take of a folder
    // now, which it starts on its next step, marking the holder clearing.
    function clear(path) {
        tell("disk-clear", { path: path });
    }

    // One at a time, as the guard's foot says one sentence.
    property bool guardAsked: false

    function tell(act, said) {
        if (guardAsked)
            return;
        guardAsked = true;
        told = "\u2026";
        Watchers.act(Object.assign({ act: act }, said), answer => {
            piece.guardAsked = false;
            piece.told = piece.toldOf(answer);
        });
    }

    // The foot's words for an act's answer, as every place words one: what
    // the guard printed where it was done, the daemon's words for a
    // refusal, else "couldn't", and "not answering" where nothing came. A
    // refusal's cause goes to the journal, never the foot.
    function toldOf(answer) {
        if (answer === null || answer === undefined)
            return (Words.of("on:disk").said ?? {}).silent ?? "";
        if (answer.error)
            console.info(`disk: the guard's act was refused: ${answer.error.code}: `
                + `${answer.error.message ?? ""}`);
        return Ready.refusal(answer, Words.field.becoming ?? {}) || (answer.said ?? "");
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
    // critically full, never as the guard's status file spells them.
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

    // Sizes as every size reads, decimal at three figures, "19.3 GB", for
    // this piece and the Disks place both (VKCF3Z, 2026-11-08).
    function shown(bytes) {
        return FileFacts.weighed(bytes);
    }

    // How long, "5 h 57 min", and past a day in days (Drawers.js), for
    // this piece and the Disks place both.
    function duration(seconds) {
        return Drawers.span(seconds, Words.of("on:disk").said ?? {});
    }

    // Each guard's status, the account's and the machine's, as the machine
    // daemon tells it: what it says, whether it's gone quiet, and how its
    // unit stood as it did.
    component Status: QtObject {
        property var said: null
        property bool silent: false
        property var unit: null
    }

    Status {
        id: account
        said: Watchers.guard
        silent: Watchers.guardSilent
        unit: Watchers.guardUnit
    }

    Status {
        id: machine
        said: Watchers.machineGuard
        silent: Watchers.machineGuardSilent
        unit: Watchers.machineGuardUnit
    }
}
