// What the shell works out from the meetings the google program keeps
// (src/state.rs): which meeting is next, when it is in words, whether it's
// about to start, how Joel answered it, and the line that answers it
// (src/ask.rs). Calendar.qml holds the meetings and asks these, and
// the checks run them in Qt's own engine (tests/calendar.qml). Times are
// milliseconds since 1970, as the program gives them.
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
// Up to how far ahead "in 12 min" is said rather than the time: an hour.
const SOON = 3_600_000;
// How wide a clock's hours and minutes are written.
const DIGITS = 2;

const DAYS = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

// What the program said, as one line of its socket, or null when the line
// isn't its state, so a half line or a stray one changes nothing.
function heard(line) {
    let said;
    try {
        said = JSON.parse(line);
    } catch (_error) {
        return null;
    }
    if (!said || typeof said.account !== "string" || !Array.isArray(said.meetings)) {
        return null;
    }
    return { account: said.account, meetings: said.meetings, heard: said.heard ?? null };
}

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
