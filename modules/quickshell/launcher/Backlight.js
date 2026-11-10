// What the backlight daemon says of the screen's level, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does:
// the level it last settled, what its curve asks for the room it heard
// and whether it follows the room, from backlight-now; and its curve's
// points, from its learned file; and what the machine daemon says of the
// backlight's steps and the room's sensor. Displays.qml reads the files,
// holds the daemon's records and draws the screen's drawer.
.pragma library
.import "Numbers.js" as Numbers
.import "Order.js" as Order
.import "Plate.js" as Plate

// A fact a line, its name, a tab and its value, as the daemon writes them.
const FACT = /^([a-z]+)\t(.*)$/;
// A curve's point a line, its lux and its level, as the daemon writes it;
// a blank line or one opening with # is no point.
const SPACES = /\s+/;
const COMMENT = "#";
const FIELDS = 2;
// A figure as Rust's parse takes one: decimal, never hex as Number takes.
const DECIMAL = /^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$/;
// The curve's levels lie between these, and its lux are never below none,
// as Curve::through keeps them.
const DIMMEST = 0.02;
const BRIGHTEST = 1;
const NO_LUX = 0;
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

// A figure the daemon would read, or NaN.
function figure(text) {
    return DECIMAL.test(String(text ?? "")) ? Number(text) : Number.NaN;
}

// What backlight-now says: { level, lux, asks, following }, the levels
// shares of the whole, lux and asks null while no light is heard; or null while it
// says nothing, as when the daemon has stopped and emptied it, or says
// it in a way the daemon never writes. `following` is the daemon's: light
// heard and the lid open, the curve setting the level; a pick is learned
// into the curve at once, so there's no pick held apart from it.
function now(text) {
    const said = {};
    for (const line of String(text ?? "").split("\n")) {
        const found = FACT.exec(line);
        if (found) {
            said[found[1]] = found[2];
        }
    }
    const level = figure(said.level);
    const known = Number.isFinite(level)
        && (said.following === "yes" || said.following === "no");
    if (!known) {
        return null;
    }
    const finite = value => Number.isFinite(value) ? value : null;
    return {
        level,
        lux: finite(figure(said.lux)),
        asks: finite(figure(said.asks)),
        following: said.following === "yes"
    };
}

// The curve's points, dimmest first: [{ lux, level }], each kept where
// the daemon keeps it. A file the daemon would refuse, any line of it not
// two figures, gives none, as the daemon falls back to its standard curve
// then.
function curve(text) {
    const lines = String(text ?? "").split("\n").map(line => line.trim())
        .filter(line => line !== "" && !line.startsWith(COMMENT));
    const points = lines.map(line => line.split(SPACES)).map(fields => ({
        whole: fields.length === FIELDS,
        lux: figure(fields[0]),
        level: figure(fields[1])
    }));
    const read = points.every(point => point.whole && Number.isFinite(point.lux)
        && Number.isFinite(point.level));
    return !read ? [] : points.map(point => ({
        lux: Math.max(point.lux, NO_LUX),
        level: Math.min(Math.max(point.level, DIMMEST), BRIGHTEST)
    })).sort((one, other) => one.lux - other.lux);
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
// hears, its raw count, the lux a count is, its offset, its name and its
// device; or null with none.
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
        name: found.name,
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
