// What the machine daemon says of the screen's level, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// the level the backlight program last settled, what its curve asks for
// the room it heard, whether it follows the room and its curve's points
// (modules/machine's steered.rs, which reads and parses its files); and
// the backlight's steps and the room's sensor (light.rs). Displays.qml
// holds the daemon's records and draws the screen's drawer.
.pragma library
.import "Numbers.js" as Numbers
.import "Order.js" as Order
.import "Plate.js" as Plate

const PERCENT = 100;
// The daemon moves the level only for a gap this wide or wider
// (steer.rs's SLIGHT), so a narrower one is where it rests.
const SLIGHT = 0.03;
// Lux are said in lux, and from a thousand in klx, a thousand each; a
// reading from where its three digits round to a thousand, so 999.6 reads
// "1.00 klx" rather than "1,000 lx".
const LUX = "lx";
const KILOLUX = "klx";
const KILO = 1000;
const ROUNDS_TO_KILO = 999.5;

// The level as a share of the whole, rounded as Brightness says it.
function share(level) {
    return Math.round(level * PERCENT);
}

// A vocabulary's words with each {name} in them filled from the values.
function fill(words, values) {
    return Object.keys(values).reduce((filled, name) =>
        filled.replace(`{${name}}`, values[name]), String(words ?? ""));
}

// The machine daemon's record of the built-in screen's brightness, or
// null with none.
function brightness(records) {
    return records.find(record => record?.kind === "brightness") ?? null;
}

// The level the backlight program last settled: { level, lux, asks,
// following }, the levels shares of the whole, lux and asks null while no
// light is heard; or null while it says none, as when it has stopped.
// `following` is the program's: light heard and the lid open, the curve
// setting the level; a pick is learned into the curve at once, so there's
// no pick held apart from it.
function settled(records) {
    const found = brightness(records);
    if (!Number.isFinite(found?.level_ratio) || typeof found.following !== "boolean") {
        return null;
    }
    const finite = value => Number.isFinite(value) ? value : null;
    return {
        level: found.level_ratio,
        lux: finite(found.lux),
        asks: finite(found.asks_ratio),
        following: found.following
    };
}

// The curve's points, dimmest first, as the daemon says them: [{ lux,
// level }], none while it says none.
function curve(records) {
    return (brightness(records)?.curve ?? [])
        .map(point => ({ lux: point.lux, level: point.level_ratio }));
}

// What sets the level, in the vocabulary's words: the room while the
// curve follows it, and what it asks while the level is still stepping
// there; "" while no light is heard or the lid is shut, where nothing does.
function follows(settled, said) {
    if (settled === null || !settled.following) {
        return "";
    }
    // Compared raw, as the daemon compares them.
    const stepping = settled.asks !== null
        && Math.abs(settled.asks - settled.level) >= SLIGHT;
    return Plate.runs([said("the-room", {}),
        stepping ? said("on-its-way", { share: `${share(settled.asks)}%` }) : ""]);
}

// What the Room Light says: the room as the sensor hears it now, or as
// the daemon last heard it; or, where the daemon says it heard no light,
// that none is, so the drawer says why Follows is gone (VKCF3Z,
// 2026-11-08). A shut lid still hears the room, so it's said then too.
// "" where nothing says anything.
function room(settled, heard, said, lux) {
    if (heard !== null) {
        return lux(heard);
    }
    if (settled === null) {
        return "";
    }
    return settled.lux !== null ? lux(settled.lux) : said("no-light", {});
}

// The room's light as it's measured, to three significant digits with
// its zeros kept: "28.5 lx", "5.20 lx", "1.24 klx" (VKCF3Z, 2026-11-08).
function reading(lux, group, point) {
    return lux >= ROUNDS_TO_KILO
        ? `${Numbers.significant(lux / KILO, group, point)} ${KILOLUX}`
        : `${Numbers.significant(lux, group, point)} ${LUX}`;
}

// A curve's lux, a point the daemon chose rather than one it measured, so
// a name: every digit and none added, in a reading's units so the two
// compare: "50 lx", "2 klx", "1.5 klx" (VKCF3Z, 2026-11-08).
function chosen(lux, group, point) {
    return lux >= KILO
        ? `${Numbers.format(lux / KILO, undefined, group, point)} ${KILOLUX}`
        : `${Numbers.format(lux, undefined, group, point)} ${LUX}`;
}

// The first of a kind of the machine daemon's records (light.rs), by its
// device's name as the kernel lists them, or null with none.
function first(records, kind, holds) {
    return Order.sorted(records.filter(record => record?.kind === kind && holds(record)),
        Order.byName(record => record.device))[0] ?? null;
}

// The screen's backlight, from the machine daemon's records: its steps
// now and most, its share, the steps the hardware reads back where they
// differ, as when firmware moved it, how its steps map to light, what
// drives it, its device and the screen it lights; or null with none that
// says the most it can be.
function lit(records) {
    const found = first(records, "backlight",
        record => record.max_brightness > 0 && Number.isFinite(record.brightness));
    if (found === null) {
        return null;
    }
    const reads = found.actual_brightness;
    return {
        now: found.brightness,
        most: found.max_brightness,
        share: Math.round(found.brightness / found.max_brightness * PERCENT),
        reads: Number.isFinite(reads) && reads !== found.brightness ? reads : null,
        stepping: found.scale ?? null,
        driven: found.type ?? null,
        device: found.device,
        connector: found.connector ?? null
    };
}

// The room's light sensor, from the machine daemon's records: the lux it
// hears, its raw count, the lux a count is, its offset and its device; or
// null with none. Its name is always als, the only one light.rs reads.
function sensor(records) {
    const found = first(records, "light", () => true);
    if (found === null) {
        return null;
    }
    return {
        lux: Number.isFinite(found.lux) ? found.lux : null,
        raw: Number.isFinite(found.raw) ? found.raw : null,
        perCount: found.scale,
        offset: found.offset ?? 0,
        device: found.device
    };
}

// What the kernel says of the backlight past its share, as a screen's
// drawer says it after Brightness: the steps it Reads where they differ
// from those set, its Stepping and what it's Driven By, each only where
// the kernel says it (VKCF3Z, 2026-11-09). `named` names a fact by its
// id, `said` is the vocabulary's words, and `count` groups a figure in
// threes.
function kernelFacts(light, named, said, count) {
    const word = (kind, key) => key === null ? "" : said[`${kind}-${key}`] ?? "";
    return drawn([
        { id: "reads", value: light.reads === null ? "" : count(light.reads) },
        { id: "stepping", value: word("stepping", light.stepping) },
        { id: "driven-by", value: word("driven", light.driven) }
    ], named);
}

// Facts by id, those with nothing to say left out, each named.
function drawn(facts, named) {
    return facts.filter(fact => fact.value !== "")
        .map(fact => ({ label: named(fact.id), value: fact.value }));
}

// What the sensor counts, as a screen's drawer says it after Room Light:
// its Raw count, the lux Per Count, and its Offset where it isn't none.
function sensorFacts(sensed, named, said, count) {
    return drawn([
        { id: "raw", value: sensed.raw === null ? "" : count(sensed.raw) },
        { id: "per-count", value: said.perCount.replace("{n}", count(sensed.perCount)) },
        { id: "offset", value: sensed.offset === 0 ? "" : count(sensed.offset) }
    ], named);
}

// Each of the curve's points as its share at its lux, a run apart,
// dimmest first: "15% at 0 lx · 38% at 6 lx".
function curveSaid(points, said, lux) {
    return Plate.runs(points.map(point =>
        said("at", { share: `${share(point.level)}%`, lux: lux(point.lux) })));
}
