// What the launcher's Heat makes of the machine's sensors, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// the kernel's hwmon chips, as Sensors.qml reads them, gathered into the
// parts Joel knows (the processor and its cores, its NPU, image processor
// and media and display engines as its own telemetry says them, the disk
// and its other sensors, the memory, the air inside, each fan), how near
// each is to the most it's meant to reach, and how often the processor has
// slowed itself to cool down. Heat.qml reads the files and draws the rows;
// Machine.qml draws its processor's and disk's temperature with the same
// level, so the two say them alike.
.pragma library
.import "Machine.js" as Machine

// A sensor on a chip that measures many, as dell_ddv's are, by its label,
// as the part it stands for. One labelled "CPU" or "Other" is left out: the
// processor's own chip says the first, and the rest say nothing of what
// they measure.
const LABELLED = [
    ["Ambient", "air"],
    ["SODIMM", "memory"],
    ["GPU", "graphics"]
];

// What a thermal zone measures, by its type, for a machine whose hwmon
// chips leave a part out. On a laptop with every chip they're all said
// already (the package, the Wi-Fi card, the firmware's own sensors), so
// they're read only for a part nothing else gives.
const ZONES = [
    [/^(x86_pkg_temp|TCPU|B0D4|TCPU_PCI)$/, "processor"],
    [/^acpitz$/, "board"],
    [/^iwlwifi/, "wifi"],
    [/^pch_/, "chipset"]
];

// The processor's own parts no hwmon chip measures, as its telemetry
// reader (ProcessorTelemetry.qml) says them, by the part Heat calls each.
const TELEMETRY = [
    ["npu", "npu"],
    ["imageProcessor", "image-processor"],
    ["media", "media-engine"],
    ["display", "display-engine"]
];

// The parts in the order they're listed, the processor first as what's
// asked most, and its own parts after its graphics.
const ORDER = ["processor", "graphics", "npu", "image-processor", "media-engine",
    "display-engine", "disk", "memory", "wifi", "chipset", "board", "battery", "air"];

// What a part is drawn against when its chip says no most: the disk's
// usual most, and a battery's, which wants to stay well below a
// processor's.
const USUAL = { disk: 85, battery: 60 };
const USUAL_MOST = 100;
// How near the most a temperature warns, where the chip says no lower
// point of its own.
const NEAR = 10;
// The least span of degrees a spark is drawn across, so a reading that
// barely moves draws flat rather than its own noise full.
const SPAN = 10;
// A fan's spark is drawn across this many turns a minute at least.
const FAN_SPAN = 500;

const CORE = /^Core (\d+)$/;
const WHOLE = /^(Composite|Package|Tctl|Tdie)/;
// A word with no capital but its first, which a sentence lowers.
const PLAIN_WORD = /^[A-Z][a-z]*$/;
const SPACES = /\s+/;

function labelled(label) {
    const match = LABELLED.find(each => each[0] === label);
    return match ? match[1] : "";
}

function zonePart(type) {
    const match = ZONES.find(each => each[0].test(type));
    return match ? match[1] : "";
}

// Whether any chip gives a part's temperature, so a thermal zone isn't
// read for it.
function gives(chips, part) {
    const names = chip => chip.temps.some(sensor => labelled(sensor.label) === part);
    return chips.some(chip => chip.temps.length > 0
        && (chip.part === part || (chip.part === "" && names(chip))));
}

// A fan's id within Heat's words, by its label: "Video Fan" is video-fan.
function fanId(label) {
    return label.toLowerCase().trim().split(SPACES).join("-");
}

// A part's name, a title, as a sentence says it after "the": "Air Inside"
// is "air inside", while a word with more capitals, as "Wi-Fi" in
// "Wi-Fi Card", keeps them.
function phrase(name) {
    return name
        .split(" ")
        .map(word => (PLAIN_WORD.test(word) ? word.toLowerCase() : word))
        .join(" ");
}

function given(value) {
    return value !== null && value !== undefined;
}

