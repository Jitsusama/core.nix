// What the launcher's machine answers make of the kernel's files, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// how busy the processors and each core were between two reads, which kind
// each core is, how fast a disk read and wrote between two reads, which
// programs use the processors and the memory, what held the processor
// back and what kept it from resting as its own telemetry says, and the
// memory modules the machine read at boot. Machine.qml reads the files
// and says the figures.
.pragma library

// A line of /proc/stat counting a processor's time: "cpu" for them all,
// "cpu3" for one.
const CPU_LINE = /^cpu(\d*)\s/;
const SPACES = /\s+/;
// A process's line of /proc/<pid>/stat: its id and its name in brackets.
const PROCESS_LINE = /^(\d+) \(/;
// The ranges a CPU list is written in: "0-3,8".
const RANGE = /^(\d+)(?:-(\d+))?$/;
// Memory pressure: the share of the last ten seconds something waited.
const SOME_PRESSURE = /^some avg10=([\d.]+)/m;
// A line of /proc/meminfo.
const MEMINFO_LINE = /^(\w+):\s+(\d+)/;
// The marks a processor's maker puts in its name, and the clock some add.
const MARKS = /\((R|TM)\)|\bCPU\b|@.*$/gi;

// The fields of a processor's line that count time spent, from user to
// steal; guest time is counted in user time already.
const COUNTED = 8;
const IDLE = 3;
const WAITING_ON_IO = 4;
// The fields of a process's stat after its name: its user and system
// time in ticks, and its resident pages. Its line starts with its id, in
// digits.
const USER_TIME = 11;
const SYSTEM_TIME = 12;
const RESIDENT = 21;
const DIGIT_ZERO = 48;
const DIGIT_NINE = 57;
// The fields of a disk's stat: reads and writes done, sectors read and
// written, the milliseconds each spent, and the milliseconds it was busy.
const READS = 0;
const SECTORS_READ = 2;
const READING_MS = 3;
const WRITES = 4;
const SECTORS_WRITTEN = 6;
const WRITING_MS = 7;
const BUSY_MS = 9;
// The kernel counts a disk's sectors in 512 bytes, whatever the disk's own.
const SECTOR = 512;
const MILLISECONDS = 1000;
const PERCENT = 100;

// A core this busy or busier is maxed.
const MAXED = 0.95;
// Past this many maxed cores they're counted, not named.
const NAMED = 4;
// How many programs are named as using the machine: as many as the
// launcher's card shows at once, so the list fills it without scrolling.
// Three left Joel asking what came after (2026-10-25, "why does memory
// only show the top 3 using it?").
const TOP = 8;
// The longest a process's name is kept by the kernel, after which it's cut.
const NAME_KEPT = 15;
const WRAPPED = "-wrapped";

// What each processor's counters say, the whole first and then each core
// by its number: the time it's been busy, and all its time.
function counters(text) {
    const cores = [];
    let whole = null;
    for (const line of text.split("\n").filter(each => CPU_LINE.test(each))) {
        const named = CPU_LINE.exec(line);
        const fields = line.trim().split(SPACES).slice(1, 1 + COUNTED).map(Number);
        const all = fields.reduce((sum, field) => sum + field, 0);
        const idle = fields[IDLE] + (fields[WAITING_ON_IO] ?? 0);
        const counted = { busy: all - idle, all };
        if (named[1] === "") {
            whole = counted;
        } else {
            cores.push(Object.assign({ n: Number(named[1]) }, counted));
        }
    }
    cores.sort((a, b) => a.n - b.n);
    return { whole, cores };
}

// How busy a processor was between two reads of its counters, from 0 to 1;
// with no read before, since the machine started.
function busy(now, then) {
    const all = now.all - (then ? then.all : 0);
    return all > 0 ? (now.busy - (then ? then.busy : 0)) / all : 0;
}

// Each core's share between two reads, in core order.
function loads(now, then) {
    return now.cores.map(core => ({
        n: core.n,
        load: busy(core, then ? then.cores.find(before => before.n === core.n) : null)
    }));
}

// The maxed cores, as a row says them: none, by number while they're few,
// or how many of how many.
function maxed(cores) {
    const flat = cores.filter(core => core.load >= MAXED).map(core => core.n);
    if (flat.length === 0) {
        return "none";
    }
    if (flat.length > NAMED) {
        return `${flat.length} of ${cores.length}`;
    }
    if (flat.length === 1) {
        return `core ${flat[0]}`;
    }
    return `cores ${flat.slice(0, -1).join(", ")} and ${flat.slice(-1)[0]}`;
}

// A processor's model as /proc/cpuinfo names it, without its trade marks:
// "Intel(R) Core(TM) Ultra X7 358H" is an Intel Core Ultra X7 358H.
function model(name) {
    return name.replace(MARKS, " ").trim().split(SPACES).join(" ");
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
    const cores = [];
    for (const cluster of reading.clusters ?? []) {
        const most = strongest(cluster.heldBack);
        if (most) {
            for (const n of cpus(cluster.cpus)) {
                cores.push({ n, share: most.share, by: most.by });
            }
        }
    }
    cores.sort((a, b) => a.n - b.n);
    const whole = strongest(reading.heldBack);
    const most = whole ?? strongest(cores);
    return most ? { share: most.share, by: most.by, whole: whole !== null, cores } : null;
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

// Each core's kind, by its number, on a processor of more than one kind:
// those the kernel calls cores are for performance, and of its smaller
// ones, those that clock highest are efficient and the rest low-power.
// Null when the kernel says no core is a performance core.
function kinds(performance, cores) {
    if (performance.length === 0) {
        return null;
    }
    const small = cores.filter(core => !performance.includes(core.n));
    const fastest = Math.max(...small.map(core => core.max));
    const kind = {};
    for (const core of cores) {
        if (performance.includes(core.n)) {
            kind[core.n] = "performance";
        } else {
            kind[core.n] = core.max >= fastest ? "efficient" : "low-power";
        }
    }
    return kind;
}

// A disk's counters, from its /sys/block/<disk>/stat.
function disk(text) {
    const fields = text.trim().split(SPACES).map(Number);
    return fields.length > BUSY_MS && !fields.some(Number.isNaN) ? fields : null;
}

// How a disk did between two reads, `seconds` apart: bytes a second read
// and written, its share of the time busy, and the milliseconds a request
// took, or null with none made. With no read before, since it started.
function rates(now, then, seconds) {
    const change = field => now[field] - (then ? then[field] : 0);
    const requests = change(READS) + change(WRITES);
    const time = seconds > 0 ? seconds : 1;
    return {
        read: change(SECTORS_READ) * SECTOR / time,
        write: change(SECTORS_WRITTEN) * SECTOR / time,
        busy: Math.min(1, change(BUSY_MS) / (time * MILLISECONDS)),
        latency: requests > 0 ? (change(READING_MS) + change(WRITING_MS)) / requests : null
    };
}

// A program by the name it's known by: the kernel keeps fifteen letters of
// a process's name, and Nix runs a wrapped program as ".kitty-wrapped",
// cut short as ".firefox-wrappe" when the name is long.
function program(name) {
    if (!name.startsWith(".")) {
        return name;
    }
    const bare = name.slice(1);
    for (let cut = WRAPPED.length; cut > 0; cut -= 1) {
        const end = WRAPPED.slice(0, cut);
        const whole = cut === WRAPPED.length || name.length === NAME_KEPT;
        if (whole && bare.endsWith(end) && bare.length > end.length) {
            return bare.slice(0, -end.length);
        }
    }
    return bare;
}

// Where the field `count` past the one starting at `at` starts, in a line
// ending at `end`, or `end` when there's none.
function walk(text, at, end, count) {
    let start = at;
    for (let n = 0; n < count && start < end; n += 1) {
        const space = text.indexOf(" ", start);
        start = space < 0 || space >= end ? end : space + 1;
    }
    return start;
}

// The number a field starting at `at` says.
function numberAt(text, at, end) {
    const space = text.indexOf(" ", at);
    return Number(text.slice(at, space < 0 || space > end ? end : space));
}

// What one read of the processes says: the page size first, then
// /proc/stat, then each process's stat line. The whole machine's ticks, and
// each process by its id with its program, its ticks and its resident
// bytes. It's a fifth of a megabyte, read each second while the programs
// using the processors or the memory are shown, so it's walked by
// position: splitting every line into its fifty fields took twelve
// milliseconds of the shell's thread, and under load forty.
function processes(text) {
    const firstEnd = text.indexOf("\n");
    const page = Number(text.slice(0, firstEnd < 0 ? text.length : firstEnd)) || 1;
    const programs = {};
    const cpuAt = text.startsWith("cpu ") ? 0 : text.indexOf("\ncpu ") + 1;
    const cpuEnd = text.indexOf("\n", cpuAt);
    const machine = cpuAt === 0 && !text.startsWith("cpu ") ? null
        : text.slice(cpuAt, cpuEnd < 0 ? text.length : cpuEnd);
    const ticks = machine ? counters(machine).whole.all : 0;
    let start = 0;
    while (start < text.length) {
        const found = text.indexOf("\n", start);
        const end = found < 0 ? text.length : found;
        const first = text.charCodeAt(start);
        // A process's line starts with its id, a digit, and its name in
        // brackets follows it.
        const opened = first >= DIGIT_ZERO && first <= DIGIT_NINE ? text.indexOf(" (", start) : -1;
        if (opened > start && opened < end && PROCESS_LINE.test(text.slice(start, opened + 2))) {
            const close = text.lastIndexOf(")", end);
            const user = walk(text, close + 2, end, USER_TIME);
            const system = walk(text, user, end, SYSTEM_TIME - USER_TIME);
            const resident = walk(text, system, end, RESIDENT - SYSTEM_TIME);
            programs[text.slice(start, opened)] = {
                name: program(text.slice(opened + 2, close)),
                ticks: numberAt(text, user, end) + numberAt(text, system, end),
                bytes: numberAt(text, resident, end) * page
            };
        }
        start = end + 1;
    }
    return { ticks, programs };
}

// The programs using most of something, by name, each program's
// processes summed: [{ name, value }], at most TOP, none at nothing.
function top(values) {
    return Object.keys(values)
        .map(name => ({ name, value: values[name] }))
        .filter(each => each.value > 0)
        .sort((a, b) => b.value - a.value || (a.name < b.name ? -1 : 1))
        .slice(0, TOP);
}

// The programs that used the processors most between two reads, each
// with its share of the whole machine, from 0 to 1. A process that started
// between them is left out, since its time before can't be told. With no
// read before (`then` null), since the machine started, as the processors'
// own first figure is, so a first read has its programs too.
function using(now, then) {
    const all = now.ticks - (then ? then.ticks : 0);
    const ticks = {};
    if (all <= 0) {
        return [];
    }
    for (const id of Object.keys(now.programs)) {
        const process = now.programs[id];
        const before = then ? then.programs[id] : { name: process.name, ticks: 0 };
        if (before && before.name === process.name) {
            ticks[process.name] = (ticks[process.name] ?? 0) + process.ticks - before.ticks;
        }
    }
    return top(ticks).map(each => ({ name: each.name, value: each.value / all }));
}

// The programs holding the most memory, in bytes.
function holding(now) {
    const bytes = {};
    for (const process of Object.values(now.programs)) {
        bytes[process.name] = (bytes[process.name] ?? 0) + process.bytes;
    }
    return top(bytes);
}

// /proc/meminfo's fields, in KiB.
function meminfo(text) {
    const info = {};
    for (const line of text.split("\n")) {
        const field = MEMINFO_LINE.exec(line);
        if (field) {
            info[field[1]] = Number(field[2]);
        }
    }
    return info;
}

// The share of the last ten seconds something waited for memory, 0 to 1,
// or null where the kernel doesn't say.
function pressure(text) {
    const some = SOME_PRESSURE.exec(text);
    return some ? Number(some[1]) / PERCENT : null;
}

// The memory modules the machine read at boot, as /run/memory-modules.json
// lists them: their type and speed where every module says the same, how
// many and how big, and who made them. Null with none listed or no type.
function modules(text) {
    let said = null;
    try {
        said = JSON.parse(text);
    } catch (_error) {
        return null;
    }
    const listed = Array.isArray(said?.modules) ? said.modules : [];
    const alike = field => {
        const values = listed.map(each => String(each[field] ?? "").trim())
            .filter(value => value !== "");
        return values.length > 0 && values.every(value => value === values[0]) ? values[0] : "";
    };
    const type = alike("type");
    if (type === "") {
        return null;
    }
    return {
        type,
        speed: alike("speed"),
        size: alike("size"),
        maker: alike("manufacturer"),
        count: listed.length
    };
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
