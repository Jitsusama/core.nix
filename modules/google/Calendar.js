// What the shell works out from the meetings the google program keeps
// (src/state.rs): which meeting is next, when it is in words, which one
// wants Joel and what its notice tells, whether Return joins it, how Joel
// answered it, how it repeats, the lines that answer and cancel it
// (src/ask.rs), and which Meet call a window is. Calendar.qml asks these
// of the meetings Google.qml holds, Mail.js asks the day and the time, and
// the checks run them in Qt's own engine (tests/calendar.qml). Times are
// milliseconds since 1970, as the program gives them.
.pragma library

// In milliseconds.
const MINUTE = 60_000;
const DAY = 86_400_000;

// How soon a meeting coming takes over from the one going on: ten minutes.
const TAKES_OVER = 600_000;
// Up to how far ahead "in 12 min" is said rather than the time, and how
// far ahead a meeting wants Joel in Now, counting down, unless one of
// its reminders comes sooner: an hour.
const SOON = 3_600_000;
// How long after its start a meeting still wants him, to join it: five
// minutes.
const AFTER = 300_000;
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

// How long before its start a meeting wants Joel: an hour, or from its
// first reminder when that's sooner, so a reminder a day ahead is said.
function ahead(meeting) {
    return Math.max(SOON, (meeting.reminders?.[0] ?? 0) * MINUTE);
}

// The meeting that wants Joel, or null: of those that could be next, the
// first to start that's within its time ahead, or started at most five
// minutes ago. The Today row names what's on (`next`); this names what's
// coming, so a meeting going on doesn't hide the next one's count.
function wanting(meetings, now) {
    return coming(meetings, now)
        .filter(meeting => now >= meeting.start - ahead(meeting)
            && now <= meeting.start + AFTER)
        .sort((a, b) => a.start - b.start || a.end - b.end)[0] ?? null;
}

// Whether a meeting that wants Joel is in the margin: from ten minutes
// before its start. Further off, Now alone lists it.
function marked(meeting, now) {
    return Boolean(meeting) && meeting.start - now <= TAKES_OVER;
}

// What its notice tells by now, which says it again each time it changes:
// its last reminder come, by its minutes ("10 min"), "start" once it
// starts, and nothing before its first reminder.
function telling(meeting, now) {
    if (!meeting) {
        return "";
    }
    if (now >= meeting.start) {
        return "start";
    }
    const come = (meeting.reminders ?? [])
        .filter(minutes => now >= meeting.start - minutes * MINUTE);
    return come.length > 0 ? `${Math.min(...come)} min` : "";
}

// When what its notice tells by now came: its start, or its last reminder.
function told(meeting, now) {
    const said = telling(meeting, now);
    return said === "start" ? meeting.start
        : meeting.start - Number.parseInt(said, 10) * MINUTE;
}

