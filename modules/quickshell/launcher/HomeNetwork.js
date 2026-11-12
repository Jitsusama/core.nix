// The figures the launcher says a network's rates and sizes in, every
// figure in its own unit, said one way, as the Home Network's are: the
// Work Network (Warp.js), Machine and the virtual machines say theirs so
// too. And how many nodes and devices the Home Network holds, as its row
// says at home. Its rows and their drawers are the eero daemon's
// (modules/eero's rows.rs and drawers.rs), served ready, so nothing here
// reads them. Held in Qt's own engine (tests/home-network.qml).
.pragma library
.import "Numbers.js" as Numbers

const KILO = 1000;
// Every figure takes three significant digits, so 369.9 reads 370 and
// 1,200.9 Mb/s reads 1.20 Gb/s (VKCF3Z, 2026-11-07).
const DIGITS = 3;
// A rate's units from Mb/s, as eero gives one, each a thousand of the
// one before.
const RATE_UNITS = [[1e-6, "b/s"], [1e-3, "kb/s"], [1, "Mb/s"], [KILO, "Gb/s"]];
// Bytes in SI units, as the file picker says a size.
const BYTE_UNITS = ["B", "kB", "MB", "GB", "TB"];

// A number grouped and pointed as the launcher's settings say, at most
// `places` decimals.
function number(value, places, settings) {
    const set = settings ?? {};
    return Numbers.format(value, places, set.group, set.point);
}

// A figure to three significant digits, by the launcher's one rule
// (Numbers.significant), grouped and pointed as the settings say: 369.9
// as "370", 11.94 as "11.9", 1.2 as "1.20".
function significant(value, settings) {
    return Numbers.significant(value, settings?.group, settings?.point);
}

// A rate given in Mb/s, in whichever unit reads it in three digits:
// "370 Mb/s", "1.20 Gb/s", "98.0 b/s".
function rate(value, settings) {
    if (value === null || value === undefined) {
        return "";
    }
    const size = Math.abs(Number(Number(value).toPrecision(DIGITS)));
    const unit = RATE_UNITS.slice().reverse().find((each) => size >= each[0])
        ?? RATE_UNITS[0];
    return `${significant(value / unit[0], settings)} ${unit[1]}`;
}

// Bytes in SI units, to three digits: "940 kB", "2.40 MB".
function bytes(value, settings) {
    if (value === null || value === undefined) {
        return "";
    }
    let scaled = Number(Number(value).toPrecision(DIGITS));
    let unit = 0;
    while (scaled >= KILO && unit < BYTE_UNITS.length - 1) {
        scaled /= KILO;
        unit += 1;
    }
    return `${unit === 0 ? number(scaled, 0, settings) : significant(scaled, settings)} `
        + BYTE_UNITS[unit];
}

// A count of a noun, "1 node", "3 nodes".
function counted(count, noun) {
    return `${count} ${count === 1 ? noun : `${noun}s`}`;
}

// How many nodes and devices the place holds, as its row says at home.
function holding(model) {
    const nodes = (model?.nodes ?? []).length;
    const devices = (model?.devices ?? []).length;
    return `${counted(nodes, "node")} \u00b7 ${counted(devices, "device")}`;
}
