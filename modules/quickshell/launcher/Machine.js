// What the launcher's machine answers make of the machine daemon's records
// (modules/machine), in plain JavaScript so a check can run it in Qt's own
// engine, as the shell does: which cores are maxed, where the processor's
// time went, what a disk's controller warns of, which disk is the
// machine's, what held the processor back and what kept it from resting
// as its own telemetry says, the processor's own sensor among its chip's,
// and a row the daemon serves ready, as Machine.qml draws it. Machine.qml
// says the figures.
.pragma library
.import "Numbers.js" as Numbers

// A measure keeps three significant digits, its trailing zeros too, by
// the launcher's one rule (Numbers.significant), and takes the next unit
// past 999 (docs/design.md, Set, not styled): "915 MB", "1.52 GB", "4.70
// GHz". Every amount of bytes, memory's and a
// disk's alike, is in the decimal units the files and Programs already
// use, converted from bytes, so one page never mixes two families
// (VKCF3Z, 2026-11-07).
const STEP = 1000;
// Past this a figure rounds to four digits, so it takes the next unit.
const ROUNDS_UP = 999.5;
const BYTES = ["B", "kB", "MB", "GB", "TB", "PB"];

// A figure in the units it's counted in, stepping a thousand at a time
// from the first: bytes in whole bytes below a kilobyte.
function measure(value, units) {
    let scaled = value;
    let at = 0;
    while (Math.abs(scaled) >= ROUNDS_UP && at < units.length - 1) {
        scaled /= STEP;
        at += 1;
    }
    // Below ROUNDS_UP a figure rounds to three digits at most, so it never
    // reads a thousand of its unit: 999.7 kB has stepped to "1.00 MB".
    const said = at === 0 && units === BYTES ? String(Math.round(scaled))
        : Numbers.significant(scaled);
    return `${said} ${units[at]}`;
}

function bytes(count) {
    return measure(count, BYTES);
}

function rate(count) {
    return `${bytes(count)}/s`;
}

// A clock, from the kilohertz the kernel counts it in.
function hertz(khz) {
    return measure(khz * STEP, ["Hz", "kHz", "MHz", "GHz"]);
}

// Two rates that run opposite ways, each saying which it is, or "none"
// while both are still (VKCF3Z, 2026-11-07).
function both(inward, outward) {
    if (!inward && !outward) {
        return "none";
    }
    return `${rate(inward ?? 0)} in \u00b7 ${rate(outward ?? 0)} out`;
}

// A note's items, joined with dots, those empty or at zero given way.
function items(parts) {
    return parts.filter(part => part !== "" && part !== null && part !== undefined)
        .join(" \u00b7 ");
}

// The ranges a CPU list is written in: "0-3,8".
const RANGE = /^(\d+)(?:-(\d+))?$/;

// A core this busy or busier is maxed.
const MAXED = 0.95;
// Past this many maxed cores they're counted, not named.
const NAMED = 4;

// The maxed cores, as a row says them: none, by number while they're few,
// or how many of how many. Each core is { n, load }.
function maxed(loads) {
    const flat = loads.filter(core => core.load >= MAXED).map(core => core.n);
    if (flat.length === 0) {
        return "none";
    }
    if (flat.length > NAMED) {
        return `${flat.length} of ${loads.length}`;
    }
    if (flat.length === 1) {
        return `core ${flat[0]}`;
    }
    return `cores ${flat.slice(0, -1).join(", ")} and ${flat.slice(-1)[0]}`;
}

// The CPU numbers a list such as /sys/devices/cpu_core/cpus names.
function cpus(text) {
    const listed = [];
    const ranges = text.trim().split(",").map(part => RANGE.exec(part.trim()))
        .filter(range => range !== null);
    for (const range of ranges) {
        const last = Number(range[2] ?? range[1]);
        for (let n = Number(range[1]); n <= last; n += 1) {
            listed.push(n);
        }
    }
    return listed;
}

// CPU numbers as a note says them: "core 4", "cores 0 to 3", or each
// named where they don't run unbroken, "cores 0, 2 and 5".
function cores(numbers) {
    if (numbers.length === 0) {
        return "";
    }
    if (numbers.length === 1) {
        return `core ${numbers[0]}`;
    }
    const last = numbers[numbers.length - 1];
    if (last - numbers[0] === numbers.length - 1) {
        return `cores ${numbers[0]} to ${last}`;
    }
    return `cores ${numbers.slice(0, -1).join(", ")} and ${last}`;
}

