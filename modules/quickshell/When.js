// The day something was on, and when it was, said the one way the
// launcher says when (Y2YBTS, 2026-11-03), so a file's row, a call's, a
// mail's and a meeting's read alike. Plain JavaScript, held to it through
// FileFacts.js by tests/file-facts.qml, through google's Mail.js and
// Calendar.js by tests/mail.qml and tests/calendar.qml, and through
// call-transcripts' Transcripts.js by its tests/transcripts.qml.
.pragma library

const MINUTE = 60_000;
const HOUR = 3_600_000;
const DAY = 86_400_000;
// How many days back, or on, a day is said by its weekday.
const LATELY_DAYS = 6;
// How wide a clock's hours and minutes are written.
const DIGITS = 2;

function midnight(date) {
    return new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
}

// How many days before `now` a moment's day is, by the calendar, so a
// day an hour short or long, as summer time makes, still counts as one.
function daysBefore(at, now) {
    return Math.round((midnight(now) - midnight(at)) / DAY);
}

// The day it was or will be on, both in milliseconds: nothing today,
// which the one saying it says as a time; "yesterday" or "tomorrow"; its
// weekday within six days either way; this year, "28 Oct"; else "28 Oct
// 2024". A day runs forward as it runs back, so a deadline reads as a
// file's day does (VKCF3Z, 2026-11-08). `said` gives the words: its
// `yesterday` and `tomorrow`, and `date(date, format)`, Qt's `formatDate`
// in the shell, so days and months are named as the locale names them. One
// that gives no `tomorrow` says tomorrow by its weekday.
function day(time, now, said) {
    const at = new Date(time);
    const today = new Date(now);
    const days = daysBefore(at, today);
    if (days === 0) {
        return "";
    }
    if (days === 1) {
        return said.yesterday;
    }
    if (days === -1 && typeof said.tomorrow === "string") {
        return said.tomorrow;
    }
    if (days !== 0 && Math.abs(days) <= LATELY_DAYS) {
        return said.date(at, "dddd");
    }
    return said.date(at, at.getFullYear() === today.getFullYear() ? "d MMM" : "d MMM yyyy");
}

// A day alone, as a note says when something was: "today", "yesterday",
// "on Thursday" within six days, then "on 28 Oct" and "on 13 Nov 2025",
// as `day` names them. `said` gives `today` and `on`, "on {day}", besides
// what `day` takes; yesterday and tomorrow stand alone, since nobody says
// "on yesterday".
function onDay(time, now, said) {
    const named = day(time, now, said);
    if (named === "") {
        return said.today;
    }
    return named === said.yesterday || named === said.tomorrow ? named
        : String(said.on).replace("{day}", named);
}

// Its time on a clock, "09:05".
function clock(time) {
    const at = new Date(time);
    const two = number => String(number).padStart(DIGITS, "0");
    return `${two(at.getHours())}:${two(at.getMinutes())}`;
}

// When something was or will be, its day and its time: today its time
// alone, "06:06", and on another day its day as `day` says it, then its
// time, "yesterday 14:10", "Wednesday 09:30", "28 Oct 09:30". A meeting
// over says when it was by it, and a drawer's fact says when beside a row
// whose side does, so the launcher says a moment one way (Y2YBTS,
// 2026-11-09).
function moment(time, now, said) {
    const on = day(time, now, said);
    return on === "" ? clock(time) : `${on} ${clock(time)}`;
}

// When something was, as a row says it at its side: "just now" within a
// minute, "12 min ago" within the hour, its time earlier today, "14:02",
// and before today its day as `day` says it. A moment ahead of now, from
// a wrong clock, reads as one behind does: its time today, then its day.
// `said` gives `now` and `minutes` besides what `day` takes, as mail's
// words do; mail says when it came by it, and a meeting over when it was
// held.
function came(time, now, said) {
    const since = now - time;
    if (since >= 0 && since < MINUTE) {
        return said.now;
    }
    if (since >= 0 && since < HOUR) {
        return String(said.minutes).replace("{minutes}", Math.floor(since / MINUTE));
    }
    const on = day(time, now, said);
    return on === "" ? clock(time) : on;
}
