// What Timers works out apart from drawing, in plain JavaScript so
// tests/timing.qml holds it to its cases in Qt's own engine, as the shell
// runs it: the span and name typed after "timer", how long a timer has
// left as it stands, and the time left and a span as the launcher says
// them, and the stopwatch's count and laps. Every time is milliseconds; a
// record is the timers daemon's (modules/quickshell/timers/src/lib.rs).
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
    const split = parts(span);
    const words = ["hours", "minutes", "seconds"].filter(unit => split[unit] > 0)
        .map(unit => `${split[unit]} ${units[unit]}`);
    return words.length > 0 ? words.join(" ") : `0 ${units.seconds}`;
}

// A time of day as the launcher says one, "20:42".
function timeOfDay(at) {
    const day = new Date(at);
    const two = number => String(number).padStart(2, "0");
    return `${two(day.getHours())}:${two(day.getMinutes())}`;
}

// What a timer says at its right at `now`, in `words`, its own: the
// time left, held and said so while paused, or when it rang, so every
// row says how it stands (VKCF3Z, 2026-11-07), in Timers and Now alike.
function side(record, now, words) {
    const counting = clock(left(record, now));
    switch (record.state) {
    case "running":
        return counting;
    case "paused":
        return (words.pausedSide ?? "").replace("{left}", counting);
    default:
        return ((record.late ? words.late : words.rung) ?? "").replace("{clock}",
            timeOfDay(record.rang));
    }
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

const TENTH = SECOND / 10;
const TENTHS = SECOND / TENTH;

// What the stopwatch has counted at `now`: what it held, and the stretch
// since it was last set going while it runs.
function counted(watch, now) {
    const since = watch?.since;
    return (watch?.counted ?? 0) + (since === undefined ? 0 : Math.max(0, now - since));
}

// A count as the stopwatch shows it, to the tenth: "0:42.1", and
// "1:02:03.4" past an hour. It rounds down, since a stopwatch never shows
// time that hasn't passed yet, where a timer's clock rounds up.
function tenths(span) {
    const all = Math.floor(Math.max(0, span) / TENTH);
    const seconds = Math.floor(all / TENTHS);
    const hours = Math.floor(seconds / (SECONDS * SECONDS));
    const minutes = Math.floor(seconds / SECONDS) % SECONDS;
    const two = number => String(number).padStart(2, "0");
    const shown = `${two(seconds % SECONDS)}.${all % TENTHS}`;
    return hours > 0 ? `${hours}:${two(minutes)}:${shown}` : `${minutes}:${shown}`;
}

// What Now says of the timers and the stopwatch at `now`
// (pieces/Timers.qml): whether any is on, a timer running, paused or
// rung, so none is forgotten, or the stopwatch running; whether one has
// rung; the timer its side tells of, the first as Timers lists them,
// so one rung, else the soonest to ring, else the paused one with least
// left, or null where the stopwatch alone runs; and what it names, the
// timers in that order and the running stopwatch last (VKCF3Z,
// 2026-11-08).
function onNow(timers, watch, now) {
    const listed = ordered(timers, now);
    const counting = watch?.state === "running";
    return {
        on: listed.length > 0 || counting,
        rung: listed.some(record => record.state === "rung"),
        first: listed[0] ?? null,
        named: counting ? listed.concat([watch]) : listed
    };
}

// What Now's Timers row says at its right, as items (VKCF3Z, 5fa2cbda):
// with one on, what that one's own row in Timers says, "3:12" or
// "paused \u00b7 9:50"; with several, how many run and the first's,
// "2 running \u00b7 3:12", as Agents says "3 running". A first that rang
// or is held leads, since its state is the news, and how many still run
// follows: "rang 21:20 \u00b7 1 running", or "rang 21:20" alone with none
// running (VKCF3Z, 158ac71f). The count gives way; the figure never does.
// `words` are the timer's, `several` among them.
function nowItems(standing, watch, now, words) {
    const first = standing.first;
    const figure = {
        said: first !== null ? side(first, now, words) : tenths(counted(watch, now)),
        stays: true
    };
    const running = standing.named.filter(record => record.state === "running").length;
    const count = { said: String(words.several ?? "").replace("{count}", String(running)) };
    if (first !== null && first.state !== "running") {
        return running > 0 ? [figure, count] : [figure];
    }
    return standing.named.length > 1 && running > 0 ? [count, figure] : [figure];
}

// Whether the stopwatch alone runs, with no timer on: then Now's row
// counts it to the tenth, drawn afresh by the launcher from its side
// (Launcher.qml's counts), so it gives no items, which would stand in
// for that side and stop the count (Y2YBTS, 89588a65).
function watchAlone(standing, watch) {
    return standing.first === null && watch?.state === "running";
}

// Whether Now's row must be told the time each second: only while the
// launcher is open, where its side is drawn, and a timer counts down then.
// The margin wears only the mark, which says how things stand and not
// the time, so a clock ticking with the launcher closed would wake the
// shell each second for nothing (Y2YBTS, 2026-11-08); and the stopwatch
// alone is drawn afresh by the launcher, to the tenth, so Now's rows
// aren't made again each second for it.
function ticks(timers, shown) {
    return shown && timers.some(record => record.state === "running");
}

// A figure more, as a count grows one.
const FIGURE = 10;

// The count at which the stopwatch's next grows a figure: ten minutes, an
// hour, then ten hours, a hundred and on.
function grows(span) {
    if (span < FIGURE * MINUTE) {
        return FIGURE * MINUTE;
    }
    let at = HOUR;
    while (span >= at) {
        at *= FIGURE;
    }
    return at;
}

// The widest the stopwatch's count reads until it next grows a figure,
// "9:59.9" below ten minutes and "59:59.9" below an hour, so the room
// its side keeps holds every count of its range (Y2YBTS, 2026-11-08).
function widest(span) {
    return tenths(grows(span) - TENTH);
}

// The stopwatch's laps, newest first, each { number, lap, at }: its
// number from the first, its own stretch, and the count when it was
// marked.
function laps(watch) {
    const marked = watch?.laps ?? [];
    return marked.map((at, index) => ({
        number: index + 1,
        lap: at - (index > 0 ? marked[index - 1] : 0),
        at
    })).reverse();
}
