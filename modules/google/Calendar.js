// What the shell works out from the meetings the google program keeps
// (src/state.rs): which meeting is next, when it is in words, whether it's
// about to start and said as a notice, whether Return joins it, how Joel answered it, the
// lines that answer and cancel it (src/ask.rs), and which Meet call a
// window is. Calendar.qml asks these of the meetings Google.qml holds,
// Mail.js asks the day and the time, and the checks run them in Qt's own
// engine (tests/calendar.qml). Times are milliseconds since 1970, as the
// program gives them.
.pragma library

// In milliseconds.
const MINUTE = 60_000;
const DAY = 86_400_000;

// How soon a meeting coming takes over from the one going on: ten minutes.
const TAKES_OVER = 600_000;
// How long before and after its start a meeting is about to start, as
// Now's wants say it (15J7N3's piece): two minutes before, five after.
const BEFORE = 120_000;
const AFTER = 300_000;
// How long before its start a meeting about to start is said as a
// notice, once: a minute.
const HERALDS = 60_000;
// Up to how far ahead "in 12 min" is said rather than the time: an hour.
const SOON = 3_600_000;
// How wide a clock's hours and minutes are written.
const DIGITS = 2;

const DAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August",
    "September", "October", "November", "December"];

// The meetings that could be next: not over, not all day, and not ones
// Joel said no to.
function coming(meetings, now) {
    return meetings.filter(meeting => meeting.end > now && !meeting.allDay
        && meeting.answer !== "no");
}

// The meeting that's next, or null: the one going on, unless another
// starts within ten minutes, which is the one he's about to go to. The
// Today row and Now's wants both name this one, so they never differ.
function next(meetings, now) {
    const open = coming(meetings, now).sort((a, b) => a.start - b.start || a.end - b.end);
    const upcoming = open.find(meeting => meeting.start > now) ?? null;
    // The one going on that started last, the one Joel most likely went to.
    const current = open.reduce((latest, meeting) => meeting.start <= now ? meeting : latest,
        null);
    if (upcoming && (!current || upcoming.start - now <= TAKES_OVER)) {
        return upcoming;
    }
    return current;
}

// Whether `meeting` is about to start: from two minutes before its start
// until five after.
function aboutToStart(meeting, now) {
    return Boolean(meeting) && now >= meeting.start - BEFORE && now <= meeting.start + AFTER;
}

// Whether a meeting about to start is said as a notice by now: from a
// minute before its start until it's no longer about to start.
function heralded(meeting, now) {
    return aboutToStart(meeting, now) && meeting.start - now <= HERALDS;
}

// Whether another notice already told Joel of `meeting`, such as
// Calendar's own page in Chrome: one named for it, come within ten minutes
// of its start, so the shell doesn't say it twice. The shell's own says
// when beside the name ("Standup in 1 min"), so it's never one of them.
function toldElsewhere(meeting, notices) {
    if (!meeting) {
        return false;
    }
    const title = meeting.title.trim().toLowerCase();
    return title !== "" && notices.some(notice => notice.at >= meeting.start - TAKES_OVER
        && notice.summary.trim().toLowerCase() === title);
}

// Whether Return joins a meeting's call rather than showing the meeting:
// it has a call, isn't over or all day, and is on or starts within ten
// minutes. Any further off, he's more likely looking at it, answering it
// or moving it than going to it, so Return never joins one by surprise.
function joinable(meeting, now) {
    return Boolean(meeting) && Boolean(meeting.link) && !meeting.allDay
        && meeting.end > now && meeting.start - now <= TAKES_OVER;
}

function clock(date) {
    const two = n => String(n).padStart(DIGITS, "0");
    return `${two(date.getHours())}:${two(date.getMinutes())}`;
}

// Local midnight before `time`, so days are counted as the clock does.
function midnight(time) {
    const day = new Date(time);
    day.setHours(0, 0, 0, 0);
    return day.getTime();
}

