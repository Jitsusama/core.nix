// How this machine is doing, answered in the field: "cpu" or "load" for how
// busy its processors have been since last asked, "memory" or "ram" for
// what's in use, and "uptime" for how long it's been up. All of it is read
// from the kernel's files under /proc, so nothing is spawned.
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

    function answer(typed) {
        const text = typed.trim().toLowerCase();
        if (/^(cpu|cpus|load|processors?)$/.test(text))
            return cpu();
        if (/^(memory|ram|mem|swap)$/.test(text))
            return memory();
        if (text === "uptime")
            return uptime();
        return null;
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
