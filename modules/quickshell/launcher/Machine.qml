// How this machine is doing, answered in the field, each thing with its
// parts a step in:
//
// Each thing's parts lead with what moves most and end with what
// describes it, its Model last (VKCF3Z, 2026-11-07); words.json lists
// them in that order, and `ordered` keeps it.
//
// - "cpu", "processor", "cores" or "load": how busy the processors are,
//   where their Time was Spent, the Pressure on them (how long something
//   waited for one), how often they Switch tasks, take Interrupts and
//   Start processes, their Cores (one reading each, oldest core first,
//   with its kind, clock and temperature), which are Maxed, the Clock and
//   its range, base, driver, governor, preference and turbo, what Held it
//   Back over the last second and each core held, and what Kept it From
//   Resting over the last ten seconds and each part that did (as its own
//   telemetry says, ProcessorTelemetry.qml), the Temperature, the
//   programs Using it, the Load, how many tasks are Running, the NPU's
//   Load, the Kinds of core and the Model with its family, microcode and
//   caches. Both telemetry rows stand whenever the telemetry answers, at
//   0% when nothing holds it back or keeps it from resting, so the rows
//   under them don't move.
// - "memory" or "ram": what's Used, the Pressure, how often pages Fault,
//   the Swapping and Paging, the programs Using it, the Swap, what's
//   Cached, what it's Held By, what's Committed, the programs killed for
//   it (Out of Memory), Every Count the kernel keeps, and its Type, each
//   module as the machine read it at boot.
// - "disk", "storage" or "free space": the Space each disk has, as the disk
//   guard says it (pieces/Disk.qml, so that fact has one home), or the file
//   system where no guard watches, then how fast the disk is Reading,
//   Writing, Discarding and Flushing, the Pressure on it, how Busy it is,
//   each kind of request's Latency, its Queue, its Temperature, Health,
//   File Systems and Model, with its firmware, kind, link, size, block
//   size and scheduler.
// - "uptime": how long the machine has been up, and of that how long it
//   was Asleep and its processors Idle, when it Started and its Kernel.
//
// A part has its own words too, "disk speed" or "ram type", which answer
// it alone. The words that ask each, and every part's name, are the
// vocabulary's (launcher/words.json, Words.qml).
//
// Every figure is the machine daemon's (modules/machine), held over one
// connection to its socket (Daemon.qml), so nothing is read or spawned
// here: the daemon reads the kernel and asks UDisks, and says each
// record as it changes. It reads only while watched, so the processor,
// memory and boot are watched while one of Machine's answers is shown,
// the disks while a Disk answer or row is, and the programs using the machine, which cost it the most to
// read, only while a Using row can be seen: the processor's or the
// memory's parts stepped into, or Using asked for alone. Its
// records come at most once a second, and once more a moment after the
// first, whose rates are since the machine started, so a rate stands for
// now as soon as it can; each answer here is asked again as they come.
// With the daemon down, each thing stays, dimmed, saying it isn't
// answering.
import qs
import Quickshell
import QtQuick
import "Heat.js" as Heat
import "Interfaces.js" as Interfaces
import "Machine.js" as Machine
// Bytes as decimal to three digits, "12.3 GB", as what's told to a session
// says them; it's the launcher's one such reading, kept there for networks.
import "HomeNetwork.js" as HomeNetwork
import "Numbers.js" as Numbers
import "Order.js" as Order
import "Holders.js" as Holders
import "SessionRow.js" as SessionRow

