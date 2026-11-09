// How Time words chrony's keeping, as timekeeping reads it from chronyd
// (timekeeping/src/service.rs): its seconds in the unit that reads them,
// its frequencies, its spans, and where each server stands and why. Kept
// apart from Clocks.qml so each figure's wording is tested on its own
// (tests/answers.qml). Every word comes from the vocabulary, passed in as
// `said`, and every measured figure keeps three significant digits, its
// trailing zeros too, in the settings' decimal mark, `point` (VKCF3Z,
// 2026-11-07, as Machine.js and Screens.js keep them).
.pragma library
.import "Numbers.js" as Numbers

const MINUS = "\u2212";
// The standings whose own words say why chronyd took the server or not,
// each with the reason it says. A reason beyond it, a server disagreeing
// with the trusted ones rather than the others, still adds something
// (Y2YBTS's review of #2489).
const WHY_EACH_STANDING_SAYS = { chosen: "chosen", tooVariable: "tooVariable",
    disagrees: "disagrees" };

const NANOSECONDS = 1e9;
const MICROSECONDS = 1e6;
const MILLISECONDS = 1e3;
// A unit reads a figure until it would be a thousand of it.
const NEXT_UNIT = 1000;
const SIGNIFICANT = 3;
// Under half a nanosecond, chronyd's finest, reads as none.
const FINEST = 0.5;
const MINUTE = 60;
const HOUR = 3600;
const DAY = 86_400;
const PER_MILLION = 1e6;
// Under half a thousandth of a part per million reads as on time.
const STILL = 0.0005;
// chronyd's reach register holds the last eight polls, a bit each.
const POLLS = 8;

function sign(value) {
    return value < 0 ? MINUS : "+";
}

// A size to three significant digits, its trailing zeros kept, in the
// decimal mark `point`, as every measured figure is said (Numbers.js):
// "915", "1.52", "0.0400". A unit reads under a thousand of it, so none
// is grouped.
function digits(size, point) {
    return Numbers.significant(size, "", point);
}

// Whether a size reads as a thousand or more in its unit, once it's
// rounded to three digits, so the next unit reads it.
function past(size) {
    return Number(size.toPrecision(SIGNIFICANT)) >= NEXT_UNIT;
}

// Seconds in the unit that reads them, to three significant digits: "29.8
// ns", "388 \u00b5s", "36.1 ms", "1.50 s". Signed, a figure says "+" or a
// true minus.
function seconds(value, signed, point) {
    const size = Math.abs(value);
    if (size * NANOSECONDS < FINEST) {
        return "0 s";
    }
    const marked = signed ? sign(value) : "";
    const units = [[NANOSECONDS, "ns"], [MICROSECONDS, "\u00b5s"], [MILLISECONDS, "ms"]];
    const [scale, unit] = units.find(([each]) => !past(size * each)) ?? [1, "s"];
    return `${marked}${digits(size * scale, point)} ${unit}`;
}

// Parts per million, to three significant digits.
function ppm(value, signed, point) {
    return `${signed ? sign(value) : ""}${digits(Math.abs(value), point)} ppm`;
}

// A span of seconds as hours, minutes and seconds, the largest two that
// say it; under a second, to a tenth.
function span(value, point) {
    if (value < 1) {
        return `${String(Number(value.toFixed(1))).replace(".", point ?? ".")} s`;
    }
    const whole = Math.round(value);
    const hours = Math.floor(whole / HOUR);
    const minutes = Math.floor((whole % HOUR) / MINUTE);
    const left = whole % MINUTE;
    if (hours > 0) {
        return minutes === 0 ? `${hours} h` : `${hours} h ${minutes} min`;
    }
    if (minutes > 0) {
        return left === 0 ? `${minutes} min` : `${minutes} min ${left} s`;
    }
    return `${left} s`;
}

// Whether seconds read as none in the unit that reads them.
function none(value) {
    return seconds(value, false) === "0 s";
}

// How far a clock is out, from how far ahead of true time it is: "12.0 µs
// fast", "412 µs slow", or on time; in the decimal mark `said.point`.
function out(ahead, said) {
    if (none(ahead)) {
        return said.onTime;
    }
    return (ahead > 0 ? said.fast : said.slow)
        .replace("{offset}", seconds(ahead, false, said.point));
}

// How fast a clock runs, from its parts per million fast: "6.27 ppm
// fast", "3.23 ppm slow".
function runs(fast, said) {
    if (Math.abs(fast) < STILL) {
        return said.onTime;
    }
    return (fast > 0 ? said.fast : said.slow).replace("{offset}", ppm(fast, false, said.point));
}

// What a clock gaining `gain` parts per million does in a day.
function daily(gain, said) {
    const day = (gain * DAY) / PER_MILLION;
    if (none(day)) {
        return said.onTime;
    }
    return (day > 0 ? said.gains : said.loses)
        .replace("{seconds}", seconds(day, false, said.point));
}

// The leap second announced.
function leap(leaping, said) {
    return said[`leap:${leaping}`] ?? "";
}

// How many of the last eight polls answered, each poll a bit of the
// register, the latest lowest.
function answered(register) {
    let count = 0;
    let left = register;
    for (let poll = 0; poll < POLLS; poll += 1) {
        count += left % 2;
        left = Math.floor(left / 2);
    }
    return count;
}