// Where the processor's time went, as Time Spent's parts are listed, each
// by its part's id and the kernel's fields it sums: { id, share }, for
// each field the kernel keeps. Guests' time is in the user's already, and
// said beside it as what of it ran a virtual machine. Time stolen is only
// ever taken by a machine this one runs inside, so on one that runs on
// its own hardware (`guest` false) it's always none, and left out
// (modules/machine/README.md says so).
const SPENT = [
    ["user", ["user"]],
    ["system", ["system"]],
    ["iowait", ["iowait"]],
    ["interrupts", ["irq", "softirq"]],
    ["niced", ["nice"]],
    ["stolen", ["steal"]],
    ["guests", ["guest", "guest_nice"]]
];

function spent(time, guest) {
    if (!time) {
        return [];
    }
    return SPENT.filter(([id]) => guest || id !== "stolen")
        .filter(([, fields]) => fields.some(field => typeof time[field] === "number"))
        .map(([id, fields]) => ({
            id,
            share: fields.reduce((sum, field) => sum + (time[field] ?? 0), 0)
        }));
}

// A served row's side as the launcher says it (ready-rows.md's Times),
// since nothing that reads the clock is worded by a daemon: each item's
// words, and a time by its form, `lasted` as `told.lasted` says the
// seconds counted up from it and any other as `told.moment` says when,
// set into the item's own words where it has some ("up {when}").
function sideSaid(side, now, told) {
    const said = item => {
        if (item?.at === undefined) {
            return item?.said ?? "";
        }
        const when = item.as === "lasted" ? told.lasted(Math.max(0, now - item.at) / 1000)
            : told.moment(item.at, now);
        return String(item.said ?? "{when}").replace("{when}", when);
    };
    return (side ?? []).map(said).filter(part => part !== "").join(" \u00b7 ");
}

// What a row the daemon serves ready (the crate's wrapped form, its
// fields under `row`) is drawn with: the key the surfaces know it by,
// its name, side and graph as served, what Copy copies, its glyph's
// theme role and its hue. It's Machine.qml's until the launcher reads
// served rows itself (ReadyRows.qml).
function served(record, now, told) {
    const row = record?.row ?? {};
    const copy = (row.acts ?? []).find(act => act.key === "verb:copy")?.copy;
    return {
        key: row.thing ?? row.row ?? "",
        name: row.name ?? "",
        side: sideSaid(row.side, now, told),
        graph: row.graph,
        note: row.note ?? "",
        copy: copy ?? "",
        glyph: row.glyph ?? "",
        hue: row.hue ?? "machine"
    };
}

// What an NVMe disk's controller warns of, by the bits of its critical
// warning (NVMe base specification, SMART / Health Information): each
// warning's id, which the vocabulary words. UDisks says them by name.
const WARNINGS = {
    spare: "spare",
    temperature: "temperature",
    degraded: "degraded",
    readonly: "read-only",
    volatile_mem: "volatile-memory",
    pmr_readonly: "persistent-memory-read-only"
};

function warnings(critical) {
    if (!Array.isArray(critical)) {
        return [];
    }
    return critical.map(bit => WARNINGS[bit] ?? String(bit));
}

// The NPU's clock, as its driver counts it in megahertz: "at 1.40 GHz of
// 2.05 GHz", or "resting" while it's powered down.
const MEGAHERTZ = 1000;

function npuClock(npu) {
    return npu.clock_mhz > 0
        ? `at ${hertz(npu.clock_mhz * MEGAHERTZ)} of ${hertz(npu.max_mhz * MEGAHERTZ)}`
        : "resting";
}

// The machine's disk, of the disk records: the biggest, as a laptop's one
// disk is.
function biggest(disks) {
    return disks.reduce((most, disk) => ((disk.size_b ?? 0) > (most?.size_b ?? -1) ? disk : most),
        null);
}

// What held a processor back, as a note says it after "by": the whole
// processor's limits by the names Intel gives them, and a core cluster's
// by the kind of limit it was.
const LIMITS = {
    pl1: "its power limit, PL1",
    pl2: "its short power limit, PL2",
    pl3: "its peak power limit, PL3",
    platformPl1: "the whole machine's power limit, PL1",
    platformPl2: "the whole machine's short power limit, PL2",
    platformPl3: "the whole machine's peak power limit, PL3",
    prochot: "being told to slow down",
    regulatorHot: "a regulator running hot",
    heat: "heat",
    power: "power",
    current: "current",
    other: "something else"
};

