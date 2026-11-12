// What the shell works out from the meetings the google program keeps
// (src/state.rs): which meeting is next, when it is in words, which one
// wants Joel and what its notice tells, whether Return joins it, how Joel
// answered it and how its guests did, what its row notes, and the lines
// that answer, cancel and move it (src/ask.rs); Repeats.js says how it
// repeats, and Meet.js which call a window is. Calendar.qml asks these of
// the meetings Google.qml holds, Meet.js asks the time, and the checks
// run them in Qt's own engine (tests/calendar.qml). Times are
// milliseconds since 1970, as the program gives them.
.pragma library
.import "When.js" as When
.import "Repeats.js" as Repeats

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

// The names of the days and months, as Repeats.js says a series by them.
const DAYS = Repeats.DAYS;
const MONTHS = Repeats.MONTHS;

// Between the items of one run, as every side and fact joins them.
const BETWEEN = " \u00b7 ";

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

// Its time on a clock, "09:05", as When.js writes every one.
function clock(date) {
    return When.clock(date.getTime());
}

// Local midnight before `time`, so days are counted as the clock does.
function midnight(time) {
    const day = new Date(time);
    day.setHours(0, 0, 0, 0);
    return day.getTime();
}

// Whether Now holds a meeting as its next: on, starting today, or
// starting within the hour, the hour it wants Joel from (`wanting`). One
// just past midnight is next too, so Now shows the calendar's own row for
// it rather than the coming meeting's piece, which has no details.
function upNext(meeting, now) {
    return Boolean(meeting) && meeting.end > now
        && meeting.start < Math.max(midnight(now) + DAY, now + SOON);
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
// "tomorrow" or its day. One over says when it was held, from its start,
// today as When.js's came says it, before today its day and its time,
// "yesterday 15:00", in `said`'s words, never "now" (VKCF3Z, ccccbe5f).
function when(meeting, now, said) {
    const held = meeting.end <= now ? When.day(meeting.start, now, said) : null;
    if (held !== null && !meeting.allDay) {
        return held === "" ? When.came(meeting.start, now, said)
            : When.moment(meeting.start, now, said);
    }
    const start = new Date(meeting.start);
    const day = held ?? dayOf(meeting.start, now);
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
// October \u00b7 14:00 to 14:30", or "Wednesday 28 October \u00b7 all day",
// the day and its times two items of one run, joined by a dot as every run
// is (docs/design.md's Set, Not Styled).
function dated(meeting) {
    const day = new Date(meeting.start);
    return [`${DAYS[day.getDay()]} ${day.getDate()} ${MONTHS[day.getMonth()]}`, span(meeting)]
        .join(BETWEEN);
}

// How Joel answered an invitation, as his calendar would say it to him,
// or nothing for a meeting that isn't one, as one he holds.
const ANSWERED = { yes: "going", maybe: "maybe", no: "not going", waiting: "not answered" };

function answered(meeting) {
    return ANSWERED[meeting?.answer] ?? "";
}

// A meeting's times and, for an invitation, his answer: "14:00 to 14:30
// \u00b7 maybe", the answer a word following its figures after a dot.
function about(meeting) {
    return [span(meeting), answered(meeting)].filter(part => part !== "").join(BETWEEN);
}

// What a meeting's row notes after its name: its times and his answer
// while it's to come or on, "14:00 to 14:30 \u00b7 maybe", and nothing once
// it's over, where its right alone says when it was and its drawer keeps
// When and Your Answer (VKCF3Z, c7d9e2bd).
function noted(meeting, now) {
    return meeting.end <= now ? "" : about(meeting);
}

// Whether a meeting is an invitation, which takes an answer.
function invited(meeting) {
    return answered(meeting) !== "";
}

// Someone asked, by name, with how they answered when they did: "Grace
// Hopper \u00b7 not going", the answer after a dot, so a comma is left to
// join names alone. One with no name is said by their address.
function asked(person) {
    const name = person.name || person.email || "";
    const said = ANSWERED[person.answer] ?? "";
    return said === "" ? name : `${name}${BETWEEN}${said}`;
}

// The addresses of those on a meeting besides Joel: who holds it, unless
// he does, then each guest.
function guestEmails(meeting) {
    return (meeting.holds ? [] : [meeting.organizer?.email])
        .concat((meeting.guests ?? []).map(guest => guest.email));
}

// How those asked answered, counted, as a calendar heads its list of
// guests: "9 going, 2 not going, 3 not answered", each answer in the
// order an invitation offers them. One not yet heard from counts as not
// answered. Nothing for one guest, whom Asked says whole.
function tallied(guests) {
    if (guests.length < 2) {
        return "";
    }
    const counts = new Map(Object.values(ANSWERED).map(said => [said, 0]));
    for (const guest of guests) {
        const said = ANSWERED[guest.answer] ?? ANSWERED.waiting;
        counts.set(said, counts.get(said) + 1);
    }
    return [...counts].filter(([, count]) => count > 0)
        .map(([said, count]) => `${count} ${said}`).join(", ");
}

// Its details, as the launcher's drawer shows them beside its row: its
// title, when and where it is, its call and the number it's dialled into
// by, his answer, who holds it, how many of those asked answered each
// way and who they are, the files on it by their titles, and what it's
// about, every one plain text, since the description is someone else's
// words; and, said, how many of its series came before (Google.js's).
function details(meeting, untitled, earlier = "") {
    const attachments = meeting.attachments ?? [];
    const facts = [
        { label: "When", value: dated(meeting) },
        { label: "Repeats", value: Repeats.repeated(meeting.repeats) },
        { label: "Earlier", value: earlier },
        { label: "Where", value: meeting.place ?? "" },
        { label: "Call", value: meeting.link ?? "" },
        { label: "Dial In", value: dialled(meeting.dial) },
        { label: "Your Answer", value: answered(meeting) },
        { label: "Holds It", value: meeting.organizer ? asked(meeting.organizer) : "" },
        { label: "Answers", value: tallied(meeting.guests ?? []) },
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

// Where Move the Meeting offers to take one, each by its state of
// verb:calendar:move in the vocabulary, which names it, and how far: by
// minutes, or by days, kept at the same time on the clock (the google
// daemon's calendar_words.rs LATER).
const LATER = [
    { state: "quarter", minutes: 15 },
    { state: "half", minutes: 30 },
    { state: "hour", minutes: 60 },
    { state: "tomorrow", days: 1 },
    { state: "week", days: 7 }
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
// [{ state, start, end }], each named by its state.
function later(meeting) {
    const lasts = meeting.end - meeting.start;
    return LATER.map(by => {
        const start = shifted(meeting.start, by);
        return { state: by.state, start, end: start + lasts };
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