// Where a temperature starts to warn: the chip's own high point where
// that's below its critical one, as a disk's is, and else ten degrees
// short of the critical, as a processor's high and critical are one; or
// nowhere, with neither said, since nothing says where trouble starts.
function warnsFrom(high, crit) {
    if (given(high) && !(given(crit) && high >= crit)) {
        return high;
    }
    if (given(crit)) {
        return crit - NEAR;
    }
    return null;
}

// How near a temperature is to the most it's meant to reach: its share
// of that most, or of its part's usual most where the chip says none,
// and whether it warns.
function level(value, high, crit, part) {
    const most = crit ?? high ?? USUAL[part] ?? USUAL_MOST;
    const from = warnsFrom(high, crit);
    return { value: Math.max(0, Math.min(1, value / most)), warn: from !== null && value >= from };
}

// A spark's values, from 0 to 1, across the span its reads cover or
// `least`, whichever is more, centred, so a steady reading draws as a
// level line halfway up and a climb draws as one.
function spark(values, least) {
    if (values.length === 0) {
        return [];
    }
    const most = Math.max(...values);
    const fewest = Math.min(...values);
    const span = Math.max(least ?? SPAN, most - fewest);
    const floor = (most + fewest) / 2 - span / 2;
    return values.map(value => (value - floor) / span);
}

function fanSpark(values) {
    return spark(values, FAN_SPAN);
}

// Each core's logical processor, by the core's number as its chip labels
// it, so Heat's Core 1 is the one Machine's Cores call Core 1: the lowest
// of the processors that share it.
function coreCpus(cpus) {
    const out = {};
    for (const each of cpus) {
        if (out[each.core] === undefined || each.cpu < out[each.core]) {
            out[each.core] = each.cpu;
        }
    }
    return out;
}

function hottest(temps) {
    return temps.reduce((a, b) => (a && a.value >= b.value ? a : b), null);
}

// A temperature part: its id, its whole reading and limits, its other
// sensors and its cores.
function entry(id, whole, sensors, cores) {
    return {
        id,
        value: whole.value,
        high: whole.high ?? null,
        crit: whole.crit ?? null,
        sensors,
        cores
    };
}

// The processor's part: as its package reads, and each core by its
// logical processor.
function processorPart(chip, cpuOfCore) {
    const cores = chip.temps.filter(sensor => CORE.test(sensor.label)).map(sensor => {
        const n = Number(CORE.exec(sensor.label)[1]);
        return { cpu: cpuOfCore[n] ?? n, value: sensor.value };
    }).sort((a, b) => a.cpu - b.cpu);
    return entry("processor", Machine.processor(chip.temps), [], cores);
}

// The parts one chip gives: the processor's, one whole reading of a chip
// that measures one part (a disk's composite, or its hottest), its other
// sensors beside, or each named sensor of one that measures many.
function chipParts(chip, cpuOfCore) {
    if (chip.temps.length === 0) {
        return [];
    }
    if (chip.part === "processor") {
        return [processorPart(chip, cpuOfCore)];
    }
    if (chip.part === "") {
        return chip.temps.filter(sensor => labelled(sensor.label) !== "")
            .map(sensor => entry(labelled(sensor.label), sensor, [], []));
    }
    const whole = chip.temps.find(sensor => WHOLE.test(sensor.label)) ?? hottest(chip.temps);
    const sensors = chip.temps.filter(sensor => sensor !== whole)
        .map(sensor => ({ n: sensor.n, label: sensor.label, value: sensor.value }));
    return [entry(chip.part, whole, sensors, [])];
}

// The parts in their order, then as the chips were listed, so two disks
// keep theirs; a part said twice, as two disks are, is numbered (nth),
// and one said once is 0.
function placed(parts) {
    const rank = id => (ORDER.includes(id) ? ORDER.indexOf(id) : ORDER.length);
    const sorted = parts.map((each, at) => ({ each, at }))
        .sort((a, b) => rank(a.each.id) - rank(b.each.id) || a.at - b.at)
        .map(one => one.each);
    for (const each of sorted) {
        const alike = sorted.filter(other => other.id === each.id);
        each.nth = alike.length > 1 ? alike.indexOf(each) + 1 : 0;
    }
    return sorted;
}

