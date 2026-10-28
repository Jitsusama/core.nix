// How this machine is doing, answered in the field, each thing with its
// parts a step in:
//
// - "cpu", "processor", "cores" or "load": how busy the processors are, and
//   their Cores (one reading each, oldest core first), which are Maxed, the
//   Clock, the Temperature (as Heat reads it), the programs Using it, the
//   Load, the Kinds of core and the Model.
// - "memory" or "ram": what's Used, the Pressure (how long something waited
//   for it), the Swap, what's Cached, the programs Using it, and its Type,
//   as the machine read it at boot.
// - "disk", "storage" or "free space": the Space each disk has, as the disk
//   guard says it (pieces/Disk.qml, so that fact has one home), or the file
//   system where no guard watches, then how fast the disk is Reading and
//   Writing, how Busy it is, its Latency, Temperature, Health and Model.
// - "uptime": how long the machine has been up.
//
// A part has its own words too, "disk speed" or "ram type", which answer
// it alone. The words that ask each, and every part's name, are the
// vocabulary's (launcher/words.json, Words.qml). Everything is read from
// the kernel's files when asked, on the shell's own thread, since handing
// each small file to a reader thread and waiting for it cost more than the
// read under load; and a rate is the difference between two reads, so the
// last is kept with its time. While the launcher shows one of these, or
// holds it stepped into, it's read again once a second, and never more:
// nothing is read for a key. An answer's parts are made only once they're
// looked at, so a headline alone reads no more than it says. What programs
// are running is read by one spawned cat, only while the parts saying them
// are shown, the disk's health by asking UDisks, with no prompt, and the
// file system's room by stat where no guard says it, each at most once a
// second while shown, and the health once a minute; the programs once as
// the launcher opens, too.
import qs
import Qt.labs.folderlistmodel
import Quickshell.Io
import QtQuick
import "Heat.js" as Heat
import "Machine.js" as Machine
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["cpu", "memory", "uptime", "disk speed"]

    // Each answer by the thing the vocabulary names it as (Words.qml),
    // whose words ask it, best first, whole and by their start
    // (Provider.qml).
    readonly property var kinds: ({
        memory: "machine:memory",
        cpu: "machine:cpu",
        uptime: "machine:uptime",
        disks: "machine:disk"
    })
    // A part asked for alone, by its own words, as the thing and the
    // part's name.
    readonly property var parts: {
        const out = {};
        for (const kind of Object.keys(kinds))
            for (const part of Words.of(kinds[kind]).parts)
                for (const question of Words.answers(part))
                    out[question] = [kind, Words.name(part)];
        return out;
    }
    questions: Object.keys(kinds).reduce((all, kind) => all.concat(Words.started(kinds[kind])), [])
        .concat(Object.keys(parts))

    function answer(typed) {
        const question = asks(typed);
        if (question === "")
            return null;
        const part = parts[question];
        if (part)
            return alone(part[0], part[1]);
        return told(Object.keys(kinds).find(kind => Words.answers(kinds[kind]).includes(question)));
    }

    // A part's name, as the vocabulary says it: "Cores" for
    // machine:cpu:cores.
    function named(id, fill) {
        return Words.name(`machine:${id}`, "", fill);
    }

    function told(kind) {
        switch (kind) {
        case "cpu":
            return cpu();
        case "memory":
            return memory();
        case "uptime":
            return uptime();
        case "disks":
            return disks();
        default:
            return null;
        }
    }

    // One part as an answer: its figure as the headline, its own parts a
    // step in, and Return copying what it shows, as every value does.
    function alone(kind, name) {
        const whole = told(kind);
        const part = whole ? whole.rows.find(row => row.name === name) : null;
        if (!part)
            return null;
        return {
            headline: part.side,
            copy: part.copied ?? part.side ?? "",
            graph: part.graph,
            glyph: whole.glyph,
            hue: "machine",
            title: part.name,
            rows: part.verbs ? part.verbs() : [],
            note: part.note !== "" ? part.note : whole.note,
            live: provider
        };
    }

    // An answer that's asked again each second while it's shown or held
    // (the Timer below), so its figures move, and its parts stepped into
    // are never more than a second old. Its parts are made by `parts` only
    // when they're first looked at, and it says it has some (`parted`), so
    // the headline alone, asked each second, makes none of them.
    function fresh(fields, parts) {
        const said = Object.assign({ live: provider, parted: true, hue: "machine" }, fields);
        let made = null;
        Object.defineProperty(said, "rows", {
            enumerable: true,
            get: () => made ?? (made = parts())
        });
        return said;
    }

    // How rows are drawn. A figure says something and Return closes; a leaf
    // worth having, such as a model, copies what it says; a part holding
    // more (Provider's holderRow) steps in on Return as on the right arrow,
    // wearing its thing's glyph.
    function figure(fields) {
        return copyRow(Object.assign({ copy: "", hue: "machine", glyph: "", note: "" }, fields));
    }

    // A part whose programs aren't read yet, as they're read only once the
    // launcher opens: listed from the first, saying it's still reading, so
    // its place is held and nothing below it moves once they are.
    function reading(name) {
        return figure({ name: name, side: "\u2026" });
    }

    // The programs using most of something, known by their place rather
    // than their name, so the first stays first as the programs change
    // and the rows say who it is now, rather than swapping places.
    function ranked(what, at) {
        return `machine:${what}:using:${at + 1}`;
    }

    function leaf(fields) {
        return copyRow(Object.assign({
            copy: fields.side,
            copied: fields.side,
            hue: "machine",
            glyph: "",
            note: ""
        }, fields));
    }

    // A graph a row draws (Choice.qml): a bar has one value, a strip one a
    // member and a spark one a read, the newest last, each 0 to 1.
    readonly property int graphed: 20

    function bar(value, warn) {
        return { shape: "bar", values: [Math.max(0, Math.min(1, value))], warn: warn === true };
    }

    // A strip of more members than it draws gives each of its places the
    // most of the members it stands for, so a maxed core still shows.
    function strip(values) {
        const per = Math.ceil(values.length / graphed);
        const drawn = [];
        for (let at = 0; at < values.length; at += per)
            drawn.push(Math.max(...values.slice(at, at + per)));
        return { shape: "strip", values: drawn, warn: false };
    }

    function spark(values, least) {
        const drawn = Machine.spark(values.slice(-graphed), least);
        return { shape: "spark", values: drawn, warn: false };
    }

    // Figures, written with the machine's own grouping and point.
    function number(value, places) {
        return Numbers.format(value, places, settings.group, settings.point);
    }

    function gib(kib) {
        return `${number(kib / 1048576, 1)} GiB`;
    }

    function percent(part) {
        return `${Math.round(part * 100)}%`;
    }

    // Bytes in decimal units, as a disk's maker counts them.
    function bytes(count) {
        const units = [[1e12, "TB"], [1e9, "GB"], [1e6, "MB"], [1e3, "kB"]];
        const unit = units.find(each => count >= each[0]);
        if (!unit)
            return `${Math.round(count)} B`;
        const scaled = count / unit[0];
        return `${number(scaled, scaled < 10 ? 1 : 0)} ${unit[1]}`;
    }

    function rate(count) {
        return `${bytes(count)}/s`;
    }

    function degrees(value) {
        return `${number(value, 0)} \u00b0C`;
    }

    // A temperature part: the processor's or the disk's whole reading, as
    // Heat's Processor and Disk say it and drawn the same way, against
    // the most it's meant to reach and warning near it (Heat.js's level).
    // It's one figure here, since its detail, each core, the disk's other
    // sensors, its history and how often it's slowed, is Heat's alone.
    function heated(id, heat, part) {
        const said = Heat.level(heat.value, heat.high, heat.most, part);
        return figure({
            name: named(id),
            side: degrees(heat.value),
            graph: bar(said.value, said.warn),
            note: heat.most ? `at most ${degrees(heat.most)}` : ""
        });
    }

    function ghz(khz) {
        return `${number(khz / 1e6, 1)} GHz`;
    }

    // Each thing's last two reads, { last, before }, each { at, said }: a
    // rate is the difference between them. Read again once a second at
    // most, or once a moment after a read with nothing recent before it, so
    // the parts stepped into soon after asking say what's happening now
    // rather than since the last time it was asked.
    property var reads: ({})
    // Each spark's reads, the newest last.
    property var series: ({})
    // What's worked out once: what's fixed about the processors, and the
    // machine's disk.
    readonly property var memo: ({ processors: null, block: "" })
    readonly property int second: 1000
    readonly property int soonest: 200
    // Reads further apart than this aren't a rate of now, and start a
    // spark afresh.
    readonly property int stale: 5000

    function settled(kept) {
        return kept !== undefined && kept.before !== null && kept.last.at - kept.before.at <= stale;
    }

    function due(kept, now) {
        if (!kept || !kept.last)
            return true;
        const age = now - kept.last.at;
        return age >= second || (!settled(kept) && age >= soonest);
    }

    // Reads a thing again if it's due, and gives its reads.
    function sampled(kind, reader) {
        const now = Date.now();
        const kept = reads[kind];
        if (due(kept, now)) {
            reads[kind] = { last: { at: now, said: reader() }, before: kept ? kept.last : null };
            if (!settled(reads[kind]))
                soon.restart();
        }
        return reads[kind];
    }

    // Seconds between a thing's two reads, or null with none before.
    function between(kept) {
        return kept.before ? (kept.last.at - kept.before.at) / 1000 : null;
    }

    // What a figure between two reads stands for, as the footer says it.
    function since(kept) {
        return !kept.before ? "since the machine started"
            : settled(kept) ? "just now" : "since you last asked";
    }

    // Adds a read to a spark, starting it afresh after a gap. A first read
    // says only what's been since the machine started, so it adds nothing.
    function push(name, value, kept) {
        if (!kept.before)
            return;
        const drawn = settled(kept) ? series[name] ?? [] : [];
        series[name] = drawn.concat([value]).slice(-graphed);
    }

    // The processors.

    // What's fixed about them, read once: the model, each core's most clock,
    // and which cores are for performance.
    function fixed() {
        if (memo.processors !== null)
            return memo.processors;
        const named = /^model name\s*:\s*(.*)$/m.exec(read(cpuinfo));
        const counted = Machine.counters(read(stat)).cores;
        const maxes = counted.map(core => ({
            n: core.n,
            max: Number(said(`/sys/devices/system/cpu/cpu${core.n}/cpufreq/cpuinfo_max_freq`)) || 0
        }));
        const performance = Machine.cpus(said("/sys/devices/cpu_core/cpus"));
        memo.processors = {
            model: named ? Machine.model(named[1]) : "",
            maxes: maxes,
            kinds: Machine.kinds(performance, maxes)
        };
        return memo.processors;
    }

    function cpuRead() {
        const info = fixed();
        return sampled("cpu", () => ({
            counters: Machine.counters(read(stat)),
            clocks: info.maxes.map(core => Number(said(
                `/sys/devices/system/cpu/cpu${core.n}/cpufreq/scaling_cur_freq`)) || 0),
            load: read(loadavg).trim().split(/\s+/).slice(0, 3),
            heat: processorHeat()
        }));
    }

    // The processor's temperature, as Heat says it: its package's sensor
    // alone where its chip has one, rather than every core's read to find
    // it, and every sensor only where it hasn't.
    function processorHeat() {
        const chip = sensors.chips().find(each => each.part === "processor");
        if (!chip)
            return null;
        const whole = sensors.readings(chip, "temp", label => Machine.namesWhole(label));
        return whole.length > 0 ? sensors.degrees(chip, whole[0]) : sensors.processor();
    }

    function cpu() {
        const kept = cpuRead();
        const now = kept.last.said;
        if (now.counters.whole === null)
            return null;
        const then = kept.before ? kept.before.said.counters : null;
        const total = Machine.busy(now.counters.whole, then ? then.whole : null);
        return fresh({
            headline: `${percent(total)} busy`,
            graph: bar(total),
            glyph: Words.glyph(kinds.cpu),
            title: Words.name(kinds.cpu),
            note: since(kept)
        }, () => cpuParts(kept));
    }

    // The processors' parts, from the reads the headline said.
    function cpuParts(kept) {
        list();
        const info = fixed();
        const now = kept.last.said;
        const cores = Machine.loads(now.counters, kept.before ? kept.before.said.counters : null);
        const kindOf = n => info.kinds ? info.kinds[n] : "";
        const chip = Words.glyph(kinds.cpu);
        const rows = [holderRow({
            name: named("cpu:cores"),
            side: `${cores.length}`,
            graph: strip(cores.map(core => core.load)),
            glyph: chip,
            note: since(kept)
        }, () => cores.map((core, i) => figure({
            name: named("cpu:core", { n: core.n }),
            side: percent(core.load),
            graph: bar(core.load),
            note: [kindOf(core.n) === "efficient" ? "an efficient core"
                : kindOf(core.n) ? `a ${kindOf(core.n)} core` : "",
                now.clocks[i] > 0 ? `at ${ghz(now.clocks[i])}` : ""]
                .filter(part => part !== "").join(", ")
        }))), figure({
            name: named("cpu:maxed"),
            side: Machine.maxed(cores),
            note: "at 95% or more"
        })];
        const clocked = now.clocks.filter(clock => clock > 0);
        if (clocked.length > 0) {
            const mean = clocked.reduce((a, b) => a + b, 0) / clocked.length;
            const most = info.maxes.reduce((a, b) => a + b.max, 0) / info.maxes.length;
            rows.push(figure({
                name: named("cpu:clock"),
                side: ghz(mean),
                graph: most > 0 ? bar(mean / most) : undefined,
                note: most > 0 ? `on average, of ${ghz(most)} at most` : "on average"
            }));
        }
        if (now.heat)
            rows.push(heated("cpu:temperature", now.heat, "processor"));
        const users = programs
            ? Machine.using(programs.last.said, programs.before?.said ?? null) : [];
        if (users.length > 0)
            rows.push(holderRow({
                name: named("cpu:using"),
                side: `${users[0].name} ${percent(users[0].value)}`,
                glyph: chip,
                note: "of the whole processor"
            }, () => users.map((user, at) => figure({
                key: ranked("cpu", at),
                name: user.name,
                side: percent(user.value),
                graph: bar(user.value)
            }))));
        else if (!programs)
            rows.push(reading(named("cpu:using")));
        if (now.load.length === 3)
            rows.push(figure({
                name: named("cpu:load"),
                side: now.load.join(" \u00b7 "),
                note: "over 1, 5 and 15 minutes"
            }));
        if (info.kinds) {
            const order = ["performance", "efficient", "low-power"];
            const present = order.filter(kind => cores.some(core => kindOf(core.n) === kind));
            const inKind = kind => cores.map((core, i) => ({ core: core, clock: now.clocks[i] }))
                .filter(each => kindOf(each.core.n) === kind);
            rows.push(holderRow({
                name: named("cpu:kinds"),
                side: present.map(kind => inKind(kind).length).join(" \u00b7 "),
                glyph: chip,
                note: present.map(kind => `${inKind(kind).length} ${kind}`).join(", ")
            }, () => present.map(kind => {
                const members = inKind(kind);
                const clocks = members.map(each => each.clock).filter(clock => clock > 0);
                const numbers = members.map(each => each.core.n);
                const last = numbers[numbers.length - 1];
                const unbroken = last - numbers[0] === numbers.length - 1;
                return figure({
                    name: named(`cpu:${kind}`),
                    side: clocks.length > 0
                        ? `${members.length} at ${ghz(clocks.reduce((a, b) => a + b, 0)
                            / clocks.length)}`
                        : `${members.length}`,
                    graph: strip(members.map(each => each.core.load)),
                    note: numbers.length === 1 ? `core ${numbers[0]}`
                        : unbroken ? `cores ${numbers[0]} to ${last}`
                        : `cores ${numbers.slice(0, -1).join(", ")} and ${last}`
                });
            })));
        }
        if (info.model !== "")
            rows.push(leaf({ name: named("cpu:model"), side: info.model }));
        return rows;
    }

    // The processes, read by one cat of each one's stat after the page
    // size and /proc/stat, so their ticks and the machine's come from one
    // moment: { last, before }, as reads keeps a thing's. They're read as
    // the launcher opens too, so the programs Using the processors and the
    // memory are there the first time either is asked, rather than a read
    // or two later: the processors' since the last read, or since the
    // machine started, until another a moment later says now. A read is
    // made sense of only when it's asked, so opening costs one cat and
    // nothing on the shell's own thread.
    property var programs: null

    function list() {
        if (lister.running || (settings.processes ?? "") === "")
            return;
        if (due(programs, Date.now()))
            lister.running = true;
        else if (!settled(programs) && !soon.running)
            soon.restart();
    }

    function listed(text) {
        let said = null;
        const last = { at: Date.now() };
        Object.defineProperty(last, "said", {
            enumerable: true,
            get: () => said ?? (said = Machine.processes(text))
        });
        programs = { last: last, before: programs?.last ?? null };
        if (!settled(programs))
            soon.restart();
        if (shown)
            provider.answered();
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                provider.list();
        }
    }

    // The memory.

    function memoryRead() {
        return {
            info: Machine.meminfo(read(meminfo)),
            pressure: Machine.pressure(read(memoryPressure)),
            modules: Machine.modules(read(memoryModules))
        };
    }

    function memory() {
        const kept = sampled("memory", memoryRead);
        const info = kept.last.said.info;
        if (!(info.MemTotal > 0))
            return null;
        const used = info.MemTotal - info.MemAvailable;
        return fresh({
            headline: `${gib(used)} used, ${percent(used / info.MemTotal)}`,
            graph: bar(used / info.MemTotal),
            glyph: Words.glyph(kinds.memory),
            title: Words.name(kinds.memory),
            note: ""
        }, () => memoryParts(kept));
    }

    // The memory's parts, from the read the headline said.
    function memoryParts(kept) {
        list();
        const now = kept.last.said;
        const info = now.info;
        const used = info.MemTotal - info.MemAvailable;
        const rows = [figure({
            name: named("memory:used"),
            side: `${gib(used)} of ${gib(info.MemTotal)}`,
            graph: bar(used / info.MemTotal),
            note: `${gib(info.MemAvailable)} available`
        })];
        if (now.pressure !== null)
            rows.push(figure({
                name: named("memory:pressure"),
                side: `${percent(now.pressure)} held up`,
                graph: bar(now.pressure, now.pressure >= 0.1),
                note: "of the last ten seconds, waiting for memory"
            }));
        if (info.SwapTotal > 0)
            rows.push(figure({
                name: named("memory:swap"),
                side: `${gib(info.SwapTotal - info.SwapFree)} of ${gib(info.SwapTotal)}`,
                graph: bar((info.SwapTotal - info.SwapFree) / info.SwapTotal)
            }));
        if (info.Cached !== undefined)
            rows.push(figure({
                name: named("memory:cached"),
                side: gib(info.Cached),
                graph: bar(info.Cached / info.MemTotal),
                note: "files kept in memory, given up when it's wanted"
            }));
        const holders = programs ? Machine.holding(programs.last.said) : [];
        if (holders.length > 0)
            rows.push(holderRow({
                name: named("memory:using"),
                side: `${holders[0].name} ${gib(holders[0].value / 1024)}`,
                glyph: Words.glyph(kinds.memory),
                note: "each program's processes together"
            }, () => holders.map((each, at) => figure({
                key: ranked("memory", at),
                name: each.name,
                side: gib(each.value / 1024),
                graph: bar(each.value / 1024 / info.MemTotal)
            }))));
        else if (!programs)
            rows.push(reading(named("memory:using")));
        const modules = now.modules;
        if (modules) {
            const type = modules.speed !== ""
                ? `${modules.type} at ${modules.speed}` : modules.type;
            const counted = modules.count === 1 ? "module" : "modules";
            rows.push(leaf({
                name: named("memory:type"),
                side: type,
                note: [modules.count > 0 && modules.size !== ""
                    ? `${modules.count} ${counted} of ${modules.size}` : "",
                    modules.maker !== "" ? `by ${modules.maker}` : ""]
                    .filter(part => part !== "").join(", ")
            }));
        }
        return rows;
    }

    // The disk.

    // The machine's disk: of the disks the kernel lists, leaving out
    // loops, RAM, mapped and optical ones, the biggest, as a laptop's one
    // disk is. Found once its folder's been listed.
    function blockDevice() {
        if (memo.block !== "")
            return memo.block;
        let biggest = 0;
        for (let i = 0; i < blocks.count; i++) {
            const name = blocks.get(i, "fileName");
            if (/^(loop|ram|zram|dm-|md|sr|fd|nbd)/.test(name))
                continue;
            const size = Number(said(`/sys/block/${name}/size`));
            if (size > biggest) {
                biggest = size;
                memo.block = name;
            }
        }
        return memo.block;
    }

    function diskRead() {
        const name = blockDevice();
        if (name === "")
            return null;
        const kept = sampled("disks", () => ({
            counters: Machine.disk(said(`/sys/block/${name}/stat`)),
            up: Number(read(up).trim().split(/\s+/)[0]),
            heat: sensors.disk(name)
        }));
        if (kept.last.said.counters === null)
            return kept;
        // Each read adds once to the sparks.
        if (kept.last !== kept.sparked) {
            kept.sparked = kept.last;
            const moved = diskRates(kept);
            push("read", moved.read, kept);
            push("write", moved.write, kept);
            push("moved", moved.read + moved.write, kept);
        }
        return kept;
    }

    function diskRates(kept) {
        const then = kept.before ? kept.before.said.counters : null;
        return Machine.rates(kept.last.said.counters, then, between(kept) ?? kept.last.said.up);
    }

    // The disk's health, as its controller tells UDisks: { at, said }.
    property var health: null
    property string drive: ""
    readonly property int healthFresh: 60000

    function asksHealth(name) {
        const program = settings.busctl ?? "";
        if (program === "" || name === "" || driveAsked.running || healthAsked.running)
            return;
        if (health !== null && Date.now() - health.at < healthFresh)
            return;
        if (drive === "") {
            driveAsked.command = [program, "--json=short", "get-property",
                "org.freedesktop.UDisks2", `/org/freedesktop/UDisks2/block_devices/${name}`,
                "org.freedesktop.UDisks2.Block", "Drive"];
            driveAsked.running = true;
        } else {
            healthAsked.command = [program, "--json=short", "call", "org.freedesktop.UDisks2",
                drive,
                "org.freedesktop.UDisks2.NVMe.Controller", "SmartGetAttributes", "a{sv}", "1",
                "auth.no_user_interaction", "b", "true"];
            healthAsked.running = true;
        }
    }

    // The file system's room where no guard says it: { at, free, total }.
    property var room: null

    function asksRoom() {
        const program = settings.stat ?? "";
        if (program === "" || roomAsked.running || (room !== null && Date.now() - room.at < second))
            return;
        roomAsked.command = [program, "--file-system", "--format=%a %b %S", "/"];
        roomAsked.running = true;
    }

    function disks() {
        const fact = Facts.all.find(fact => fact.entry.name === "disk");
        const guard = fact && fact.piece && fact.piece.present ? fact.piece : null;
        const name = blockDevice();
        const kept = diskRead();
        asksHealth(name);
        if (guard === null)
            asksRoom();
        const counted = kept !== null && kept.last.said.counters !== null;
        const worst = guard ? guard.worst : null;
        const watching = worst === null ? "nothing watched"
            : `${guard.shown(worst.free)} free on ${worst.path}`;
        const headline = guard ? watching
            : room ? `${bytes(room.free)} free on /`
            : counted ? rate(diskRates(kept).read + diskRates(kept).write) : "";
        if (headline === "")
            return null;
        return fresh({
            headline: headline,
            graph: worst ? bar(1 - worst.free / worst.total, worst.band !== "healthy")
                : room ? bar(1 - room.free / room.total) : undefined,
            glyph: Words.glyph(kinds.disks),
            title: Words.name(kinds.disks),
            note: guard ? guardSays(guard, worst) : counted ? since(kept) : "",
            // The guard's own words, below the rows.
            reads: guard && worst ? worst.says ?? "" : ""
        }, () => diskParts(guard));
    }

    // How the guard stands, in a phrase: the band the worst file system is
    // in and what its last pass did, such as "filling; nothing more is safe
    // to take", so Joel sees why the disk is still short.
    function guardSays(guard, worst) {
        if (guard.silent)
            return "the disk guard has stopped saying";
        const said = [worst && worst.band !== "healthy" ? guard.banded(worst.band) : "",
            guard.context].filter(part => part !== "");
        return said.length > 0 ? said.join("; ") : "from the disk guard";
    }

    // What's filling the disks, as the guards measured it (Joel, 12:27:
    // "why isn't the disk guard helping?"): each holder biggest first, its
    // share of the disk it's on, and whether the guard may clear any of it
    // or why not, with the guard's own sentence below. Beside the part, the
    // biggest, so what matters most is seen without a step in.
    function filling(guard, chip) {
        const said = Words.of("machine:disk:filling").said ?? {};
        const quests = (launcher?.sources ?? []).find(one => one.settings.name === "quests");
        const titled = id => ((quests?.quests ?? []).find(quest => quest.id === id) ?? {}).title;
        const called = holder => {
            const owner = holder.owner ?? {};
            const name = owner.kind === "quest" ? titled(owner.name) || owner.name
                : owner.kind === "worktree" ? String(owner.name).replace(/^worktree-/, "")
                : owner.kind === "other" ? holder.path.replace(Sessions.home, "~")
                : owner.name;
            const made = (said[owner.kind] ?? "{name}").replace("{name}", name);
            return made.charAt(0).toUpperCase() + made.slice(1);
        };
        const each = holder => {
            const disk = guard.holding(holder.path) ?? guard.worst;
            return figure({
                name: called(holder),
                side: guard.shown(holder.bytes),
                graph: disk ? bar(holder.bytes / disk.total) : undefined,
                note: holder.clearable ? said.clears ?? "" : holder.why ?? "",
                reads: holder.says ?? ""
            });
        };
        const minutes = Math.max(0, Math.round((Date.now() / 1000 - guard.holdersAt) / 60));
        const biggest = guard.holders[0];
        return holderRow({
            name: named("disk:filling"),
            side: `${called(biggest)} \u00b7 ${guard.shown(biggest.bytes)}`,
            glyph: chip,
            note: minutes < 1 ? said.now ?? "" : (said.ago ?? "").replace("{minutes}", minutes)
        }, () => guard.holders.map(each));
    }

    function diskParts(guard) {
        const chip = Words.glyph(kinds.disks);
        const rows = [];
        if (guard && guard.disks.length > 0) {
            // The worst band first, as the disk piece judges it, so the
            // first is the headline's, and within a band the least room.
            const watched = guard.disks.slice()
                .sort((a, b) => guard.worse(a, b) ? -1 : guard.worse(b, a) ? 1 : a.free - b.free);
            const each = disk => figure({
                name: disk.path,
                side: disk.band === "healthy" ? `${guard.shown(disk.free)} free`
                    : `${guard.shown(disk.free)} free \u00b7 ${guard.banded(disk.band)}`,
                graph: bar(1 - disk.free / disk.total, disk.band !== "healthy")
            });
            const worst = watched[0];
            const space = {
                name: named("disk:space"),
                side: `${guard.shown(worst.free)} free`,
                graph: bar(1 - worst.free / worst.total, worst.band !== "healthy"),
                note: watched.length > 1 ? `on ${worst.path}, the least of ${watched.length}`
                    : `on ${worst.path}`
            };
            rows.push(watched.length > 1
                ? holderRow(Object.assign(space, { glyph: chip }), () => watched.map(each))
                : figure(space));
            if (guard.holders.length > 0)
                rows.push(filling(guard, chip));
        } else if (room) {
            rows.push(figure({
                name: named("disk:space"),
                side: `${bytes(room.free)} free`,
                graph: bar(1 - room.free / room.total),
                note: `on /, of ${bytes(room.total)}`
            }));
        }
        const kept = reads.disks;
        if (kept && kept.last.said.counters !== null) {
            const moved = diskRates(kept);
            const when = since(kept);
            const floor = 1e6;
            rows.push(holderRow({
                name: named("disk:speed"),
                side: rate(moved.read + moved.write),
                graph: spark(series.moved ?? [], floor),
                glyph: chip,
                note: when
            }, () => [figure({
                name: named("disk:reading"),
                side: rate(moved.read),
                graph: spark(series.read ?? [], floor),
                note: when
            }), figure({
                name: named("disk:writing"),
                side: rate(moved.write),
                graph: spark(series.write ?? [], floor),
                note: when
            })]), figure({
                name: named("disk:busy"),
                side: percent(moved.busy),
                graph: bar(moved.busy),
                note: when
            }), figure({
                name: named("disk:latency"),
                side: moved.latency === null ? "idle"
                    : `${number(moved.latency, moved.latency < 10 ? 2 : 0)} ms`,
                note: moved.latency === null ? "nothing asked of it"
                    : "for each request, on average"
            }));
            const heat = kept.last.said.heat;
            if (heat)
                rows.push(heated("disk:temperature", heat, "disk"));
        }
        if (health !== null && health.said !== null) {
            const smart = health.said;
            const worn = smart.percent_used / 100;
            const failing = smart.avail_spare < smart.spare_thresh || smart.media_errors > 0;
            rows.push(holderRow({
                name: named("disk:health"),
                side: `${smart.percent_used}% worn`,
                graph: bar(worn, failing),
                glyph: chip,
                note: failing ? "its controller says it's failing" : "as its controller says"
            }, () => [figure({
                name: named("disk:spare"),
                side: `${smart.avail_spare}%`,
                graph: bar(smart.avail_spare / 100, smart.avail_spare < smart.spare_thresh),
                note: `it warns below ${smart.spare_thresh}%`
            }), figure({
                name: named("disk:written"),
                side: bytes(smart.total_data_written)
            }), figure({
                name: named("disk:read"),
                side: bytes(smart.total_data_read)
            }), figure({
                name: named("disk:errors"),
                side: `${smart.media_errors}`,
                note: "the disk couldn't read or write"
            }), figure({
                name: named("disk:shutdowns"),
                side: `${smart.unsafe_shutdowns}`,
                note: `of ${smart.power_cycles} times it was powered`
            })]));
        }
        const model = memo.block !== ""
            ? said(`/sys/block/${memo.block}/device/model`).replace(/\s+/g, " ") : "";
        if (model !== "")
            rows.push(leaf({ name: named("disk:model"), side: model }));
        return rows;
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

    // What a file in /sys says, or "" when it says nothing.
    function said(file) {
        probe.path = file;
        return read(probe).trim();
    }

    // While one of its answers is shown, as the answer to what's typed, a
    // row in a category's list or a step held, it's asked again each
    // second, so its figures move; and once a moment after a first read, so
    // a rate stands for now as soon as it can.
    readonly property bool shown: launcher !== null && launcher.open
        && (launcher.answer?.live === provider
            || launcher.held.some(step => step.row.live === provider)
            || launcher.matches.some(row => row.live === provider))

    Timer {
        interval: provider.second
        repeat: true
        running: provider.shown
        onTriggered: provider.answered()
    }

    Timer {
        id: soon
        interval: provider.soonest
        onTriggered: {
            if (provider.shown)
                provider.answered();
        }
    }

    Sensors {
        id: sensors
    }

    // The disks the kernel lists, listed once.
    FolderListModel {
        id: blocks
        folder: "file:///sys/block"
        showFiles: false
        showDotAndDotDot: false
        showHidden: false
        sortField: FolderListModel.Name
    }

    Process {
        id: lister
        command: [provider.settings.processes ?? ""]
        stdout: StdioCollector {
            id: processes
            onStreamFinished: provider.listed(processes.text)
        }
    }

    Process {
        id: driveAsked
        stdout: StdioCollector {
            id: driveSaid
            onStreamFinished: {
                const object = provider.parsed(driveSaid.text).data ?? "";
                // A disk UDisks has no drive for is never asked again.
                provider.drive = typeof object === "string" && object !== "/" ? object : "";
                if (provider.drive !== "")
                    provider.asksHealth(provider.memo.block);
                else
                    provider.health = { at: Date.now() + 1e12, said: null };
            }
        }
    }

    Process {
        id: healthAsked
        stdout: StdioCollector {
            id: healthSaid
            onStreamFinished: {
                const fields = (provider.parsed(healthSaid.text).data ?? [])[0] ?? null;
                const smart = {};
                for (const key of Object.keys(fields ?? {}))
                    smart[key] = fields[key].data;
                // One that isn't NVMe, or that UDisks won't answer without
                // asking Joel, has no health to say, and isn't asked again
                // for a minute.
                provider.health = {
                    at: Date.now(),
                    said: smart.percent_used !== undefined ? smart : null
                };
                provider.answered();
            }
        }
    }

    Process {
        id: roomAsked
        stdout: StdioCollector {
            id: roomSaid
            onStreamFinished: {
                const [free, total, size] = roomSaid.text.trim().split(/\s+/).map(Number);
                provider.room = total > 0
                    ? { at: Date.now(), free: free * size, total: total * size } : null;
                provider.answered();
            }
        }
    }

    FileView {
        id: stat
        path: "/proc/stat"
        preload: false
        blockAllReads: true
        printErrors: false
    }
    FileView {
        id: loadavg
        path: "/proc/loadavg"
        preload: false
        blockAllReads: true
        printErrors: false
    }
    FileView {
        id: meminfo
        path: "/proc/meminfo"
        preload: false
        blockAllReads: true
        printErrors: false
    }
    FileView {
        id: memoryPressure
        path: "/proc/pressure/memory"
        preload: false
        blockAllReads: true
        printErrors: false
    }
    // What the machine read of its memory modules at boot, as root, since
    // the firmware's tables are root's alone.
    FileView {
        id: memoryModules
        path: "/run/memory-modules.json"
        preload: false
        blockAllReads: true
        printErrors: false
    }
    FileView {
        id: cpuinfo
        path: "/proc/cpuinfo"
        preload: false
        blockAllReads: true
        printErrors: false
    }
    FileView {
        id: up
        path: "/proc/uptime"
        preload: false
        blockAllReads: true
        printErrors: false
    }
    FileView {
        id: probe
        preload: false
        blockAllReads: true
        printErrors: false
    }
}