// Whether a meeting is on or starts today, as the Today row shows only
// today's.
function today(meeting, now) {
    return Boolean(meeting) && meeting.start < midnight(now) + DAY && meeting.end > now;
}

// The day `time` is on, as said before its time: nothing for today,
// "tomorrow", or the day of the week.
function dayOf(time, now) {
    const days = Math.round((midnight(time) - midnight(now)) / DAY);
    if (days <= 0) {
        return "";
    }
    return days === 1 ? "tomorrow" : DAYS[new Date(time).getDay()];
}

// When a meeting is, as the Today row says it after its title: "now" while
// it's on, "in 12 min" within the hour, else its time, with "tomorrow" or
// the day of the week in front past today. An all-day meeting is "today",
// "tomorrow" or its day.
function when(meeting, now) {
    const start = new Date(meeting.start);
    const day = dayOf(meeting.start, now);
    if (meeting.allDay) {
        return day === "" ? "today" : day;
    }
    if (meeting.start <= now) {
        return "now";
    }
    if (meeting.start - now < SOON) {
        return `in ${Math.max(1, Math.ceil((meeting.start - now) / MINUTE))} min`;
    }
    return day === "" ? `at ${clock(start)}` : `${day} at ${clock(start)}`;
}

// A meeting's times, as its row says them: "14:00 to 14:30", or "all day".
function span(meeting) {
    if (meeting.allDay) {
        return "all day";
    }
    return `${clock(new Date(meeting.start))} to ${clock(new Date(meeting.end))}`;
}

// A meeting's day and times in full, as its details say them: "Monday 26
// October, 14:00 to 14:30", or "Wednesday 28 October, all day".
function dated(meeting) {
    const day = new Date(meeting.start);
    return `${DAYS[day.getDay()]} ${day.getDate()} ${MONTHS[day.getMonth()]}, ${span(meeting)}`;
}

// How Joel answered an invitation, as his calendar would say it to him,
// or nothing for a meeting that isn't one, as one he holds.
const ANSWERED = { yes: "going", maybe: "maybe", no: "not going", waiting: "not answered" };

function answered(meeting) {
    return ANSWERED[meeting?.answer] ?? "";
}

// A meeting's times and, for an invitation, his answer: "14:00 to 14:30,
// maybe".
function about(meeting) {
    return [span(meeting), answered(meeting)].filter(part => part !== "").join(", ");
}

// Whether a meeting is an invitation, which takes an answer.
function invited(meeting) {
    return answered(meeting) !== "";
}

// The line that asks the program to answer a meeting yes, maybe or no.
function answer(meeting, said) {
    return `${JSON.stringify({ answer: { id: meeting.id, with: said } })}\n`;
}

// A Meet call's code, as abc-defg-hij, from the app id Chrome gives the
// window it opens a call's link in as an app of its own: its host and
// path, the path's slash an underscore, then its profile, as
// chrome-meet.google.com__abc-defg-hij-Default. Any other window, Meet's
// own front page and a call in a browser tab among them, is no call: "".
const MEET_WINDOW = /^chrome-meet\.google\.com__([a-z]{3}-[a-z]{4}-[a-z]{3})-/;
// The code a Meet link joins by, before any ?authuser= that follows it.
const MEET_LINK = /^https:\/\/meet\.google\.com\/([a-z-]+)/;

function meetCall(appId) {
    const found = MEET_WINDOW.exec(appId ?? "");
    return found ? found[1] : "";
}

// The meeting whose call has that code, or null for one nobody asked
// Joel to, such as a link someone pasted.
function calledBy(meetings, code) {
    return meetings.find(meeting => code !== ""
        && MEET_LINK.exec(meeting.link ?? "")?.[1] === code)
        ?? null;
}

// The line that asks the program to cancel a meeting Joel holds.
function cancel(meeting) {
    return `${JSON.stringify({ cancel: { id: meeting.id } })}\n`;
}
