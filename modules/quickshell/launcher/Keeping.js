// How Time words chrony's keeping, as timekeeping reads it from chronyd
// (timekeeping/src/service.rs): its seconds in the unit that reads them,
// its frequencies, its spans, and where each server stands and why. Kept
// apart from Clocks.qml so each figure's wording is tested on its own
// (tests/answers.qml). Every word comes from the vocabulary, passed in as
// `said`.
.pragma library

const MINUS = "\u2212";

const NANOSECONDS = 1e9;
const MICROSECONDS = 1e6;
const MILLISECONDS = 1e3;
// A unit reads a figure until it would be a thousand of it.
const NEXT_UNIT = 1000;
// Seconds and parts per million are said to a thousandth, as chronyc says
// them.
const THOUSANDTHS = 3;
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

// Seconds in the unit that reads them: nanoseconds and microseconds whole,
// milliseconds to a tenth, seconds to a thousandth. Signed, a figure says
// "+" or a true minus.
function seconds(value, signed) {
    const size = Math.abs(value);
    const marked = signed && size !== 0 ? sign(value) : "";
    if (size === 0) {
        return "0 s";
    }
    if (Math.round(size * NANOSECONDS) < NEXT_UNIT) {
        return `${marked}${Math.round(size * NANOSECONDS)} ns`;
    }
    if (Math.round(size * MICROSECONDS) < NEXT_UNIT) {
        return `${marked}${Math.round(size * MICROSECONDS)} \u00b5s`;
    }
    if (Number((size * MILLISECONDS).toFixed(1)) < NEXT_UNIT) {
        return `${marked}${(size * MILLISECONDS).toFixed(1)} ms`;
    }
    return `${marked}${size.toFixed(THOUSANDTHS)} s`;
}

// Parts per million, to a thousandth, as chronyc says them.
function ppm(value, signed) {
    return `${signed ? sign(value) : ""}${Math.abs(value).toFixed(THOUSANDTHS)} ppm`;
}

// A span of seconds as hours, minutes and seconds, the largest two that
// say it; under a second, to a tenth.
function span(value) {
    if (value < 1) {
        return `${Number(value.toFixed(1))} s`;
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
    return ["0 s", "0 ns"].includes(seconds(value, false));
}

// How far a clock is out, from how far ahead of true time it is: "12 µs
// fast", "0.4 ms slow", or on time.
function out(ahead, said) {
    if (none(ahead)) {
        return said.onTime;
    }
    return (ahead > 0 ? said.fast : said.slow).replace("{offset}", seconds(ahead, false));
}

// How fast a clock runs, from its parts per million fast: "6.267 ppm
// fast", "3.225 ppm slow".
function runs(fast, said) {
    if (Math.abs(fast) < STILL) {
        return said.onTime;
    }
    return (fast > 0 ? said.fast : said.slow).replace("{offset}", ppm(fast, false));
}

// What a clock gaining `gain` parts per million does in a day.
function daily(gain, said) {
    const day = (gain * DAY) / PER_MILLION;
    if (none(day)) {
        return said.onTime;
    }
    return (day > 0 ? said.gains : said.loses).replace("{seconds}", seconds(day, false));
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

// The selection's score, to a tenth, as chronyc selectdata says it.
function score(value) {
    return value.toFixed(1);
}

// How lopsided a server's delay is, from -0.5 to +0.5, to a hundredth
// and signed, as chronyc ntpdata says it.
function asymmetry(value) {
    return `${sign(value)}${Math.abs(value).toFixed(2)}`;
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
