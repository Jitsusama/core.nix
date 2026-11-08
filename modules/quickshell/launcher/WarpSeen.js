// What the launcher's WARP keeps of what it read while it's shown, and
// the sparks and drawers that makes of the tunnel's Round Trip, Loss and
// Traffic, since warp's record holds one reading of each. Warp.js says
// how the link stands; Warp.qml keeps what saw returns and hands drawn
// the parts Warp.told gave. Plain reading, held in Qt's own engine
// (tests/answers.qml).
.pragma library
.import "HomeNetwork.js" as HomeNetwork
.import "Warp.js" as Warp

const PERCENT = 100;
const SECOND = 1000;
// How many reads a spark holds, the newest last.
const KEPT = 20;
// The least a spark's top stands for: 100 ms of round trip, and 5% of
// packets lost.
const LEAST_TRIP = 100;
const LEAST_LOSS = 0.05;
const BITS = 8;
const MEGA = 1e6;

// One more read kept, the oldest giving way past KEPT, a figure warp
// didn't give kept as nothing.
function kept(values, value) {
    return (Number.isFinite(value) ? values.concat([value]) : values).slice(-KEPT);
}

// What the shell has seen of WARP while it's shown, once `record` is read
// at `now`, in milliseconds: its state, the times of its reads, and the
// tunnel's round trips, losses and the speed each way between two reads
// in bytes a second, the newest last, as Wi-Fi keeps its signal. A state
// changed starts them again, and a counter gone back, as a link made
// again starts its own, gives no speed for that read.
function saw(seen, record, now) {
    const state = String(record?.status?.state ?? "");
    const tunnel = record?.tunnel ?? {};
    const same = seen && seen.state === state ? seen : null;
    const before = same ?? { times: [], latency: [], loss: [], inward: [], outward: [] };
    const seconds = same ? (now - same.at) / SECOND : 0;
    const gone = (key, was) => {
        const value = tunnel[key];
        return seconds > 0 && Number.isFinite(value) && Number.isFinite(was) && value >= was
            ? (value - was) / seconds : null;
    };
    const inward = gone("received_bytes", same?.received);
    const outward = gone("sent_bytes", same?.sent);
    const both = inward !== null && outward !== null;
    return {
        state,
        at: now,
        received: tunnel.received_bytes ?? null,
        sent: tunnel.sent_bytes ?? null,
        times: kept(before.times, now),
        latency: kept(before.latency, tunnel.latency_ms),
        loss: kept(before.loss, tunnel.loss_fraction),
        inward: both ? kept(before.inward, inward) : before.inward,
        outward: both ? kept(before.outward, outward) : before.outward,
    };
}

// A spark of reads, each against the most of them or `least`, whichever
// is more, so a calm link's jitter draws small.
function sparkOf(values, least) {
    const most = Math.max(least, ...values);
    return { shape: "spark", values: values.map((value) => (most > 0 ? value / most : 0)),
        warn: false };
}

// A rate in bytes a second, as Network's speed says one.
function rateOf(bytes, settings) {
    return HomeNetwork.rate(bytes * BITS / MEGA, settings);
}

// The tunnel's parts of `parts` (Warp.told's) with a history, sparked,
// each with a drawer whose eyebrow says once whose figures they are ("via
// WARP \u00b7 tunnel", VKCF3Z, 2026-11-08): Round Trip and Loss their
// range over the reads and how many, Traffic its speed each way now. One
// read sparks nothing, so no drawer says only what the row does.
function drawn(parts, seen, settings, said) {
    if (!seen) {
        return parts;
    }
    const eyebrow = Warp.worded(said, "eyebrow");
    const times = seen.times ?? [];
    const span = times.length > 1 ? (times[times.length - 1] - times[0]) / SECOND : 0;
    const reads = (values) => ({ id: "reads", value: Warp.worded(said, "reads", {
        count: Warp.count(values.length, settings),
        span: Warp.worded(said, "seconds", { figure: Warp.count(Math.round(span), settings) }),
    }) });
    const ranged = (values, least, say) => {
        const spark = sparkOf(values, least);
        return { graph: spark, drawer: { eyebrow, facts: [{ id: "range", value: Warp.worded(said,
            "range", { low: say(Math.min(...values)), high: say(Math.max(...values)) }),
        figure: spark }, reads(values)] } };
    };
    const percent = (value) => Warp.worded(said, "percent",
        { figure: HomeNetwork.significant(value * PERCENT, settings) });
    const made = {
        latency: seen.latency.length > 1
            ? ranged(seen.latency, LEAST_TRIP, (value) => Warp.ms(value, settings, said)) : null,
        loss: seen.loss.length > 1 ? ranged(seen.loss, LEAST_LOSS, percent) : null,
        traffic: seen.inward.length > 1 ? {
            graph: sparkOf(seen.inward.map((value, at) => value + seen.outward[at]), 0),
            drawer: { eyebrow, facts: [
                { id: "in", value: rateOf(seen.inward[seen.inward.length - 1], settings),
                    figure: sparkOf(seen.inward, 0) },
                { id: "out", value: rateOf(seen.outward[seen.outward.length - 1], settings),
                    figure: sparkOf(seen.outward, 0) },
            ] },
        } : null,
    };
    return parts.map((each) => (made[each.id] ? Object.assign(each, made[each.id]) : each));
}