// Whether another notice already told Joel what the shell's would, such
// as Calendar's own page in Chrome at the same reminder: one named for it,
// come since that reminder, or its start, so the shell doesn't say it
// twice. The shell's own says when beside the name ("Standup in 10 min"),
// so it's never one of them.
function toldElsewhere(meeting, notices, now) {
    if (!meeting || telling(meeting, now) === "") {
        return false;
    }
    const title = meeting.title.trim().toLowerCase();
    const since = told(meeting, now);
    return title !== "" && notices.some(notice => notice.at >= since
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

// Someone asked, by name, with how they answered when they did: "Grace
// Hopper, not going". One with no name is said by their address.
function asked(person) {
    const name = person.name || person.email || "";
    const said = ANSWERED[person.answer] ?? "";
    return said === "" ? name : `${name}, ${said}`;
}

// Its details, as the launcher's drawer shows them beside its row: its
// title, when and where it is, its call and the number it's dialled into
// by, his answer, who holds it and who else is asked, the files on it by
// their titles, and what it's about, every one plain text, since the
// description is someone else's words.
function details(meeting, untitled) {
    const attachments = meeting.attachments ?? [];
    const facts = [
        { label: "When", value: dated(meeting) },
        { label: "Repeats", value: repeated(meeting.repeats) },
        { label: "Where", value: meeting.place ?? "" },
        { label: "Call", value: meeting.link ?? "" },
        { label: "Dial In", value: dialled(meeting.dial) },
        { label: "Your Answer", value: answered(meeting) },
        { label: "Holds It", value: meeting.organizer ? asked(meeting.organizer) : "" },
        { label: "Asked", value: (meeting.guests ?? []).map(asked).join("; ") },
        {
            label: attachments.length === 1 ? "Attachment" : "Attachments",
            value: attachments.map(file => file.title).join(", ")
        }
    ];
    return {
        title: meeting.title === "" ? untitled : meeting.title,
        facts: facts.filter(fact => fact.value !== ""),
        body: meeting.about ?? ""
    };
}

// What one of a series counts by, one and many.
const EVERY = {
    day: ["day", "days"],
    week: ["week", "weeks"],
    month: ["month", "months"],
    year: ["year", "years"]
};
// Which of a day in its month, from the first.
const WHICH = ["", "first", "second", "third", "fourth", "fifth"];
// Monday to Friday, as the days count from Sunday.
const WEEKDAYS = "1,2,3,4,5";
// An ordinal's ending goes by its last figure, but for 11th to 13th.
const TEN = 10;
const HUNDRED = 100;
const TEENS = [11, 12, 13];
const ENDINGS = ["th", "st", "nd", "rd"];

// 1st, 2nd, 3rd, 4th, 11th, 21st.
function ordinal(n) {
    const ending = TEENS.includes(n % HUNDRED) ? "th" : ENDINGS[n % TEN] ?? "th";
    return `${n}${ending}`;
}

// Which of a day in its month: first, or from the end, last and second
// last.
function which(n) {
    if (n > 0) {
        return WHICH[n] ?? ordinal(n);
    }
    return n === -1 ? "last" : `${WHICH[-n] ?? ordinal(-n)} last`;
}

// Words in a list, as one says them: "a", "a and b", "a, b and c".
function listed(words) {
    if (words.length < 2) {
        return words.join("");
    }
    return `${words.slice(0, -1).join(", ")} and ${words.at(-1)}`;
}

// How a meeting repeats, as its details say it: "Every weekday", "Every 2
// weeks on Monday and Wednesday", "Every month on the last Friday, until
// 31 December 2026"; or "" for one that doesn't, or whose series' rule
// wasn't heard (src/calendar.rs).
function repeated(repeats) {
    const [one, many] = EVERY[repeats?.every] ?? [];
    if (!one) {
        return "";
    }
    const days = repeats.days ?? [];
    const plain = days.every(day => day.nth === null);
    const weekdays = days.map(day => day.day).sort().join(",") === WEEKDAYS;
    let said = "";
    if (repeats.every === "week" && repeats.interval === 1 && plain && weekdays) {
        said = "Every weekday";
    } else {
        said = repeats.interval === 1 ? `Every ${one}` : `Every ${repeats.interval} ${many}`;
        const on = days.map(day => (day.nth === null
            ? DAYS[day.day] : `the ${which(day.nth)} ${DAYS[day.day]}`));
        const dates = (repeats.monthDays ?? []).map(day => (day > 0
            ? `the ${ordinal(day)}` : `the ${which(day)} day`));
        if (on.length + dates.length > 0) {
            said += ` on ${listed(on.concat(dates))}`;
        }
    }
    if (repeats.times) {
        return `${said}, ${repeats.times} times`;
    }
    if (repeats.until) {
        const last = new Date(repeats.until);
        return `${said}, until ${last.getDate()} ${MONTHS[last.getMonth()]} ${last.getFullYear()}`;
    }
    return said;
}

// The number a call is dialled into by, and its PIN when it asks one, as
// +1 555-010-0199, PIN 123456789; or "" for a call with none.
function dialled(dial) {
    if (!dial) {
        return "";
    }
    return dial.pin ? `${dial.number}, PIN ${dial.pin}` : dial.number;
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

// A Meet code as Joel types or pastes it, three letters, four and three,
// with or without its dashes and spaces, or its whole link, written as
// Meet writes it: "abc-defg-hij"; or "" for anything else, which Meet
// wouldn't know.
const MEET_CODE = /^([a-z]{3})-?([a-z]{4})-?([a-z]{3})$/;
const SPACES = /\s+/g;

function meetCode(given) {
    const plain = String(given ?? "").trim().toLowerCase().replace(SPACES, "");
    const parts = MEET_CODE.exec(MEET_LINK.exec(plain)?.[1] ?? plain);
    return parts ? `${parts[1]}-${parts[2]}-${parts[3]}` : "";
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

// Whether Joel may move a meeting: Google lets him change it (the
// program says through which calendar), and it isn't all day, whose
// times are days.
function movable(meeting) {
    return Boolean(meeting) && Boolean(meeting.through) && !meeting.allDay;
}

// Where Move the Meeting offers to take one, each its name and how far:
// by minutes, or by days, kept at the same time on the clock.
const LATER = [
    { name: "15 Minutes Later", minutes: 15 },
    { name: "30 Minutes Later", minutes: 30 },
    { name: "An Hour Later", minutes: 60 },
    { name: "Tomorrow, Same Time", days: 1 },
    { name: "Next Week, Same Time", days: 7 }
];

// Its start moved by minutes, or by days at the same time on the clock,
// which a change of the clocks between doesn't move.
function shifted(start, by) {
    if (by.days) {
        const day = new Date(start);
        day.setDate(day.getDate() + by.days);
        return day.getTime();
    }
    return start + by.minutes * MINUTE;
}

// Where Move the Meeting offers to take a meeting, each lasting as long:
// [{ name, start, end }].
function later(meeting) {
    const lasts = meeting.end - meeting.start;
    return LATER.map(by => {
        const start = shifted(meeting.start, by);
        return { name: by.name, start, end: start + lasts };
    });
}

// The first of Joel's other meetings that a meeting moved to run from
// `start` to `end` would overlap, or null: not one all day, nor one he
// said no to.
function clashes(meeting, start, end, meetings) {
    return meetings.find(other => other.id !== meeting.id && !other.allDay
        && other.answer !== "no" && other.start < end && other.end > start) ?? null;
}

// A time typed to move a meeting to: its hour, a colon and its minutes
// when it has them, and "am" or "pm" when typed, after a day when one
// is: "14:30", "9", "tomorrow 10:00", "thursday 9", "thu 2pm".
const TYPED_TIME = /^(?:(\S+)\s+)?(\d{1,2})(?::(\d{2}))?\s*(am|pm)?$/;
// Hours on a clock.
const HOURS = 24;
const HALF_DAY = 12;
const MINUTES = 60;
// How few letters of a day's name name it: "thu" is Thursday.
const DAY_LETTERS = 3;

// The day a word names, as days from today: "today", "tomorrow", or a
// day of the week by its name or its first three letters or more, the
// next one on or after today; or null for a word that names none. Today's
// own name comes back as a week on, for a time gone by today.
function dayNamed(word, now) {
    if (word === undefined || word === "today") {
        return 0;
    }
    if (word === "tomorrow") {
        return 1;
    }
    const named = word.length >= DAY_LETTERS ? DAYS.findIndex(day => day.toLowerCase().startsWith(word))
        : -1;
    return named < 0 ? null : (named - new Date(now).getDay() + DAYS.length) % DAYS.length;
}

// When a typed time would start a meeting, lasting as long, or null for
// text that isn't a time, or one gone by. A day of the week named as
// today, at a time gone by, is next week's.
function typed(text, meeting, now) {
    const found = TYPED_TIME.exec(text.trim().toLowerCase());
    if (!found) {
        return null;
    }
    const days = dayNamed(found[1], now);
    let hour = Number(found[2]);
    const minute = Number(found[3] ?? 0);
    if (found[4]) {
        if (hour < 1 || hour > HALF_DAY) {
            return null;
        }
        hour = hour % HALF_DAY + (found[4] === "pm" ? HALF_DAY : 0);
    }
    if (days === null || hour >= HOURS || minute >= MINUTES) {
        return null;
    }
    const day = new Date(now);
    day.setDate(day.getDate() + days);
    day.setHours(hour, minute, 0, 0);
    const weekday = ![undefined, "today", "tomorrow"].includes(found[1]);
    if (day.getTime() <= now && weekday) {
        day.setDate(day.getDate() + DAYS.length);
    }
    const start = day.getTime();
    return start <= now ? null : { start, end: start + meeting.end - meeting.start };
}

// Who Google tells of a change to a meeting, by name: everyone else on
// it, and who holds it when that isn't Joel.
function toldOfChanges(meeting) {
    return (meeting.organizer ? [meeting.organizer] : []).concat(meeting.guests ?? [])
        .map(person => person.name);
}

// Who's told, as the sure row says it: "Telling Nobody Else", "Telling
// Ada Lovelace", "Telling Ada Lovelace and Grace Hopper", or the first
// two and how many more.
function tellingThem(people) {
    if (people.length === 0) {
        return "Telling Nobody Else";
    }
    if (people.length <= 2) {
        return `Telling ${people.join(" and ")}`;
    }
    const more = people.length - 2;
    return `Telling ${people.slice(0, 2).join(", ")} and ${more} ${more === 1 ? "Other"
        : "Others"}`;
}

// The line that asks the program to move a meeting to run from `start`
// to `end`.
function move(meeting, start, end) {
    return `${JSON.stringify({ move: { id: meeting.id, start, end } })}\n`;
}