// The processor's own parts, from its telemetry reader's temperatures,
// each { part, celsius }: those Heat lists, each held to the processor's
// high and critical points, as they sit on the same die and it slows them
// all at once. A part it names that Heat doesn't list is left out.
function ownParts(telemetry, processor) {
    return (telemetry ?? []).map(each => [TELEMETRY.find(one => one[0] === each.part), each])
        .filter(pair => pair[0] !== undefined)
        .map(pair => entry(pair[0][1], {
            value: pair[1].celsius,
            high: processor ? processor.high : null,
            crit: processor ? processor.crit : null
        }, [], []));
}

// The parts, from each chip as Heat.qml reads them, { part, temps, fans },
// each temperature { n, label, value, high, crit } in degrees and each fan
// { n, label, value, max, target, power }, the thermal zones, each
// { part, value }, the first of a part no chip gives added, and the
// processor's telemetry's temperatures, each { part, celsius }. Gives
// { temps, fans }.
function gather(chips, zones, cpuOfCore, telemetry) {
    const temps = [];
    for (const chip of chips) {
        temps.push(...chipParts(chip, cpuOfCore ?? {}));
    }
    const added = (zones ?? []).filter((zone, at, all) => zone.part !== ""
        && !temps.some(each => each.id === zone.part)
        && all.findIndex(other => other.part === zone.part) === at)
        .map(zone => entry(zone.part, zone, [], []));
    const zoned = temps.concat(added);
    const own = ownParts(telemetry, zoned.find(each => each.id === "processor"));
    return { temps: placed(zoned.concat(own)), fans: fans(chips) };
}

function fanOf(fan) {
    return {
        id: fan.label === "" ? "fan" : fanId(fan.label),
        n: fan.n,
        label: fan.label,
        value: fan.value,
        max: fan.max ?? null,
        target: fan.target ?? null,
        power: fan.power ?? null
    };
}

// A named fan, with what the chip that doesn't name it says.
function toldOf(made, other) {
    if (!other) {
        return made;
    }
    made.max = made.max ?? other.max ?? null;
    made.target = made.target ?? other.target ?? null;
    made.power = made.power ?? other.power ?? null;
    return made;
}

// Two chips can report the same fans, one naming them and one not, as
// dell_ddv and dell_smm do: the named are the fans, and an unnamed chip
// with as many says, fan by fan in order, their most, their aim and their
// power. Where none are named, the first chip's unnamed fans are, by
// number.
function fans(chips) {
    const all = [];
    for (const chip of chips) {
        all.push(...chip.fans);
    }
    const named = all.filter((fan, at) => fan.label !== ""
        && all.findIndex(other => other.label === fan.label) === at);
    const unnamed = chips.map(chip => chip.fans.filter(fan => fan.label === ""))
        .filter(list => list.length > 0);
    if (named.length === 0) {
        return unnamed.length > 0 ? unnamed[0].map(fanOf) : [];
    }
    const told = unnamed.find(list => list.length === named.length) ?? [];
    return named.map((fan, i) => toldOf(fanOf(fan), told[i]));
}

// How often the processor has slowed itself to cool down since the
// machine started, from each logical processor's counts, { cpu, core,
// cores, package, packageMs, packageLongest }: the package's events are
// counted by each processor that saw them, so the most any saw is the
// package's, and a core's are said by each processor sharing it, so each
// core counts once, by its lowest processor. Gives { times, ms, longest,
// cores, total }, or null with nothing counted.
function throttled(cpus) {
    if (cpus.length === 0) {
        return null;
    }
    const most = cpus.reduce((a, b) => (a && a.package >= b.package ? a : b), null);
    const byCore = {};
    for (const each of cpus) {
        if (byCore[each.core] === undefined || each.cpu < byCore[each.core].cpu) {
            byCore[each.core] = { cpu: each.cpu, count: each.cores };
        }
    }
    const cores = Object.keys(byCore).map(core => byCore[core]).sort((a, b) => a.cpu - b.cpu);
    return {
        times: most.package,
        ms: most.packageMs ?? null,
        longest: most.packageLongest ?? null,
        cores,
        total: cores.reduce((sum, each) => sum + each.count, 0)
    };
}