function limit(by) {
    return LIMITS[by] ?? by;
}

function strongest(causes) {
    return (causes ?? []).reduce((a, b) => (a && a.share >= b.share ? a : b), null);
}

// What held the processor back over the last second, from its telemetry's
// reading (ProcessorTelemetry.qml), or null while nothing did: the share
// of that second and the limit that held it most, the whole processor's
// where one did and else its most held cluster's, and each core held, by
// its number, with what held its cluster most, as { n, share, by }. A
// limit under a twentieth isn't said by the reader, so a core missing
// wasn't held.
function heldBack(reading) {
    if (!reading) {
        return null;
    }
    const held = [];
    for (const cluster of reading.clusters ?? []) {
        const most = strongest(cluster.heldBack);
        if (most) {
            for (const n of cpus(cluster.cpus)) {
                held.push({ n, share: most.share, by: most.by });
            }
        }
    }
    held.sort((a, b) => a.n - b.n);
    const whole = strongest(reading.heldBack);
    const most = whole ?? strongest(held);
    return most ? { share: most.share, by: most.by, whole: whole !== null, cores: held } : null;
}

// Each part the processor's telemetry says kept the package from resting,
// by the reader's name for it: its own word's id, under the part's row,
// and how a note says it.
const KEEPERS = {
    npu: { id: "npu", said: "the NPU" },
    imageProcessor: { id: "image-processor", said: "the image processor" },
    media: { id: "media-engine", said: "the media engine" },
    graphics: { id: "graphics", said: "the graphics" },
    display: { id: "display-engine", said: "the display engine" },
    chipset: { id: "chipset", said: "the chipset's devices" },
    cores: { id: "cores", said: "busy cores" },
    internal: { id: "power-controller", said: "the processor's own power controller" },
    other: { id: "other", said: "something else" }
};

function keeper(by) {
    return KEEPERS[by] ?? { id: "other", said: by };
}

// The least share the reader says a part kept the package from resting
// for, a twentieth, and so the least a part is named for.
const RESTLESS = 0.05;

// What kept the package from resting over its last readings, oldest first,
// each { kept, by } as the reader says a second (ProcessorTelemetry.qml
// keeps about ten): the share of that time anything did, on average, and
// each part's share of it, longest first, leaving out those under a
// twentieth, and whether it's under a twentieth as a whole, when nothing
// is said to keep it from resting. Averaged, a share near a twentieth
// doesn't jump each second. Null with no readings.
function kept(readings) {
    if (readings.length === 0) {
        return null;
    }
    const share = readings.reduce((sum, each) => sum + each.kept, 0) / readings.length;
    const parts = {};
    for (const reading of readings) {
        for (const part of reading.by) {
            parts[part.by] = (parts[part.by] ?? 0) + part.share;
        }
    }
    const by = Object.keys(parts)
        .map(part => ({ by: part, share: parts[part] / readings.length }))
        .filter(part => part.share >= RESTLESS)
        .sort((a, b) => b.share - a.share);
    return { share, by, quiet: share < RESTLESS };
}

// A spark's values, from 0 to 1 against the most of them or `least`,
// whichever is more, so a disk all but idle draws as all but empty.
function spark(values, least) {
    const most = Math.max(least, ...values);
    return values.map(value => (most > 0 ? value / most : 0));
}

// The processor's own temperature among its chip's sensors, as Heat says
// it: the package's or the die's, else its hottest core's, else its first.
const WHOLE = /^(Package|Tctl|Tdie)/;
const CORE = /^Core/;

function hottestCore(temps) {
    return temps.filter(sensor => CORE.test(sensor.label))
        .reduce((hottest, sensor) => (hottest && hottest.value >= sensor.value ? hottest : sensor),
            null);
}

// Whether a sensor's label names the whole processor, so only that one of
// its chip's sensors need be read: the laptop's chip has seventeen.
function namesWhole(label) {
    return WHOLE.test(label);
}

function wholeProcessor(temps) {
    return temps.find(sensor => WHOLE.test(sensor.label)) ?? null;
}

function processor(temps) {
    return wholeProcessor(temps) ?? hottestCore(temps) ?? temps[0] ?? null;
}
