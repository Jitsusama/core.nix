// How this machine is doing, answered in the field: "cpu" or "load" for how
// busy its processors have been since last asked, "memory" or "ram" for
// what's in use, and "uptime" for how long it's been up, all read from the
// kernel's files under /proc; and "disk", "storage" or "free space" for
// each disk's room and how the disk guard stands, from the margin's disk
// piece (pieces/Disk.qml), so that fact has one home; and "queue" or
// "what's running" for the heavy work the machine's queue runs and holds,
// each job a row going to its session's window, from what the harness
// writes (Jobs.qml). Nothing is spawned.
import qs
import Quickshell.Io
import QtQuick
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["cpu", "memory", "uptime"]

    // The processors' counters when last asked, to tell how busy they've
    // been since; the first time, since boot.
    property var before: null

    function glyph(code) {
        return String.fromCodePoint(code);
    }

    function gib(kib) {
        return `${Numbers.format(kib / 1048576, 1, settings.group, settings.point)} GiB`;
    }

    function percent(part) {
        return `${Math.round(part * 100)}%`;
    }

    // The words it answers whole, and by their start (Provider.qml).
    // Each answer and the words that ask it, best first.
    readonly property var kinds: ({
        memory: ["memory", "ram", "mem", "swap", "free memory", "memory use"],
        cpu: ["processor", "cpu", "cores", "load", "load average", "cpu use"],
        uptime: ["uptime", "since boot", "how long up", "boot time"],
        disks: ["disk", "storage", "free space", "disk space", "space", "drive", "ssd", "how full"],
        queue: ["queue", "jobs", "what's running", "whats running", "what's waiting", "whats waiting", "waiting",
            "heavy work", "builds", "vm tests", "machine queue"]
    })
    questions: Object.keys(kinds).reduce((all, kind) => all.concat(kinds[kind]), [])

    function answer(typed) {
        const question = asks(typed);
        switch (Object.keys(kinds).find(kind => kinds[kind].includes(question))) {
        case "cpu":
            return cpu();
        case "memory":
            return memory();
        case "uptime":
            return uptime();
        case "disks":
            return disks();
        case "queue":
            return queue();
        default:
            return null;
        }
    }

    // How long ago an ISO time was, as a clock reads it: minutes, or hours
    // and minutes.
    function since(at) {
        const minutes = Math.max(0, Math.floor((Date.now() - Date.parse(at)) / 60000));
        return minutes < 60 ? `${minutes} min` : `${Math.floor(minutes / 60)}:${String(minutes % 60).padStart(2, "0")}`;
    }

    readonly property var kindNames: ({
        "vm-test": "VM test",
        "vm-check": "VM check",
        "benchmark": "benchmark",
        "build": "build"
    })

    // The queue: what runs and what waits, each going to the window of the
    // session it's for, or copying what it is when none is open.
    function queue() {
        if (Jobs.said === null)
            return null;
        const windowed = job => Sessions.running.find(session => session.presence?.session === job.owner?.session) ?? null;
        const rows = Jobs.jobs.map(job => {
            const session = windowed(job);
            const kind = kindNames[job.kind] ?? job.kind;
            const fields = {
                name: job.what,
                side: job.state === "waiting"
                    ? `${kind} \u00b7 waiting ${since(job.since)}, ${Numbers.ordinal(job.place)} in line`
                    : `${kind} \u00b7 ${since(job.since)}`,
                glyph: Theme.glyph[job.state === "waiting" ? "queued" : "running"],
                hue: job.state === "waiting" ? "muted" : "action",
                plain: true
            };
            return session === null ? copyRow(fields) : thing(Object.assign(fields, {
                verb: "switch",
                later: true,
                run: () => Niri.act({ FocusWindow: { id: session.window.id } })
            }));
        });
        const room = Jobs.said.machine ?? {};
        const counted = [Jobs.running.length > 0 ? `${Jobs.running.length} running` : "",
            Jobs.waiting.length > 0 ? `${Jobs.waiting.length} waiting` : ""].filter(part => part !== "").join(", ");
        const wasted = Jobs.said.wasted?.today ?? 0;
        return {
            headline: counted === "" ? "nothing queued" : counted,
            rows: rows,
            note: Jobs.silent ? "the queue has stopped saying"
                : [room.availableGib !== undefined ? `${room.availableGib} GiB to spare` : "",
                    room.pressure?.cpu !== undefined ? `the processors ${Math.round(room.pressure.cpu)}% held up` : "",
                    wasted > 0 ? `${wasted} min waited with room today` : ""].filter(part => part !== "").join(" \u00b7 ")
        };
    }

    function disks() {
        const fact = Facts.all.find(fact => fact.entry.name === "disk");
        const disk = fact ? fact.piece : null;
        if (!disk || !disk.present)
            return null;
        const chip = glyph(0xf02ca);
        // The worst band first, as the disk piece judges it, so the first
        // row is the headline's, and within a band the least room.
        const rows = disk.disks
            .slice()
            .sort((a, b) => disk.worse(a, b) ? -1 : disk.worse(b, a) ? 1 : a.free - b.free)
            .map(each => copyRow({
                name: `${disk.shown(each.free)} free`,
                side: each.band === "healthy" ? each.path : `${each.path} \u00b7 ${each.band}`,
                glyph: chip
            }));
        return {
            headline: disk.worst === null ? "nothing watched" : `${disk.shown(disk.worst.free)} free on ${disk.worst.path}`,
            rows: rows,
            note: disk.silent ? "the disk guard has stopped saying" : "from the disk guard"
        };
    }

    function cpu() {
        const lines = read(stat).split("\n").filter(line => /^cpu\d*\s/.test(line));
        const counters = lines.map(line => {
            const f = line.trim().split(/\s+/).slice(1).map(Number);
            const idle = f[3] + (f[4] ?? 0);
            return { busy: f.reduce((a, b) => a + b, 0) - idle, all: f.reduce((a, b) => a + b, 0) };
        });
        const share = (now, then) => {
            const all = now.all - (then ? then.all : 0);
            return all > 0 ? (now.busy - (then ? then.busy : 0)) / all : 0;
        };
        const was = before;
        before = counters;
        const total = share(counters[0], was ? was[0] : null);
        const load = read(loadavg).trim().split(/\s+/).slice(0, 3);
        const chip = glyph(0xf061a);
        const rows = [
            copyRow({ name: load.join("  "), side: "load, 1, 5 and 15 minutes", glyph: chip }),
            copyRow({ name: `${counters.length - 1} processors`, side: "", glyph: chip })
        ];
        const busiest = counters.slice(1)
            .map((now, i) => ({ i: i, part: share(now, was ? was[i + 1] : null) }))
            .sort((a, b) => b.part - a.part)[0];
        if (busiest)
            rows.push(copyRow({
                name: `${percent(busiest.part)} on processor ${busiest.i}`,
                side: "busiest",
                glyph: chip
            }));
        return {
            headline: `${percent(total)} busy`,
            rows: rows,
            note: was ? "since you last asked" : "since the machine started"
        };
    }

    function memory() {
        const info = {};
        for (const line of read(meminfo).split("\n")) {
            const m = /^(\w+):\s+(\d+)/.exec(line);
            if (m)
                info[m[1]] = Number(m[2]);
        }
        const used = info.MemTotal - info.MemAvailable;
        const chip = glyph(0xf035b);
        const rows = [
            copyRow({ name: gib(info.MemAvailable), side: "available", glyph: chip }),
            copyRow({ name: gib(info.MemTotal), side: "in all", glyph: chip })
        ];
        if (info.SwapTotal > 0)
            rows.push(copyRow({
                name: `${gib(info.SwapTotal - info.SwapFree)} of ${gib(info.SwapTotal)}`,
                side: "swap",
                glyph: chip
            }));
        return {
            headline: `${gib(used)} used, ${percent(used / info.MemTotal)}`,
            rows: rows,
            note: ""
        };
    }

    function uptime() {
        const seconds = Number(read(up).trim().split(/\s+/)[0]);
        const days = Math.floor(seconds / 86400);
        const hours = Math.floor(seconds % 86400 / 3600);
        const minutes = Math.floor(seconds % 3600 / 60);
        const parts = [];
        if (days > 0)
            parts.push(`${days} ${days === 1 ? "day" : "days"}`);
        if (hours > 0)
            parts.push(`${hours} ${hours === 1 ? "hour" : "hours"}`);
        if (days === 0)
            parts.push(`${minutes} ${minutes === 1 ? "minute" : "minutes"}`);
        return { headline: `up ${parts.join(", ")}`, rows: [], note: "" };
    }

    FileView {
        id: stat
        path: "/proc/stat"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: loadavg
        path: "/proc/loadavg"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: meminfo
        path: "/proc/meminfo"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: up
        path: "/proc/uptime"
        blockLoading: true
        printErrors: false
    }
}
