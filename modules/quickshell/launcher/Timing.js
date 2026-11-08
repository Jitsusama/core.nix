// What Timers works out apart from drawing, in plain JavaScript so
// tests/timing.qml holds it to its cases in Qt's own engine, as the shell
// runs it: the span and name typed after "timer", how long a timer has
// left as it stands, and the time left and a span as the launcher says
// them. Every time is milliseconds; a record is the timers daemon's
// (modules/quickshell/timers/src/lib.rs).
.pragma library

const SECOND = 1000;
const SECONDS = 60;
const MINUTE = SECONDS * SECOND;
const HOUR = SECONDS * MINUTE;
const HOURS_A_WEEK = 7 * 24;
// The longest span the daemon takes, a week.
const LONGEST = HOURS_A_WEEK * HOUR;

// Each unit a span is typed in, by the words that name it, longest first
// so "min" isn't read as "m" and then "in".
const UNITS = [
    [/^(hours?|hrs?|h)/, HOUR],
    [/^(minutes?|mins?|m)/, MINUTE],
    [/^(seconds?|secs?|s)/, SECOND]
];
// A number, whole or with a fraction.
const NUMBER = /^(\d+(?:\.\d+)?)\s*/;
// A clock's span, "1:30" as minutes and seconds or "1:02:03" with hours.
const CLOCK = /^(\d+):([0-5]\d)(?::([0-5]\d))?(?=\s|$)/;
// A number run into a colon that no clock reads, as "1:75".
const NOT_A_CLOCK = /^\d+:\S*/;
// What may follow a unit: nothing, a space or the next number.
const AFTER_UNIT = /^[\s\d]/;
const SPACES = /\s+/;

// A clock's span, { span, rest }, or null where `text` doesn't start
// with one. One no clock reads, as "1:75", is a span of nothing, so the
// answer says it's unfit rather than starting a minute named ":75".
function clockAt(text) {
    const matched = CLOCK.exec(text);
    if (!matched) {
        const unread = NOT_A_CLOCK.exec(text);
        return unread ? { span: 0, rest: text.slice(unread[0].length).trim() } : null;
    }
    const [whole, first, second, third] = matched;
    const span = third === undefined
        ? Number(first) * MINUTE + Number(second) * SECOND
        : Number(first) * HOUR + Number(second) * MINUTE + Number(third) * SECOND;
    return { span, rest: text.slice(whole.length).trim() };
}

// One number and its unit from the start of `text`, { span, rest, size },
// or null where it doesn't start with one; `size` is its unit's, or null
// for a bare number, which is minutes. A unit runs into nothing after it
// but a space or the next number, so in "10 mussels" the m is the name's
// and the number is bare.
function partAt(text) {
    const number = NUMBER.exec(text);
    if (!number) {
        return null;
    }
    const after = text.slice(number[0].length);
    const unit = UNITS.find(([words]) => words.test(after));
    const word = unit ? unit[0].exec(after)[0] : "";
    const ended = after.slice(word.length);
    if (!unit || (ended !== "" && !AFTER_UNIT.test(ended))) {
        return { span: Number(number[1]) * MINUTE, rest: after.trim(), size: null };
    }
    return { span: Number(number[1]) * unit[1], rest: ended.trim(), size: unit[1] };
}

// A span read from the start of `text`, { span, rest }, or null where it
// doesn't start with one. A bare number is minutes, as a kitchen timer's
// is, and each further number and unit adds to it: "1h 30m".
function spanAt(text) {
    const clocked = clockAt(text);
    if (clocked) {
        return clocked;
    }
    let span = 0;
    let rest = text;
    let last;
    for (;;) {
        const part = partAt(rest);
        // A bare number begins a span, or ends one after hours, as it's
        // said aloud: "1h 30" is ninety minutes, but the "2" of "10m 2
        // eggs" is the name's.
        const bare = part !== null && part.size === null;
        if (part === null || (last !== undefined && bare && last !== HOUR)) {
            break;
        }
        span += part.span;
        rest = part.rest;
        last = part.size;
        if (bare) {
            break;
        }
    }
    return last === undefined ? null : { span: Math.round(span), rest };
}

// The span after a name: the last words that read as one whole, as
// { span, name }, or null.
function spanAfter(typed, original) {
    const words = typed.split(SPACES);
    for (let at = 1; at < words.length; at += 1) {
        const span = spanAt(words.slice(at).join(" "));
        if (span && span.rest === "") {
            return { span: span.span, name: original.split(SPACES).slice(0, at).join(" ") };
        }
    }
    return null;
}

// What's typed after the command's word, read as a span and a name:
// "10m tea", "tea 10m", "1:30", "1h 30m pasta". Null when it holds no
// span; a span of nothing or past a week reads as `unfit`, for the answer
// to say why.
function read(text) {
    const original = String(text ?? "").trim();
    const typed = original.toLowerCase();
    if (typed === "") {
        return null;
    }
    const first = spanAt(typed);
    const found = first
        ? { span: first.span, name: original.slice(original.length - first.rest.length).trim() }
        : spanAfter(typed, original);
    if (!found) {
        return null;
    }
    return {
        span: found.span,
        name: found.name,
        unfit: found.span <= 0 || found.span > LONGEST
    };
}

// How long a timer has left at `now`, as it stands.
function left(record, now) {
    if (record.state === "running") {
        return Math.max(0, record.ends - now);
    }
    if (record.state === "paused") {
        return record.left;
    }
    return 0;
}

// A span in whole seconds, counted up, and its hours, minutes and seconds.
function parts(span) {
    const seconds = Math.ceil(Math.max(0, span) / SECOND);
    return {
        hours: Math.floor(seconds / (SECONDS * SECONDS)),
        minutes: Math.floor(seconds / SECONDS) % SECONDS,
        seconds: seconds % SECONDS
    };
}

// The time left as a clock reads it, "4:12", or "1:04:12" past an hour,
// counting whole seconds up, so a ten minute timer starts at "10:00" and
// shows "0:01" in its last second.
function clock(span) {
    const { hours, minutes, seconds } = parts(span);
    const two = number => String(number).padStart(2, "0");
    return hours > 0 ? `${hours}:${two(minutes)}:${two(seconds)}` : `${minutes}:${two(seconds)}`;
}

// A span in its largest units, as the daemon's notice says it: "10 min",
// "1 h 30 min", "1 min 30 s", each unit's word from `units`, { hours,
// minutes, seconds }.
function said(span, units) {
    const counted = parts(span);
    const words = ["hours", "minutes", "seconds"].filter(unit => counted[unit] > 0)
        .map(unit => `${counted[unit]} ${units[unit]}`);
    return words.length > 0 ? words.join(" ") : `0 ${units.seconds}`;
}

// How timers stand in the list, rung first, since they want Joel.
const RANKS = { rung: 0, running: 1, paused: 2 };

// How timers are listed: those rung first, newest first, then those
// running by the soonest end, then those paused by the least left, each
// by when it was set where the rest are even.
function ordered(records, now) {
    const rank = record => RANKS[record.state] ?? Object.keys(RANKS).length;
    const by = (one, other) => (one.state === "rung" ? other.rang - one.rang
        : left(one, now) - left(other, now));
    return records.slice().sort((one, other) => rank(one) - rank(other) || by(one, other)
        || one.set - other.set);
}
