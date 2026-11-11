// What the launcher's machine answers make of the machine daemon's records
// (modules/machine), in plain JavaScript so a check can run it in Qt's own
// engine, as the shell does: bytes as the launcher says them, and the
// processor's own sensor among its chip's. The processor's and the
// disk's own rows are the daemon's (modules/machine/src/processor_rows.rs,
// telemetry_rows.rs and disk_rows.rs), worded there and drawn as every
// place's served rows are (ReadyRows.qml).
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
