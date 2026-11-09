// The day something was on, said the one way the launcher says when
// (Y2YBTS, 2026-11-03), so a file's row and a call's read alike. Plain
// JavaScript, held to it through FileFacts.js by tests/file-facts.qml.
.pragma library

const DAY = 86_400_000;
// How many days back, or on, a day is said by its weekday.
const LATELY_DAYS = 6;

function midnight(moment) {
    return new Date(moment.getFullYear(), moment.getMonth(), moment.getDate()).getTime();
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