// How many of the last eight polls a server answered, in words.
function reach(register, said) {
    return said.reach.replace("{count}", String(answered(register)));
}

// Seconds until a server is next asked, from how often it's asked and how
// long since its last sample: never less than none.
function nextPoll(server) {
    return Math.max(0, server.poll - (server.lastSample ?? 0));
}

// Where a server stands, in a few words for its note: chosen, agreeing,
// set aside and why, or out of reach when it's answered none of its last
// eight polls.
function standing(server, said) {
    if (server.standing === "unusable") {
        if (answered(server.reach) === 0) {
            return said.unreachable;
        }
        const reason = why(server, said);
        return reason ? said.aside.replace("{why}", reason) : said.unusable;
    }
    return said[`standing:${server.standing}`] ?? "";
}

// Why chronyd chose a server or set it aside, in a sentence's words.
function why(server, said) {
    return server.chosen ? (said[`why:${server.chosen.why}`] ?? "") : "";
}

// Whether a server's standing already says why chronyd chose it or set it
// aside: chosen, too variable or disagreeing for that reason, or set aside
// with its reason after it.
function standingSaysWhy(server) {
    if (server.standing === "unusable") {
        return answered(server.reach) > 0;
    }
    const reason = server.chosen?.why;
    return reason !== undefined && reason === WHY_EACH_STANDING_SAYS[server.standing];
}

// Why, as a server's drawer says it: only where its note doesn't, so no
// fact repeats the note (VKCF3Z, 2026-11-08: "the server drawer's Why goes
// when the side says chosen").
function whyUnsaid(server, said) {
    return standingSaysWhy(server) ? "" : why(server, said);
}

// A server's parts as its drawer says them beside its row: the one the
// row's side is made from, its offset, says only its note, and goes with
// none, so no fact says the side again (Y2YBTS's review of #2489). A
// figure that only happens to read the same is another fact, and stays.
function besideSide(parts, fromId) {
    return parts
        .map(each => (each.id === fromId ? Object.assign({}, each, { side: each.note, note: "" })
            : each))
        .filter(each => each.side !== "");
}

// Whether a server has answered at all, so has an offset to say.
function sampled(server) {
    return server.lastSample !== null && server.lastSample !== undefined;
}

// A server's side, never empty (VKCF3Z, 2026-11-08): its last offset, or
// that it isn't answering, or that no sample's in yet from one that is.
function side(server, said) {
    if (sampled(server)) {
        return seconds(server.offset, true, said.point);
    }
    return answered(server.reach) === 0 ? said.silent : said.unsampled;
}

// A server's note: where it stands, unless its side already says it's out
// of reach.
function note(server, said) {
    return !sampled(server) && answered(server.reach) === 0 ? "" : standing(server, said);
}

// A figure as a plate says it, its note after its side as an item of its
// own, so the plate holds what the step in does: "+3 µs · last update
// −1 µs" (Y2YBTS's review of #2303).
function told(part) {
    return part.note ? `${part.side} \u00b7 ${part.note}` : part.side;
}

// What chronyd asks a server as: a server, a peer or a reference clock.
function mode(server, said) {
    return said[`kind:${server.kind}`] ?? "";
}

// How a server's last exchange went: interleaved or basic, and whether it
// was authenticated, as NTS or a key would make it (VKCF3Z, 2026-11-07:
// runs of items join with dots).
function exchanged(exchange, said) {
    return said.exchange
        .replace("{mode}", exchange.interleaved ? said.interleaved : said.basic)
        .replace("{authentication}", exchange.authenticated ? said.authenticated
            : said.unauthenticated);
}

// A server's packets, saying only what differs: "13 sent, all good" while
// every one sent came back valid and good, else each count (VKCF3Z).
function packets(exchange, said) {
    const counts = [exchange.received, exchange.valid, exchange.good];
    if (counts.every(count => count === exchange.sent)) {
        return said.allGood.replace("{sent}", String(exchange.sent));
    }
    return said.packets.replace("{sent}", String(exchange.sent))
        .replace("{received}", String(exchange.received))
        .replace("{valid}", String(exchange.valid))
        .replace("{good}", String(exchange.good));
}

// The selection's score, a measure as every figure is, or "" where
// chronyd has none, as serde_json writes a NaN as null.
function score(value, point) {
    return typeof value === "number" && Number.isFinite(value) ? digits(value, point) : "";
}

// How lopsided a server's delay is, from -0.5 to +0.5, signed but at none.
function asymmetry(value, point) {
    return value === 0 ? "0" : `${sign(value)}${digits(Math.abs(value), point)}`;
}

// What chronyd's activity says of the servers beyond how many there are:
// each count that isn't none, offline, in a burst and unresolved, so a
// part with nothing in it gives way (VKCF3Z, 2026-11-07).
function activity(counts, said) {
    if (!counts) {
        return [];
    }
    return [
        [counts.offline, said.offline],
        [counts.burstOnline + counts.burstOffline, said.burst],
        [counts.unresolved, said.unresolved]
    ].filter(([count]) => count > 0)
        .map(([count, words]) => words.replace("{count}", String(count)));
}
