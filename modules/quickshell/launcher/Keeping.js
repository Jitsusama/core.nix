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
