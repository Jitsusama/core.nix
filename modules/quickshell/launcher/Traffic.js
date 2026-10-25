// What the network is carrying, worked out from the kernel's counters for
// every interface (/proc/net/dev, one file for all) read twice, and what
// the line can carry and how quickly it answers, as the shaper last
// measured them (/run/shaper/status.json). Traffic.qml reads the files;
// everything here is plain arithmetic on what they said, so it's tested
// offscreen (tests/answers.qml).
.pragma library

const BITS = 8;
const KILO = 1000;
const MEGA = 1e6;
const GIGA = 1e9;
const TENS = 10;
// Where /proc/net/dev puts each count, after the interface's name.
const RECEIVED = 0;
const RECEIVED_DROPPED = 3;
const SENT = 8;
const SENT_DROPPED = 11;
const LINE = /^\s*([^\s:]+):\s*(.*)$/;
const SPACES = /\s+/;

// Every interface's counters, by name, as /proc/net/dev says them: bytes
// in and out, and packets dropped either way. Its two heading lines, and
// any line it couldn't have written, say nothing.
function counters(text) {
    const all = {};
    for (const each of String(text).split("\n")) {
        const said = LINE.exec(each);
        const fields = said ? said[2].trim().split(SPACES).map(Number) : [];
        if (fields.length > SENT_DROPPED && fields.every((field) => Number.isFinite(field))) {
            all[said[1]] = {
                in: fields[RECEIVED],
                out: fields[SENT],
                dropped: fields[RECEIVED_DROPPED] + fields[SENT_DROPPED],
            };
        }
    }
    return all;
}

// Bytes a second each way between two reads of one interface, `seconds`
// apart, and packets dropped a second. With no read before, it's since
// the interface came up, and a counter that went back (the interface came
// up again) starts from nothing.
function between(now, before, seconds) {
    const from = before ?? { in: 0, out: 0, dropped: 0 };
    const gone = (key) => Math.max(0, now[key] - (now[key] >= from[key] ? from[key] : 0));
    const over = Math.max(seconds, 1 / KILO);
    return { in: gone("in") / over, out: gone("out") / over, dropped: gone("dropped") / over };
}

// Every interface's rates, `now` and `before` each mapping a name to
// { kind, counters }, each kind as the addresses say it (Interfaces.js's
// kindOf), "" for one never listed, and the machine's own total: the
// cards' alone, Wi-Fi and wired, since what a tunnel carries goes over a
// card too.
function rates(now, before, seconds) {
    const each = Object.keys(now)
        .filter((name) => now[name].kind !== "" && now[name].counters !== null)
        .map((name) => {
            const then = before?.[name]?.counters ?? null;
            return Object.assign(
                { name, kind: now[name].kind },
                between(now[name].counters, then, seconds),
            );
        });
    const cards = each.filter((one) => one.kind === "wifi" || one.kind === "wired");
    const sum = (key) => cards.reduce((all, one) => all + one[key], 0);
    return { each, in: sum("in"), out: sum("out"), dropped: sum("dropped") };
}

// A rate in bits a second, as a network's speed is said.
function bits(bytesPerSecond, number) {
    const rate = bytesPerSecond * BITS;
    const units = [
        [GIGA, "Gb/s"],
        [MEGA, "Mb/s"],
        [KILO, "kb/s"],
    ];
    const unit = units.find((each) => rate >= each[0]);
    if (!unit) {
        return `${Math.round(rate)} b/s`;
    }
    const scaled = rate / unit[0];
    return `${number(scaled, scaled < TENS ? 1 : 0)} ${unit[1]}`;
}

// What the shaper says of one uplink: what it holds the line to each way,
// in bytes a second, what held under load, how far it is in finding the
// line's most, and how long each server took to answer while the line was
// quiet, the quickest first. Null when it doesn't shape that uplink.
function line(status, name) {
    if (!Array.isArray(status)) {
        return null;
    }
    const link = status.find((each) => each.name === name);
    if (!link) {
        return null;
    }
    const trips = (link.baselines ?? [])
        .filter((trip) => Array.isArray(trip) && Number.isFinite(trip[1]))
        .map((trip) => ({ server: String(trip[0]), ms: trip[1] }))
        .sort((a, b) => a.ms - b.ms);
    return {
        in: (link.down * KILO) / BITS,
        out: (link.up * KILO) / BITS,
        state: String(link.state ?? ""),
        follows: link.follows ?? null,
        trips,
    };
}

// The names of the shaper's mirrors, which carry each uplink's traffic in
// again and so are never counted.
function mirrors(status) {
    return Array.isArray(status) ? status.map((each) => String(each.ifb ?? "")) : [];
}

// The middle of the round trips, the one a person would call the delay.
function typical(trips) {
    if (trips.length === 0) {
        return null;
    }
    const sorted = trips.map((trip) => trip.ms).sort((a, b) => a - b);
    const middle = Math.floor(sorted.length / 2);
    return sorted.length % 2 === 1 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}