Provider {
    id: provider

    examples: ["cpu", "memory", "uptime", "disk speed"]

    // What talking to a holder's session said, for the foot.
    property string talked: ""

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
            for (const part of Words.of(kinds[kind]).parts ?? [])
                for (const question of Words.answers(part))
                    out[question] = [kind, Words.name(part)];
        return out;
    }
    questions: Object.keys(kinds).reduce((all, kind) => {
        all.push(...Words.started(kinds[kind]));
        return all;
    }, [])
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

    // The kinds of cache Linux names, each a name of its own in the
    // vocabulary (machine:cpu:cache-data), since a name is Title Case.
    readonly property var cacheTypes: ["data", "instruction", "unified"]

    // A part's name, as the vocabulary says it: "Cores" for
    // machine:cpu:cores.
    function named(id, fill) {
        return Words.name(`machine:${id}`, "", fill);
    }

    // What a part's words say beside it, by its id and the phrase's name.
    function says(id, phrase, fill) {
        let said = (Words.of(`machine:${id}`).said ?? {})[phrase] ?? "";
        for (const key of Object.keys(fill ?? {}))
            said = said.replace(`{${key}}`, fill[key]);
        return said;
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
        const part = whole && !whole.asking && !whole.gone
            ? whole.rows.find(row => row.name === name) : null;
        const resting = kind === "cpu" && name === named("cpu:kept-from-resting");
        if (!part)
            return resting && whole ? awaited(whole, name) : whole && (whole.asking || whole.gone)
                ? Object.assign({}, whole, { title: name }) : null;
        return {
            // Kept From Resting stands in a place whether or not anything
            // keeps it from resting, so it's kept there with no parts too;
            // and asked alone, it's what makes the telemetry be read.
            whole: resting,
            about: resting ? kinds.cpu : kind === "disks" ? kinds.disks : undefined,
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

    // Kept From Resting before the telemetry has said it: the reading is
    // asked for only once something shows it, so it holds its place with
    // the fixed placeholder beside it until the first comes, a second or
    // less, and nothing moves when it does. If the reader refuses, it
    // stays there dimmed, saying so. On a machine with no reader it's left
    // out.
    function awaited(whole, name) {
        if (ProcessorTelemetry.missing)
            return null;
        return {
            headline: ProcessorTelemetry.refused ? "" : "\u2026",
            copy: "",
            glyph: whole.glyph,
            hue: "machine",
            title: name,
            rows: [],
            note: ProcessorTelemetry.refused ? "can't be read" : "",
            asking: !ProcessorTelemetry.refused,
            gone: ProcessorTelemetry.refused,
            whole: true,
            about: kinds.cpu,
            live: provider
        };
    }

    // A thing whose record the daemon hasn't said: while it's starting or
    // the launcher has only just opened, its place held with the fixed
    // placeholder, so nothing moves when the record comes a moment later;
    // and while the daemon isn't answering, dimmed, saying so (VKCF3Z's
    // rules for a daemon that's down). It says it has parts, as it will
    // once its record comes, so a place lists it from the start and says
    // it again in place as the record comes (Launcher.qml's relive): a
    // place is made once, and an answer that came later would never join
    // it.
    function unheld(kind) {
        const down = daemon.state === "not answering";
        return {
            headline: down ? "" : "\u2026",
            copy: "",
            glyph: Words.glyph(kinds[kind]),
            hue: "machine",
            title: Words.name(kinds[kind]),
            about: kinds[kind],
            rows: [],
            note: down ? says("cpu", "down") : "",
            asking: !down,
            gone: down,
            parted: true,
            live: provider
        };
    }

    // An answer that's asked again as its records change, so its figures
    // move, and its parts stepped into are never more than a second old.
    // Its parts are made by `parts` only when they're first looked at, and
    // it says it has some (`parted`), so the headline alone makes none of
    // them.
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
    // more (Provider's holderRow) copies its value too, the right arrow
    // stepping into what it holds, and wears its thing's glyph.
    function figure(fields) {
        return copyRow(Object.assign({ copy: "", hue: "machine", glyph: "", note: "" }, fields));
    }

    // A part whose figures aren't held yet, as the programs aren't until a
    // part saying them is first looked at: listed from the first, saying
    // it's still reading, so its place is held and nothing below it moves
    // once they come.
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

    // A measure as Machine.js writes it, with the machine's own point.
    function pointed(said) {
        return settings.point ? said.split(".").join(settings.point) : said;
    }

    function percent(part) {
        return `${Math.round(part * 100)}%`;
    }

    // Bytes, memory's and a disk's alike, in decimal units to three
    // significant digits (Machine.js's bytes).
    function bytes(count) {
        return pointed(Machine.bytes(count));
    }

    function rate(count) {
        return pointed(Machine.rate(count));
    }

    // Two rates running opposite ways, each named, or none.
    function both(inward, outward) {
        return pointed(Machine.both(inward, outward));
    }

    // A count a second, whole once it's past ten.
    function often(count) {
        return `${number(count, count < 10 ? 1 : 0)}/s`;
    }

    function degrees(value) {
        return `${number(value, 0)} \u00b0C`;
    }

    function ghz(khz) {
        return pointed(Machine.hertz(khz));
    }

    function ms(value) {
        return `${Numbers.significant(value, settings.group, settings.point)} ms`;
    }

    // A long time, as Interfaces says one: "3 days, 2 hours".
    function lasted(seconds) {
        return Interfaces.lasted(seconds);
    }

    // A temperature part: the processor's or the disk's whole reading,
    // drawn as Heat draws it, against the most it's meant to reach and
    // warning near it (Heat.js's level). It's one figure here, since its
    // detail, each sensor, its history and how often it's slowed, is
    // Heat's alone.
    function heated(id, heat, part) {
        const said = Heat.level(heat.temperature_c, heat.high_c, heat.critical_c, part);
        return figure({
            name: named(id),
            side: degrees(heat.temperature_c),
            graph: bar(said.value, said.warn),
            note: heat.critical_c ? `at most ${degrees(heat.critical_c)}`
                : heat.high_c ? `at most ${degrees(heat.high_c)}` : ""
        });
    }

    // A pressure part, from the kernel's pressure stall information: the
    // share of the last ten seconds something waited, warning past a
    // tenth, and a step in, the last minute's and five minutes', each with
    // the share in which everything waited where the kernel keeps that.
    function pressured(id, pressure, full) {
        const some = pressure.some;
        const warn = some.avg10_pct >= 10;
        // Each window says which share is which, everything's giving way
        // where it's none.
        const shares = window => full && pressure.full
            ? says("memory:pressure", "pair", { some: number(some[window], 0),
                full: number(pressure.full[window], 0) })
            : says("memory:pressure", "some", { some: number(some[window], 0) });
        const each = (window, name) => figure({
            name: named(`${id}-${name}`),
            side: full && pressure.full && pressure.full[window] > 0 ? shares(window)
                : says("memory:pressure", "some", { some: number(some[window], 0) }),
            graph: bar(some[window] / 100, some[window] >= 10)
        });
        return holderRow({
            name: named(id),
            side: says(id, "side", { pct: number(some.avg10_pct, 0) }),
            graph: bar(some.avg10_pct / 100, warn),
            glyph: Words.glyph(`machine:${id.split(":")[0]}`),
            note: says(id, "note")
        }, () => [each("avg10_pct", "10"), each("avg60_pct", "60"), each("avg300_pct", "300")]);
    }

    // A thing's parts in the order its words list them (VKCF3Z, 2026-11-07:
    // what moves most first, then what describes, the model last), each
    // part made where its figure is, then put in its place.
    function ordered(thing, rows) {
        const order = (Words.of(thing).parts ?? []).map(part => Words.name(part));
        const place = row => {
            const at = order.indexOf(row.name);
            return at < 0 ? order.length : at;
        };
        return rows.map((row, at) => ({ row: row, at: at }))
            .sort((a, b) => place(a.row) - place(b.row) || a.at - b.at)
            .map(each => each.row);
    }

    // What a record's rates are measured over, as a note says it.
    function since(record) {
        return record.since === "boot" ? "since the machine started"
            : record.since === "asked" ? "since you last asked" : "just now";
    }

    // The daemon's records.

    // A record by its key, read beside the revision so a binding is told
    // once a change, or null while it isn't held.
    function record(key) {
        return daemon.revision >= 0 ? daemon.held[key] ?? null : null;
    }

    function processor() {
        return record("processor:0");
    }

    // The disk records, the machine's own first.
    function diskRecords() {
        const held = diskWatch.revision >= 0 ? diskWatch.held : {};
        return Object.keys(held).filter(key => key.startsWith("disk:")).map(key => held[key]);
    }

    function machineDisk() {
        return Machine.biggest(diskRecords());
    }

    // The programs using most of something, in their places, or null while
    // the programs aren't held yet.
    function using(of) {
        if (!users.answered || users.state === "not answering")
            return null;
        const held = users.revision >= 0 ? users.held : {};
        const found = [];
        for (let rank = 1; held[`using:${of}:${rank}`]; rank++)
            found.push(held[`using:${of}:${rank}`]);
        return found;
    }

    // The processors.

    function cpu() {
        const p = processor();
        if (!p || p.busy_ratio === null || p.busy_ratio === undefined)
            return unheld("cpu");
        return fresh({
            headline: `${percent(p.busy_ratio)} busy`,
            graph: bar(p.busy_ratio),
            glyph: Words.glyph(kinds.cpu),
            title: Words.name(kinds.cpu),
            about: kinds.cpu,
            note: since(p)
        }, () => cpuParts(p));
    }

    // The processors' parts, from the record the headline said.
    function cpuParts(p) {
        const chip = Words.glyph(kinds.cpu);
        const cores = (p.cores ?? []).filter(core => core.online);
        const loads = cores.map(core => ({ n: core.n, load: core.busy_ratio ?? 0 }));
        const rows = [holderRow({
            name: named("cpu:cores"),
            side: `${cores.length}`,
            graph: strip(loads.map(core => core.load)),
            glyph: chip,
            note: since(p)
        }, () => cores.map(core => figure({
            name: named("cpu:core", { n: core.n }),
            side: percent(core.busy_ratio ?? 0),
            graph: bar(core.busy_ratio ?? 0),
            note: Machine.items([core.kind ?? "",
                core.clock_khz > 0 ? ghz(core.clock_khz) : "",
                typeof core.temperature_c === "number" ? degrees(core.temperature_c) : ""])
        }))), figure({
            name: named("cpu:maxed"),
            side: Machine.maxed(loads),
            note: "at 95% or more"
        })];
        const spent = Machine.spent(p.time, p.guest === true);
        if (spent.length > 0)
            rows.push(holderRow({
                name: named("cpu:spent"),
                side: percent(p.busy_ratio),
                graph: bar(p.busy_ratio),
                glyph: chip,
                note: ""
            }, () => spent.map(each => figure({
                name: named(`cpu:spent-${each.id}`),
                side: percent(each.share),
                graph: bar(each.share),
                note: says(`cpu:spent-${each.id}`, "note")
            }))));
        // Every task waiting for a processor is never said of the whole
        // machine (the kernel keeps it for a cgroup alone, and says 0
        // here), so only some is.
        if (p.pressure?.some)
            rows.push(pressured("cpu:pressure", p.pressure, false));
        const clocked = cores.filter(core => core.clock_khz > 0);
        if (clocked.length > 0) {
            const mean = clocked.reduce((a, b) => a + b.clock_khz, 0) / clocked.length;
            const tops = cores.map(core => core.max_khz ?? 0).filter(top => top > 0);
            const most = tops.length > 0 ? tops.reduce((a, b) => a + b, 0) / tops.length : 0;
            rows.push(holderRow({
                name: named("cpu:clock"),
                side: ghz(mean),
                graph: most > 0 ? bar(mean / most) : undefined,
                glyph: chip,
                note: most > 0 ? `on average, of ${ghz(most)} at most` : "on average"
            }, () => clockParts(p, cores)));
        }
        // Held Back stands while the telemetry answers, held or not, so the
        // rows under it never move under Joel's selection (VKCF3Z).
        const telemetry = ProcessorTelemetry.reading;
        const held = Machine.heldBack(telemetry);
        if (held && held.cores.length > 0)
            rows.push(holderRow({
                name: named("cpu:held-back"),
                side: percent(held.share),
                graph: bar(held.share),
                glyph: chip,
                note: `by ${Machine.limit(held.by)}, over the last second`
            }, () => held.cores.map(core => figure({
                name: named("cpu:held-back-core", { n: core.n }),
                side: percent(core.share),
                graph: bar(core.share),
                note: `by ${Machine.limit(core.by)}`
            }))));
        else if (telemetry)
            rows.push(figure({
                name: named("cpu:held-back"),
                side: percent(held ? held.share : 0),
                graph: bar(held ? held.share : 0),
                note: held ? `by ${Machine.limit(held.by)}, over the last second`
                    : "nothing holds it back now"
            }));
        if (typeof p.temperature?.temperature_c === "number")
            rows.push(heated("cpu:temperature", p.temperature, "processor"));
        const users = using("cpu");
        if (users && users.length > 0)
            rows.push(holderRow({
                name: named("cpu:using"),
                side: `${users[0].name} ${percent(users[0].share_ratio)}`,
                glyph: chip,
                note: "of the whole processor"
            }, () => users.map((user, at) => figure({
                key: ranked("cpu", at),
                name: user.name,
                side: percent(user.share_ratio),
                graph: bar(user.share_ratio),
                note: user.processes > 1 ? `${user.processes} processes` : ""
            }))));
        else if (!users)
            rows.push(reading(named("cpu:using")));
        if ((p.load ?? []).length === 3)
            rows.push(figure({
                name: named("cpu:load"),
                side: p.load.map(load => number(load, 2)).join(" \u00b7 "),
                note: "over 1, 5 and 15 minutes"
            }));
        if (typeof p.running === "number")
            rows.push(figure({
                name: named("cpu:running"),
                side: `${p.running}`,
                note: Machine.items([p.blocked > 0 ? says("cpu:running", "blocked", {
                    blocked: p.blocked }) : "", p.tasks > 0 ? says("cpu:running", "tasks", {
                    tasks: number(p.tasks, 0) }) : ""])
            }));
        if (typeof p.switches_per_s === "number")
            rows.push(figure({
                name: named("cpu:switches"),
                side: often(p.switches_per_s),
                note: since(p)
            }));
        if (typeof p.interrupts_per_s === "number")
            rows.push(figure({
                name: named("cpu:interrupts"),
                side: often(p.interrupts_per_s),
                note: p.softirqs_per_s > 0 ? says("cpu:interrupts", "note", {
                    deferred: often(p.softirqs_per_s) }) : ""
            }));
        if (typeof p.forks_per_s === "number")
            rows.push(figure({
                name: named("cpu:started"),
                side: often(p.forks_per_s),
                note: says("cpu:started", "note")
            }));
        // The NPU's load sits with the processors', so the two read as a
        // pair (VKCF3Z, 2026-10-29).
        if (p.npu)
            rows.push(p.npu.busy_ratio === null ? reading(named("cpu:npu-load")) : figure({
                name: named("cpu:npu-load"),
                side: percent(p.npu.busy_ratio),
                graph: bar(p.npu.busy_ratio),
                note: Machine.npuClock(p.npu)
            }));
        const order = ["performance", "efficient", "low-power"];
        const present = order.filter(kind => cores.some(core => core.kind === kind));
        if (present.length > 0) {
            const inKind = kind => cores.filter(core => core.kind === kind);
            rows.push(holderRow({
                name: named("cpu:kinds"),
                side: present.map(kind => inKind(kind).length).join(" \u00b7 "),
                glyph: chip,
                note: Machine.items(present)
            }, () => present.map(kind => {
                const members = inKind(kind);
                const clocks = members.map(core => core.clock_khz ?? 0).filter(clock => clock > 0);
                return figure({
                    name: named(`cpu:${kind}`),
                    side: clocks.length > 0
                        ? `${members.length} at ${ghz(clocks.reduce((a, b) => a + b, 0)
                            / clocks.length)}`
                        : `${members.length}`,
                    graph: strip(members.map(core => core.busy_ratio ?? 0)),
                    note: Machine.cores(members.map(core => core.n))
                });
            })));
        }
        if (p.model)
            rows.push(holderRow({
                name: named("cpu:model"),
                side: p.model,
                copy: p.model,
                glyph: chip
            }, () => modelParts(p)));
        // Kept From Resting stands while the telemetry says it, kept from
        // resting or not, as Held Back does, and beside it.
        const resting = ProcessorTelemetry.kept;
        if (resting) {
            const most = resting.by[0];
            const fields = {
                name: named("cpu:kept-from-resting"),
                side: percent(resting.share),
                graph: bar(resting.share),
                note: most ? `by ${Machine.keeper(most.by).said}`
                    : resting.quiet ? "nothing keeps it from resting now"
                    : "by no one part for long"
            };
            rows.push(resting.by.length > 0 ? holderRow(Object.assign({ glyph: chip }, fields),
                () => resting.by.map(part => figure({
                    name: named(`cpu:kept-by-${Machine.keeper(part.by).id}`),
                    side: percent(part.share),
                    graph: bar(part.share)
                }))) : figure(fields));
        }
        return ordered(kinds.cpu, rows);
    }

    // The clock's range, the base it's sold at, and how it's driven.
    function clockParts(p, cores) {
        const lows = cores.map(core => core.min_khz ?? 0).filter(low => low > 0);
        const tops = cores.map(core => core.max_khz ?? 0).filter(top => top > 0);
        const bases = cores.map(core => core.base_khz ?? 0).filter(base => base > 0);
        const rows = [];
        if (lows.length > 0 && tops.length > 0)
            rows.push(figure({
                name: named("cpu:clock-range"),
                side: `${ghz(Math.min(...lows))} to ${ghz(Math.max(...tops))}`,
                note: Math.max(...tops) !== Math.min(...tops)
                    ? says("cpu:clock-range", "kinds", { least: ghz(Math.min(...tops)) }) : ""
            }));
        if (bases.length > 0)
            rows.push(figure({
                name: named("cpu:clock-base"),
                side: Math.min(...bases) === Math.max(...bases) ? ghz(bases[0])
                    : `${ghz(Math.min(...bases))} to ${ghz(Math.max(...bases))}`,
                note: says("cpu:clock-base", "note")
            }));
        if (p.driver)
            rows.push(leaf({ name: named("cpu:clock-driver"), side: p.driver }));
        if (p.governor)
            rows.push(leaf({ name: named("cpu:clock-governor"), side: p.governor }));
        if (p.preference)
            rows.push(leaf({ name: named("cpu:clock-preference"), side: p.preference }));
        if (typeof p.turbo === "boolean")
            rows.push(figure({
                name: named("cpu:clock-turbo"),
                side: p.turbo ? "on" : "off",
                note: says("cpu:clock-turbo", p.turbo ? "on" : "off")
            }));
        return rows;
    }

    // What the processor is: who made it, its family, model and stepping
    // as it says them, its microcode, and its caches.
    function modelParts(p) {
        const rows = [leaf({ name: named("cpu:model-name"), side: p.model })];
        if (p.vendor)
            rows.push(leaf({ name: named("cpu:vendor"), side: p.vendor }));
        if (typeof p.family === "number")
            rows.push(leaf({
                name: named("cpu:family"),
                side: says("cpu:family", "side", { family: p.family,
                    model: p.model_number ?? "", stepping: p.stepping ?? "" })
            }));
        if (p.microcode)
            rows.push(leaf({ name: named("cpu:microcode"), side: p.microcode }));
        const caches = p.caches ?? [];
        if (caches.length > 0)
            rows.push(holderRow({
                name: named("cpu:caches"),
                side: bytes(caches.reduce((all, cache) => all + (cache.size_b ?? 0) * cache.count, 0)),
                glyph: Words.glyph(kinds.cpu),
                note: says("cpu:caches", "note")
            }, () => caches.map(cache => figure({
                // A type Linux names has its own words; another, its own name.
                name: cacheTypes.includes(cache.type)
                    ? named(`cpu:cache-${cache.type}`, { level: cache.level })
                    : named("cpu:cache", { level: cache.level, type: cache.type }),
                side: cache.count > 1 ? `${bytes(cache.size_b)} \u00d7 ${cache.count}`
                    : bytes(cache.size_b),
                note: Machine.items([cache.shared_by > 1
                    ? says("cpu:cache", "shared", { n: cache.shared_by }) : says("cpu:cache", "own"),
                    Machine.cores(cache.cpus ?? []),
                    cache.ways ? says("cpu:cache", "ways", { ways: cache.ways }) : "",
                    cache.line_b ? says("cpu:cache", "line", { line: cache.line_b }) : ""])
            }))));
        return rows;
    }

    // The memory.

    function memory() {
        const m = record("memory:main");
        if (!m || !(m.total_b > 0))
            return unheld("memory");
        return fresh({
            headline: `${bytes(m.used_b)} used, ${percent(m.used_b / m.total_b)}`,
            graph: bar(m.used_b / m.total_b),
            glyph: Words.glyph(kinds.memory),
            title: Words.name(kinds.memory),
            note: ""
        }, () => memoryParts(m));
    }

    // The memory's parts, from the record the headline said.
    function memoryParts(m) {
        const info = m.info_b ?? {};
        const chip = Words.glyph(kinds.memory);
        const rows = [figure({
            name: named("memory:used"),
            side: `${bytes(m.used_b)} of ${bytes(m.total_b)}`,
            graph: bar(m.used_b / m.total_b),
            note: `${bytes(m.available_b)} available`
        })];
        if (m.pressure?.some)
            rows.push(pressured("memory:pressure", m.pressure, true));
        if (info.SwapTotal > 0)
            rows.push(figure({
                name: named("memory:swap"),
                side: `${bytes(info.SwapTotal - info.SwapFree)} of ${bytes(info.SwapTotal)}`,
                graph: bar((info.SwapTotal - info.SwapFree) / info.SwapTotal),
                note: info.SwapCached > 0 ? says("memory:swap", "cached", {
                    cached: bytes(info.SwapCached) }) : ""
            }));
        if (info.Cached !== undefined)
            rows.push(figure({
                name: named("memory:cached"),
                side: bytes(info.Cached),
                graph: bar(info.Cached / m.total_b),
                note: "files kept in memory, given up when it's wanted"
            }));
        const held = Machine.keptBy(info);
        const zswap = Machine.compressed(info);
        if (held.length > 0)
            rows.push(holderRow({
                name: named("memory:held"),
                side: `${named(`memory:held-${held[0].id}`)} ${bytes(held[0].bytes)}`,
                glyph: chip,
                note: says("memory:held", "note")
            }, () => held.map(part => figure({
                name: named(`memory:held-${part.id}`),
                side: bytes(part.bytes),
                graph: bar(part.bytes / m.total_b),
                note: says(`memory:held-${part.id}`, "note")
            })).concat(zswap ? [figure({
                name: named("memory:held-compressed"),
                side: says("memory:held-compressed", "side", { stored: bytes(zswap.stored),
                    pool: bytes(zswap.pool) }),
                graph: bar(zswap.pool / m.total_b),
                note: says("memory:held-compressed", "note", { ratio: number(zswap.ratio, 1) })
            })] : []).concat(info.AnonHugePages > 0 || (m.huge_pages?.HugePages_Total ?? 0) > 0
                ? [figure({
                    name: named("memory:held-huge"),
                    side: bytes((info.AnonHugePages ?? 0) + (info.Hugetlb ?? 0)),
                    note: says("memory:held-huge", "note", {
                        reserved: m.huge_pages?.HugePages_Total ?? 0 })
                })] : [])));
        // Its name alone on the row, the share beside it, and what's
        // committed of what may be in the note (VKCF3Z, 2026-11-07).
        if (info.CommitLimit > 0)
            rows.push(figure({
                name: named("memory:committed"),
                side: percent(info.Committed_AS / info.CommitLimit),
                graph: bar(info.Committed_AS / info.CommitLimit),
                note: says("memory:committed", "note", { committed: bytes(info.Committed_AS),
                    limit: bytes(info.CommitLimit) })
            }));
        if (typeof m.paged_in_b_per_s === "number")
            rows.push(figure({
                name: named("memory:paging"),
                side: both(m.paged_in_b_per_s, m.paged_out_b_per_s),
                note: says("memory:paging", "note")
            }));
        if (typeof m.faults_per_s === "number")
            rows.push(figure({
                name: named("memory:faults"),
                side: often(m.faults_per_s),
                note: m.major_faults_per_s > 0 ? says("memory:faults", "note", {
                    major: often(m.major_faults_per_s) }) : ""
            }));
        if (typeof m.swapped_in_b_per_s === "number")
            rows.push(figure({
                name: named("memory:swapping"),
                side: both(m.swapped_in_b_per_s, m.swapped_out_b_per_s),
                // What zswap took and gave back, said only while it moves.
                note: m.compressed_in_b_per_s > 0 || m.compressed_out_b_per_s > 0
                    ? says("memory:swapping", "compressed", {
                        both: both(m.compressed_in_b_per_s, m.compressed_out_b_per_s) })
                    : ""
            }));
        if (typeof m.oom_kills === "number")
            rows.push(figure({
                name: named("memory:out-of-memory"),
                side: `${m.oom_kills}`,
                graph: undefined,
                note: says("memory:out-of-memory", "note")
            }));
        const holders = using("memory");
        if (holders && holders.length > 0)
            rows.push(holderRow({
                name: named("memory:using"),
                side: `${holders[0].name} ${bytes(holders[0].resident_b)}`,
                glyph: chip,
                note: "each program's processes together"
            }, () => holders.map((each, at) => figure({
                key: ranked("memory", at),
                name: each.name,
                side: bytes(each.resident_b),
                graph: bar(each.resident_b / m.total_b),
                note: each.processes > 1 ? `${each.processes} processes` : ""
            }))));
        else if (!holders)
            rows.push(reading(named("memory:using")));
        const modules = Machine.modules(m.modules);
        if (modules.length > 0) {
            const first = modules[0];
            const type = first.speed !== "" ? `${first.type} at ${first.speed}` : first.type;
            const count = modules.reduce((all, one) => all + one.count, 0);
            const each = one => figure({
                name: one.count > 1 ? `${one.count} \u00d7 ${one.size || one.type}`
                    : one.size || one.type,
                side: one.speed !== "" ? `${one.type} at ${one.speed}` : one.type,
                note: Machine.items([one.manufacturer !== "" ? `by ${one.manufacturer}` : "",
                    one.part, one.locators.join(" \u00b7 ")])
            });
            rows.push(holderRow({
                name: named("memory:type"),
                side: type,
                copy: type,
                glyph: chip,
                note: Machine.items([count > 0 && first.size !== "" && modules.length === 1
                    ? `${count} ${count === 1 ? "module" : "modules"} of ${first.size}` : "",
                    first.manufacturer !== "" ? `by ${first.manufacturer}` : ""])
            }, () => modules.map(each)));
        }
        // Every count the kernel keeps of its memory, by its own name, so
        // none it gives is left unseen (#2204's bar): those in bytes, then
        // the huge pages it counts by number.
        const counted = Object.keys(info);
        if (counted.length > 0)
            rows.push(holderRow({
                name: named("memory:counts"),
                side: `${counted.length + Object.keys(m.huge_pages ?? {}).length}`,
                glyph: chip,
                note: says("memory:counts", "note")
            }, () => counted.map(name => figure({
                key: `machine:memory:count:${name}`,
                name: name,
                side: bytes(info[name])
            })).concat(Object.keys(m.huge_pages ?? {}).map(name => figure({
                key: `machine:memory:count:${name}`,
                name: name,
                side: `${m.huge_pages[name]}`
            })))));
        return ordered(kinds.memory, rows);
    }

    // The disk.

    // Each disk read's rates, for their sparks, the newest last: added once
    // for each read the daemon says of now.
    property var series: ({})

    function sparked(disk) {
        if (disk.since !== "now")
            return;
        const add = (name, value) => {
            series[name] = (series[name] ?? []).concat([value]).slice(-graphed);
        };
        add("read", disk.read_b_per_s ?? 0);
        add("write", disk.written_b_per_s ?? 0);
        add("moved", (disk.read_b_per_s ?? 0) + (disk.written_b_per_s ?? 0));
    }

    Connections {
        target: daemon

        function onHeard(keys) {
            if (provider.shown)
                provider.answered();
        }
    }

    Connections {
        target: diskWatch

        function onHeard(changed) {
            const own = provider.machineDisk();
            if (own && changed.includes(own.key))
                provider.sparked(own);
            if (provider.shown)
                provider.answered();
        }
    }

    Connections {
        target: users

        function onHeard(keys) {
            if (provider.shown)
                provider.answered();
        }
    }

    // The root file system's room, from the daemon, where no guard says it.
    function rootRoom(disk) {
        const all = disk ? disk.file_systems ?? [] : [];
        const root = all.find(fs => fs.mount === "/") ?? all[0] ?? null;
        return root && root.size_b > 0 ? root : null;
    }

    function disks() {
        const fact = Facts.all.find(fact => fact.entry.name === "disk");
        const guard = fact && fact.piece && fact.piece.present ? fact.piece : null;
        const disk = machineDisk();
        const room = guard === null ? rootRoom(disk) : null;
        const worst = guard ? guard.worst : null;
        const watching = worst === null ? "nothing watched"
            : `${guard.shown(worst.free)} free on ${worst.path}`;
        const headline = guard ? watching
            : room ? `${bytes(room.available_b)} free on ${room.mount}`
            : disk ? rate((disk.read_b_per_s ?? 0) + (disk.written_b_per_s ?? 0)) : "";
        if (headline === "")
            return unheld("disks");
        return fresh({
            headline: headline,
            graph: worst ? bar(1 - worst.free / worst.total, worst.band !== "healthy")
                : room ? bar(1 - room.available_b / room.size_b) : undefined,
            glyph: Words.glyph(kinds.disks),
            title: Words.name(kinds.disks),
            about: kinds.disks,
            // The guard's own words are each disk's, in its drawer (disked).
            note: guard ? guardSays(guard, worst) : disk ? since(disk) : ""
        }, () => diskParts(guard, disk, room));
    }

    // How the guard stands, in a phrase: the band the worst file system is
    // in and what its last pass did, such as "filling; nothing more is safe
    // to take", so Joel sees why the disk is still short.
    function guardSays(guard, worst) {
        if (guard.silent)
            return [guard.accountStopped ? "the disk guard stopped"
                : "the machine's disk guard stopped", guard.context]
                .filter(part => part !== "").join("; ");
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
        const titled = id => quests?.titleOf(id);
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
            return thingRow({
                thing: true,
                key: `machine:disk:holder:${holder.path}`,
                name: called(holder),
                side: guard.shown(holder.bytes),
                graph: disk ? bar(holder.bytes / disk.total) : undefined,
                note: holder.clearing ? said.clearing ?? ""
                    : holder.clearable && !holder.kept ? said.clears ?? "" : holder.why ?? "",
                reads: holder.says ?? ""
            }, () => holderParts(guard, holder, called(holder)));
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

    // What can be done to a holder, then what it reads, then Copy the Path
    // (VKCF3Z's design, the disk guard's acts): open its folder; keep it
    // from the guard's clearing, a switch that asks Joel why as it's
    // turned on, where the account's guard measured it, since only that
    // guard keeps; clear what the guard would take of it now, asking once,
    // since it can't be taken back; its size and what a clear would take.
    function holderParts(guard, holder, name) {
        const act = (id, fields) => (fields.copy ? copyRow : thing)(Object.assign({
            key: `verb:disk:${id}:${holder.path}`,
            name: Words.name(`verb:disk:${id}`),
            verb: Words.verb(`verb:disk:${id}`),
            glyph: Words.glyph(`verb:disk:${id}`),
            hue: "action"
        }, fields));
        const rows = [act("open", { run: () => Niri.spawn(["xdg-open", holder.path]) })];
        // Each session it belongs to, at work on its quest or in its
        // folder, the one that worked last first, told what Joel types and
        // which folder and how much it's about, so it can clear its own
        // (Joel: "talk to its session"); at its right its title and how it
        // stands, as Agents says them, and absent where none is, never
        // dimmed (VKCF3Z, 2026-11-08).
        const talk = Words.of("verb:disk:talk");
        const standing = Words.of("agents").said ?? {};
        // The folder as the guard names it, "~/src/reef/target", as Joel reads it too.
        const about = (talk.said?.about ?? "").replace("{path}", holder.name || holder.path)
            .replace("{size}", HomeNetwork.bytes(holder.bytes, settings));
        for (const session of Holders.owners(Sessions.running.concat(Sessions.parked),
            holder)) {
            const state = SessionRow.state(session.presence, Sessions.asks(session), false);
            const sided = Holders.talkSide(session.title, String(standing[state] ?? ""));
            rows.push(act("talk", {
                key: `verb:disk:talk:${holder.path}:${session.address}`,
                side: sided.map(item => item.said).join(" \u00b7 "),
                sideItems: sided,
                // The ask names who it's told to (VKCF3Z, 2026-11-08).
                ask: (talk.ask ?? "").replace("{name}", session.title),
                chip: session.title,
                state: () => provider.talked,
                given: words => {
                    const said = `${words.trim()} ${about}`;
                    if (words.trim() !== "" && Sessions.talk(session.address, said))
                        provider.talked = (talk.said?.sent ?? "")
                            .replace("{name}", session.title);
                }
            }));
        }
        if (holder.ours) {
            const kept = holder.kept ?? null;
            const keeping = {
                side: kept ? `yes, ${kept.reason}` : "no",
                stays: true,
                state: () => guard.told
            };
            rows.push(act("keep", kept
                ? Object.assign(keeping, {
                    verb: Words.verb("verb:disk:keep", "kept"),
                    run: () => guard.release(holder.path)
                })
                : Object.assign(keeping, {
                    ask: Words.of("verb:disk:keep").ask ?? "",
                    chip: name,
                    given: reason => guard.keep(holder.path, reason)
                })));
        }
        // Only what a clear takes now, never what waits to sit idle, and
        // never what Joel keeps; gone while the guard clears it, the
        // holder's note saying so.
        if (holder.ours && (holder.clears ?? 0) > 0 && !holder.kept && !holder.clearing) {
            const amount = guard.shown(holder.clears);
            const sure = () => [thing({
                key: `verb:disk:clear:sure:${holder.path}`,
                // The step that asks Joel to be sure, so its Return does it (Launcher.lands).
                sure: true,
                name: (Words.of("verb:disk:clear").sure ?? "").replace("{amount}", amount)
                    .replace("{name}", name),
                verb: Words.verb("verb:disk:clear"),
                glyph: Words.glyph("verb:disk:clear"),
                hue: "alert",
                stays: true,
                state: () => guard.told,
                run: () => guard.clear(holder.path)
            })];
            rows.push(act("clear", { side: amount, verbs: sure, targets: sure }));
        }
        rows.push(figure({
            key: `machine:disk:holder:size:${holder.path}`,
            name: named("disk:holder:size"),
            side: guard.shown(holder.bytes)
        }));
        // How much it grew over the last day, as the guard measured it, once
        // it has measured for an hour; what Joel looks at when something
        // fills the disk faster than it should.
        const grew = holder.grew ?? null;
        if (grew && grew.bytes > 0)
            rows.push(figure({
                key: `machine:disk:holder:grew:${holder.path}`,
                name: named("disk:holder:grew"),
                side: `${guard.shown(grew.bytes)} in ${Interfaces.lasted(grew.over)}`
            }));
        if ((holder.clears ?? 0) > 0)
            rows.push(figure({
                key: `machine:disk:holder:clears:${holder.path}`,
                name: named("disk:holder:clears"),
                side: guard.shown(holder.clears)
            }));
        // An act like any other, though copying is its Return, so it wears
        // its glyph as acts do rather than standing as a figure.
        rows.push(act("copy-path", { copy: holder.path, figure: false, plain: false }));
        return rows;
    }

    // A disk is a device, so a thing (VKCF3Z, 2026-10-29): beside it its
    // room and, while it's short, what the guard is doing about it, so
    // that's seen without selecting it; in the drawer, while it's short,
    // its figures and the guard's own sentence, and its name alone while
    // it's healthy, as the sentence then only says the room again. Its
    // room is beside it, so the drawer doesn't say Free again, as Now's
    // disk drawer doesn't (VKCF3Z, 2026-11-08); its Size and Full In stay.
    function disked(guard, disk, fields) {
        const free = `${guard.shown(disk.free)} free`;
        if (disk.band === "healthy")
            return figure(Object.assign({ thing: true, key: `machine:disk:${disk.path}`,
                side: free }, fields));
        const said = Words.of("machine:disk:space").said ?? {};
        const doing = disk.exhausted ? said.cleared
            : disk.band === "filling" ? said.clearing : said.clearingAll;
        const full = typeof disk.full_at === "number" ? disk.full_at - Date.now() / 1000 : 0;
        const facts = [
            { label: named("disk:size"), value: guard.shown(disk.total) },
            full > 0 ? { label: named("disk:full-in"), value: guard.duration(full) } : null
        ];
        return figure(Object.assign({
            thing: true,
            key: `machine:disk:${disk.path}`,
            side: `${free} \u00b7 ${doing ?? guard.banded(disk.band)}`,
            drawer: { title: fields.name, facts: facts.filter(fact => fact !== null),
                body: disk.says ?? "" }
        }, fields));
    }

    function diskParts(guard, disk, room) {
        const chip = Words.glyph(kinds.disks);
        const rows = [];
        if (guard && guard.disks.length > 0) {
            // Disks are amounts: the worst band first, as the disk piece
            // judges it, so the first is the headline's, and within a band
            // the least room, then A to Z.
            const watched = Order.sorted(guard.disks, Order.amounts({
                worse: (a, b) => guard.worse(a, b) ? -1 : guard.worse(b, a) ? 1 : a.free - b.free,
                name: one => one.path
            }));
            const each = one => disked(guard, one, {
                name: one.path,
                graph: bar(1 - one.free / one.total, one.band !== "healthy")
            });
            const worst = watched[0];
            const space = {
                name: named("disk:space"),
                graph: bar(1 - worst.free / worst.total, worst.band !== "healthy"),
                note: watched.length > 1 ? `on ${worst.path}, the least of ${watched.length}`
                    : `on ${worst.path}`
            };
            rows.push(watched.length > 1
                ? holderRow(Object.assign(space, {
                    glyph: chip,
                    side: `${guard.shown(worst.free)} free`
                }), () => watched.map(each))
                : disked(guard, worst, space));
            if (guard.holders.length > 0)
                rows.push(filling(guard, chip));
        } else if (room) {
            rows.push(figure({
                name: named("disk:space"),
                side: `${bytes(room.available_b)} free`,
                graph: bar(1 - room.available_b / room.size_b),
                note: `on ${room.mount}, of ${bytes(room.size_b)}`
            }));
        }
        if (!disk)
            return rows;
        const when = since(disk);
        const floor = 1e6;
        // Each kind's requests and those merged into others, each a
        // second, a part at zero giving way (VKCF3Z, 2026-11-07).
        const count = n => number(n, n < 10 ? 1 : 0);
        const iops = (requests, merged) => Machine.items([requests > 0
            ? says("disk:speed", "iops", { n: count(requests) }) : "",
            merged > 0 ? says("disk:speed", "merged", { n: count(merged) }) : ""]);
        rows.push(holderRow({
            name: named("disk:speed"),
            side: rate((disk.read_b_per_s ?? 0) + (disk.written_b_per_s ?? 0)),
            graph: spark(series.moved ?? [], floor),
            glyph: chip,
            note: when
        }, () => [figure({
            name: named("disk:reading"),
            side: rate(disk.read_b_per_s ?? 0),
            graph: spark(series.read ?? [], floor),
            note: iops(disk.reads_per_s, disk.read_merges_per_s)
        }), figure({
            name: named("disk:writing"),
            side: rate(disk.written_b_per_s ?? 0),
            graph: spark(series.write ?? [], floor),
            note: iops(disk.writes_per_s, disk.write_merges_per_s)
        }), figure({
            name: named("disk:discarding"),
            side: rate(disk.discarded_b_per_s ?? 0),
            note: iops(disk.discards_per_s, 0)
        }), figure({
            name: named("disk:flushing"),
            side: often(disk.flushes_per_s ?? 0),
            note: says("disk:flushing", "note")
        })]));
        if (typeof disk.busy_ratio === "number")
            rows.push(figure({
                name: named("disk:busy"),
                side: percent(disk.busy_ratio),
                graph: bar(disk.busy_ratio),
                note: when
            }));
        const latencies = [["reading", disk.read_latency_ms], ["writing", disk.write_latency_ms],
            ["discarding", disk.discard_latency_ms], ["flushing", disk.flush_latency_ms]];
        rows.push(holderRow({
            name: named("disk:latency"),
            side: typeof disk.latency_ms === "number" ? ms(disk.latency_ms) : "idle",
            glyph: chip,
            note: typeof disk.latency_ms === "number" ? "each request, on average" : ""
        }, () => latencies.map(([id, value]) => figure({
            name: named(`disk:latency-${id}`),
            side: typeof value === "number" ? ms(value) : "idle",
            note: typeof value === "number" ? when : says("disk:latency", "idle")
        }))));
        if (typeof disk.in_flight === "number")
            rows.push(figure({
                name: named("disk:queue"),
                side: `${disk.in_flight}`,
                note: Machine.items([disk.queued > 0 ? says("disk:queue", "queued", {
                    queued: Numbers.significant(disk.queued, settings.group,
                        settings.point) }) : "",
                    disk.queue_depth > 0 ? says("disk:queue", "depth", {
                        depth: disk.queue_depth }) : ""])
            }));
        if (disk.pressure?.some)
            rows.push(pressured("disk:pressure", disk.pressure, true));
        const sensors = disk.temperatures ?? [];
        const composite = sensors.find(sensor => sensor.label === "Composite") ?? sensors[0];
        if (composite && typeof composite.temperature_c === "number")
            rows.push(heated("disk:temperature", composite, "disk"));
        const smart = disk.health;
        if (smart && typeof smart.percent_used === "number")
            rows.push(healthRow(smart, chip));
        // Each file system on it, by where it's mounted, with its room
        // and its files, as the kernel says them.
        const systems = disk.file_systems ?? [];
        if (systems.length > 0)
            rows.push(holderRow({
                name: named("disk:file-systems"),
                side: `${systems.length}`,
                glyph: chip,
                note: Machine.items(systems.map(fs => fs.mount))
            }, () => systems.map(fs => figure({
                key: `machine:disk:file-system:${fs.mount}`,
                name: fs.mount,
                side: fs.size_b > 0 ? says("disk:file-systems", "room", {
                    free: bytes(fs.available_b), size: bytes(fs.size_b) }) : fs.type,
                graph: fs.size_b > 0 ? bar(1 - fs.available_b / fs.size_b) : undefined,
                note: Machine.items([fs.type, fs.free_b > fs.available_b
                    ? says("disk:file-systems", "reserved", {
                        reserved: bytes(fs.free_b - fs.available_b) }) : "",
                    fs.files > 0 ? says("disk:file-systems", "files", {
                        free: number(fs.files_free, 0), files: number(fs.files, 0) }) : "",
                    fs.device])
            }))));
        if (disk.model)
            rows.push(holderRow({
                name: named("disk:model"),
                side: disk.model,
                copy: disk.model,
                glyph: chip
            }, () => diskModelParts(disk)));
        return ordered(kinds.disks, rows);
    }

    // The disk's health, as its controller tells UDisks.
    function healthRow(smart, chip) {
        const worn = smart.percent_used / 100;
        const warned = Machine.warnings(smart.critical_warning);
        const failing = smart.avail_spare < smart.spare_thresh || smart.media_errors > 0
            || warned.length > 0;
        return holderRow({
            name: named("disk:health"),
            side: `${smart.percent_used}% worn`,
            graph: bar(worn, failing),
            glyph: chip,
            note: failing ? "its controller says it's failing"
                : typeof smart.updated_s === "number" ? says("disk:health", "asked", {
                    at: Qt.formatTime(new Date(smart.updated_s * 1000), "HH:mm") })
                : "as its controller says"
        }, () => {
            const rows = [figure({
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
            })];
            if (typeof smart.power_on_hours === "number")
                rows.push(figure({
                    name: named("disk:powered"),
                    side: lasted(smart.power_on_hours * 3600),
                    note: says("disk:powered", "note", { cycles: smart.power_cycles ?? 0 })
                }));
            if (typeof smart.ctrl_busy_time === "number")
                rows.push(figure({
                    name: named("disk:busy-for"),
                    side: lasted(smart.ctrl_busy_time * 60),
                    note: says("disk:busy-for", "note")
                }));
            rows.push(figure({
                name: named("disk:errors"),
                side: `${smart.media_errors}`,
                note: "the disk couldn't read or write"
            }));
            if (typeof smart.num_err_log_entries === "number")
                rows.push(figure({
                    name: named("disk:error-log"),
                    side: `${smart.num_err_log_entries}`,
                    note: says("disk:error-log", "note")
                }));
            rows.push(figure({
                name: named("disk:shutdowns"),
                side: `${smart.unsafe_shutdowns}`,
                note: `of ${smart.power_cycles} times it was powered`
            }));
            if (typeof smart.warning_temp_time === "number")
                rows.push(figure({
                    name: named("disk:too-hot"),
                    side: lasted((smart.warning_temp_time + (smart.critical_temp_time ?? 0)) * 60),
                    // Its thresholds are in kelvin, as NVMe counts them.
                    note: Machine.items([smart.wctemp ? says("disk:too-hot", "warning", {
                        warning: degrees(smart.wctemp - 273.15) }) : "",
                        smart.cctemp && smart.critical_temp_time > 0
                            ? says("disk:too-hot", "critical", {
                                critical: degrees(smart.cctemp - 273.15),
                                lasted: lasted(smart.critical_temp_time * 60) }) : ""])
                }));
            rows.push(figure({
                name: named("disk:warnings"),
                side: warned.length > 0
                    ? Machine.items(warned.map(id => says("disk:warnings", id) || id))
                    : says("disk:warnings", "none"),
                note: says("disk:warnings", "note")
            }));
            return rows;
        });
    }

    // What the disk is: its firmware, the link it's on, its size, its
    // blocks and how its requests are ordered.
    function diskModelParts(disk) {
        const rows = [leaf({ name: named("disk:model-name"), side: disk.model })];
        if (disk.firmware)
            rows.push(leaf({
                name: named("disk:firmware"),
                side: disk.firmware,
                note: disk.health?.nvme_revision ? says("disk:firmware", "nvme", {
                    revision: disk.health.nvme_revision }) : ""
            }));
        if (typeof disk.rotational === "boolean")
            rows.push(figure({
                name: named("disk:kind"),
                side: says("disk:kind", disk.rotational ? "spinning" : "solid"),
                note: Machine.items([disk.transport ?? "", disk.removable
                    ? says("disk:kind", "removable") : says("disk:kind", "fixed")])
            }));
        if (disk.link)
            rows.push(figure({
                name: named("disk:link"),
                side: `${disk.link.speed} \u00d7${disk.link.width}`,
                note: disk.link.max_speed && (disk.link.max_speed !== disk.link.speed
                    || disk.link.max_width !== disk.link.width)
                    ? says("disk:link", "slower", { most: `${disk.link.max_speed} \u00d7${
                        disk.link.max_width}` }) : says("disk:link", "full")
            }));
        if (disk.size_b)
            rows.push(figure({ name: named("disk:capacity"), side: bytes(disk.size_b) }));
        if (disk.logical_block_b)
            rows.push(figure({
                name: named("disk:block"),
                side: disk.physical_block_b && disk.physical_block_b !== disk.logical_block_b
                    ? says("disk:block", "both", { logical: disk.logical_block_b,
                        physical: disk.physical_block_b })
                    : `${disk.logical_block_b} B`
            }));
        if (disk.scheduler)
            rows.push(figure({
                name: named("disk:scheduler"),
                side: disk.scheduler,
                note: (disk.schedulers ?? []).length > 1
                    ? says("disk:scheduler", "also", { all: Machine.items(disk.schedulers
                        .filter(one => one !== disk.scheduler)) }) : ""
            }));
        return rows;
    }

    // The boot.

    function bootRecord() {
        const held = daemon.revision >= 0 ? daemon.held : {};
        const key = Object.keys(held).find(each => each.startsWith("boot:"));
        return key ? held[key] : null;
    }

    // How long it's been up, from when it started, so it moves as the
    // clock does, though the record changes only when the machine wakes.
    function uptime() {
        const boot = bootRecord();
        if (!boot || typeof boot.booted_at_s !== "number")
            return unheld("uptime");
        const seconds = Math.max(0, Date.now() / 1000 - boot.booted_at_s);
        const days = Math.floor(seconds / 86400);
        const hours = Math.floor(seconds % 86400 / 3600);
        const minutes = Math.floor(seconds % 3600 / 60);
        const said = [];
        if (days > 0)
            said.push(`${days} ${days === 1 ? "day" : "days"}`);
        if (hours > 0)
            said.push(`${hours} ${hours === 1 ? "hour" : "hours"}`);
        if (days === 0)
            said.push(`${minutes} ${minutes === 1 ? "minute" : "minutes"}`);
        return fresh({
            headline: `up ${said.join(", ")}`,
            glyph: Words.glyph(kinds.cpu),
            title: Words.name(kinds.uptime),
            note: ""
        }, () => bootParts(boot, seconds));
    }

    // Idle first, as what moves most; Asleep after, since it moves least
    // (VKCF3Z, 2026-11-07); then when it started and its kernel.
    function bootParts(boot, seconds) {
        const rows = [];
        if (typeof boot.idle_ratio === "number")
            rows.push(figure({
                name: named("uptime:idle"),
                side: percent(boot.idle_ratio),
                graph: bar(boot.idle_ratio),
                note: says("uptime:idle", "note")
            }));
        if (typeof boot.asleep_s === "number")
            rows.push(figure({
                name: named("uptime:asleep"),
                side: boot.asleep_s > 0 ? lasted(boot.asleep_s) : says("uptime:asleep", "never"),
                graph: seconds > 0 ? bar(boot.asleep_s / seconds) : undefined,
                note: seconds > 0 && boot.asleep_s > 0 ? says("uptime:asleep", "note", {
                    share: percent(boot.asleep_s / seconds) }) : ""
            }));
        rows.push(leaf({
            name: named("uptime:started"),
            side: Qt.formatDateTime(new Date(boot.booted_at_s * 1000), "ddd d MMM, HH:mm")
        }));
        // Its build is too long for a note, so it's a step in.
        if (boot.kernel)
            rows.push(boot.kernel_build ? holderRow({
                name: named("uptime:kernel"),
                side: boot.kernel,
                copy: boot.kernel,
                glyph: Words.glyph(kinds.cpu)
            }, () => [leaf({ name: named("uptime:release"), side: boot.kernel }),
                leaf({ name: named("uptime:build"), side: boot.kernel_build })])
                : leaf({ name: named("uptime:kernel"), side: boot.kernel }));
        return rows;
    }

    // While one of its answers is shown, as the answer to what's typed, a
    // row in a category's list or a step held, it's asked again as the
    // daemon's records change. A place lists an answer only once it has
    // parts, which come from the records, so a place holding one of
    // Machine's whole answers (the launcher's `placed`, from its words
    // alone) watches too, or Machine's own place would wait for a read
    // nothing asks. Kept From Resting, which Power holds, is read from
    // the telemetry rather than the daemon, so it isn't one of them.
    // A terminal's `launcher show` sees them too, with the launcher closed,
    // while the rows it walked through or listed are Machine's
    // (Launcher.qml's terminalSees), so a place asked cold is read.
    readonly property bool shown: launcher !== null && (launcher.open
        && (launcher.answer?.live === provider
            || launcher.held.some(step => step.row.live === provider)
            || launcher.matches.some(row => row.live === provider)
            || launcher.placed.some(id => /^machine:[a-z]+$/.test(id)))
        || terminalWalked.concat(terminalListed).some(row => row?.live === provider))
    readonly property var terminalWalked: launcher?.terminalSees.walked ?? []
    readonly property var terminalListed: launcher?.terminalSees.listed ?? []

    // Whether a daemon it watches hasn't yet answered this watch whole, so
    // what it shows is the placeholder, or what an earlier watch left,
    // still being read (Provider.stillReading).
    stillReading: [daemon, diskWatch, users].some(watch => watch.live
        && !watch.answeredThisWatch && watch.state !== "not answering")

    // What the open launcher holds, answers and lists is left as it was
    // once it closes, and set afresh only as it opens, so with it closed
    // only a terminal's rows count (Y2YBTS's review of #2433: Processor
    // left held would read every process each second for a terminal).
    readonly property bool launcherOpen: launcher?.open === true

    // Whether a Using row can be seen, so the daemon is asked for the
    // programs: the processor's or the memory's parts stepped into, or
    // Using asked for alone. Building parts to find one, or to search
    // them, asks for nothing, since every process's file is read for it
    // each second (Y2YBTS's review of #2276).
    readonly property var usingNames: [named("cpu:using"), named("memory:using")]
    readonly property bool usingWanted: shown && (launcherOpen && (launcher.held.some(step =>
        step.row?.live === provider
            && (step.row.about === kinds.cpu || step.row.about === kinds.memory))
        || launcher.answer?.live === provider && usingNames.includes(launcher.answer.title))
        || terminalWalked.some(row => row?.live === provider
            && (row.about === kinds.cpu || row.about === kinds.memory)))

    // Whether a Disk answer or row is shown, listed or stepped into, so the
    // daemon reads the disks: each read asks an NVMe disk its temperature,
    // a command to the drive, which nothing else Machine shows needs.
    readonly property bool disksWanted: shown && (launcherOpen && ([launcher.answer,
        ...launcher.held.map(step => step.row), ...launcher.matches]
        .some(row => row?.live === provider && row.about === kinds.disks)
        || launcher.placed.includes("machine:disk"))
        || terminalWalked.concat(terminalListed)
            .some(row => row?.live === provider && row.about === kinds.disks))

    // The processor's telemetry is watched while a Processor answer is
    // listed, shown or stepped into, so what held it back is there once its
    // parts are opened, and it's read for nothing else Machine answers.
    // Kept From Resting is asked alone too, and a place may hold it, as
    // Power does on battery: what the place holds now is the launcher's
    // `placed`, read from its words and the battery alone, so reading it
    // here can't loop back through an answer.
    readonly property bool watched: shown && launcherOpen && ([launcher.answer,
        ...launcher.held.map(step => step.row), ...launcher.matches]
        .some(row => row?.live === provider && row.about === kinds.cpu)
        || launcher.placed.includes("machine:cpu:kept-from-resting"))
    onWatchedChanged: ProcessorTelemetry.watching += watched ? 1 : -1

    // The uptime moves with the clock, a minute at a time, while shown.
    Timer {
        interval: 60000
        repeat: true
        running: provider.shown
        onTriggered: provider.answered()
    }

    // The machine daemon: the processor, memory and boot while one of
    // Machine's answers is shown, the disks while a Disk answer or row is,
    // and the programs while a Using row can be, in the open launcher or
    // a terminal's show. Nothing is read while neither sees them; a first
    // look holds the placeholder for the few milliseconds the daemon's
    // first read takes.
    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: ["processor", "memory", "boot"]
        live: provider.shown
        paced: provider.launcher
        keptThroughGap: false
        answersWhole: true
    }

    Daemon {
        id: diskWatch

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "disk"
        live: provider.disksWanted
        paced: provider.launcher
        keptThroughGap: false
        answersWhole: true
    }

    Daemon {
        id: users

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "using"
        live: provider.shown && provider.usingWanted
        paced: provider.launcher
        keptThroughGap: false
        answersWhole: true
    }
}
