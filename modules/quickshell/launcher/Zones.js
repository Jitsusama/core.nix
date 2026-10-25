// The time somewhere, as the launcher answers it: "time", "time in tokyo",
// "london time", "what time is it in india". Qt's JavaScript knows no time
// zone but the machine's, so each zone's changes come from tzdata at build
// time (zones.py) and are looked up here. Plain JavaScript, so
// tests/answers.qml runs it in Qt's own engine.
.pragma library

const days = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const months = ["January", "February", "March", "April", "May", "June", "July", "August",
    "September", "October", "November", "December"];

// A second, a minute and a day, in milliseconds.
const SECOND = 1000;
const MINUTE = 60_000;
const DAY = 86_400_000;
// What runs between a place's words, or starts its name, said as typed.
const PLACE_GAPS = /[_\s]+/g;
const LEADING_THE = /^the /;
// The question mark a question can end with.
const QUESTION_MARK = /\?$/;
// The time asked for here, the time asked for in a place, and a place's time.
const TIME_HERE = /^(?:time|clock|what time is it|what's the time)$/;
const TIME_IN = /^(?:time|clock|what time is it|what's the time)\s+(?:in|at|for)\s+(.+)$/;
const TIME_AFTER = /^(.+?)\s+time$/;
// An offset tzdata writes where a zone has no abbreviation of its own.
const WRITTEN_OFFSET = /^[+-]\d/;

// The zones from zones.py's lines, and every name each goes by.
function prepare(text) {
    const table = { zones: {}, named: {} };
    for (const line of text.split("\n")) {
        if (line !== "") {
            const [id, names, changes] = JSON.parse(line);
            table.zones[id] = { id, names, changes };
            for (const name of names) {
                table.named[name] = id;
            }
            table.named[id.toLowerCase()] = id;
        }
    }
    return table;
}

// How a zone stands at a moment, in seconds: [minutes east of UTC, its
// abbreviation].
function standing(zone, seconds) {
    let [found] = zone.changes;
    for (const change of zone.changes) {
        if (change[0] > seconds) {
            break;
        }
        found = change;
    }
    return [found[1], found[2]];
}

function said(place) {
    return place.trim().toLowerCase().replace(PLACE_GAPS, " ").replace(LEADING_THE, "");
}

function find(table, place) {
    const id = table.named[said(place)];
    return id ? table.zones[id] : null;
}

// What's asked for: "" for here, a place's name, or null when it isn't a
// question of time. Only "time in" a place may be looked up elsewhere, so
// "bed time" never reaches a map.
function asked(text) {
    const typed = text.trim().toLowerCase().replace(QUESTION_MARK, "");
    if (TIME_HERE.test(typed)) {
        return { place: "", named: true };
    }
    const inside = TIME_IN.exec(typed);
    if (inside) {
        return { place: inside[1], named: true };
    }
    const before = TIME_AFTER.exec(typed);
    if (before) {
        return { place: before[1], named: false };
    }
    return null;
}

function two(number) {
    return String(number).padStart(2, "0");
}

function clock(date) {
    return `${two(date.getUTCHours())}:${two(date.getUTCMinutes())}`;
}

function day(date) {
    return `${days[date.getUTCDay()]} ${date.getUTCDate()} ${months[date.getUTCMonth()]}`;
}

// Which day of its year a local date is, 1 for the first of January,
// counted in UTC, since local midnights either side of a change to or from
// summer time are an hour more or less than whole days apart.
function dayOfYear(date) {
    return (Date.UTC(date.getFullYear(), date.getMonth(), date.getDate())
        - Date.UTC(date.getFullYear(), 0, 1)) / DAY + 1;
}

// "UTC+5:45", "UTC−4", "UTC".
function offset(minutes) {
    if (minutes === 0) {
        return "UTC";
    }
    const sign = minutes > 0 ? "+" : "\u2212";
    const whole = Math.abs(minutes);
    return `UTC${sign}${Math.floor(whole / 60)}${whole % 60 ? `:${two(whole % 60)}` : ""}`;
}

// "13 h ahead of here", "4 h 30 min behind here", "the same time as here".
function apart(minutes) {
    if (minutes === 0) {
        return "the same time as here";
    }
    const whole = Math.abs(minutes);
    const hours = Math.floor(whole / 60);
    const parts = [hours ? `${hours} h` : "", whole % 60 ? `${whole % 60} min` : ""];
    const way = minutes > 0 ? "ahead of" : "behind";
    return `${parts.filter(part => part).join(" ")} ${way} here`;
}

// Each row's glyph, as the theme's mark for what it tells: the day, the
// gap, the zone. The day wears the date's, as Today's rows do.
const glyphs = { day: "date", apart: "clock", zone: "zone" };

// Which day it is there against here, by how many days apart they are.
function relative(dayApart) {
    if (dayApart > 0) {
        return "tomorrow";
    }
    return dayApart < 0 ? "yesterday" : "today";
}

// The time in a zone at `now` (milliseconds), told against `here`, the
// machine's own zone or null, whose offset is otherwise `local` minutes.
// biome-ignore lint/complexity/useMaxParams: Clocks.qml and tests/answers.qml call it so.
function tell(zone, now, here, local, name) {
    const seconds = Math.floor(now / SECOND);
    const [minutes, abbreviation] = standing(zone, seconds);
    const mine = here ? standing(here, seconds)[0] : local;
    const there = new Date(now + minutes * MINUTE);
    const ours = new Date(now + mine * MINUTE);
    const dayApart = Math.round((Date.UTC(there.getUTCFullYear(), there.getUTCMonth(),
        there.getUTCDate()) - Date.UTC(ours.getUTCFullYear(), ours.getUTCMonth(),
        ours.getUTCDate())) / DAY);
    const isHere = here !== null && zone.id === here.id;
    // Which day it is there only says something when it's another place.
    const which = isHere ? "" : relative(dayApart);
    const rows = [{ text: day(there), side: which, copy: day(there), glyph: glyphs.day }];
    if (!isHere) {
        rows.push({ text: apart(minutes - mine), side: "", copy: offset(minutes),
            glyph: glyphs.apart });
    }
    // tzdata writes an offset where a zone has no abbreviation of its own,
    // and a name the row already says is left out.
    const names = [abbreviation, offset(minutes)]
        .filter(part => part !== "" && !WRITTEN_OFFSET.test(part) && part !== zone.id);
    const unique = names.filter((part, i) => names.indexOf(part) === i);
    if (zone.id !== "") {
        rows.push({ text: zone.id, side: unique.join(" \u00b7 "), copy: zone.id,
            glyph: glyphs.zone });
    }
    return {
        headline: clock(there),
        copy: clock(there),
        rows,
        place: name ?? (isHere ? "here" : zone.names[0])
    };
}

// The answer to what's typed, an { unknown } place for the caller to look
// up when it was named as one, or null when it isn't about the time.
// biome-ignore lint/complexity/useMaxParams: Clocks.qml and tests/answers.qml call it so.
function answer(text, table, now, hereId, local) {
    const question = asked(text);
    if (!question) {
        return null;
    }
    const here = hereId ? table.zones[hereId] ?? null : null;
    if (question.place === "") {
        return here ? tell(here, now, here, local) : tell({ id: "", names: ["here"],
            changes: [[0, local, ""]] }, now, null, local, "here");
    }
    const zone = find(table, question.place);
    if (zone) {
        return tell(zone, now, here, local);
    }
    return question.named ? { unknown: question.place } : null;
}
