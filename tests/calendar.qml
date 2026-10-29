// Which meeting is next, when it is in words, whether it's about to start,
// said as a notice or told already by another, and joined by Return, its day in full, how Joel
// answered it, its details for the drawer and the line answering it, and
// which Meet call a window is and whose, from meetings like the google
// program's, run in Qt's own JavaScript engine,
// the one the shell runs it in. The
// `the-shell-tells-the-next-meeting` check copies Calendar.js and Meet.js beside this
// file and runs `qml -platform offscreen calendar.qml`. Times are made on
// the local clock, so the cases hold in whatever zone the check runs in.
import QtQuick
import "Calendar.js" as Calendar
import "Meet.js" as Meet

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const failures = [];
        let asked = 0;
        const minute = 60 * 1000;
        // Monday 26 October 2026, 15:20 here.
        const at = (day, hour, min) => new Date(2026, 9, day, hour, min).getTime();
        const now = at(26, 15, 20);
        const meeting = (id, start, end, fields) => Object.assign({
            id: id, title: id, start: start, end: end, allDay: false, answer: "yes",
            link: "https://meet.google.com/abc-defg-hij", kind: "meet", page: ""
        }, fields ?? {});
        const check = (what, said, meant) => {
            asked += 1;
            if (JSON.stringify(said) !== JSON.stringify(meant))
                failures.push(`${what}: said ${JSON.stringify(said)}, meant ${JSON.stringify(meant)}`);
        };
        const nextOf = meetings => (Calendar.next(meetings, now) ?? { id: null }).id;

        // Which is next: [what, the meetings, the one next]
        const nexts = [
            ["none at all", [], null],
            ["the one coming", [meeting("later", at(26, 16, 0), at(26, 16, 30))], "later"],
            ["the first of two coming, given out of order", [
                meeting("last", at(26, 17, 0), at(26, 17, 30)),
                meeting("first", at(26, 16, 0), at(26, 16, 30))
            ], "first"],
            ["the one going on, with the next an hour off", [
                meeting("on", at(26, 15, 0), at(26, 16, 0)),
                meeting("later", at(26, 16, 20), at(26, 17, 0))
            ], "on"],
            ["the one starting within ten minutes, over the one going on", [
                meeting("on", at(26, 15, 0), at(26, 16, 0)),
                meeting("soon", at(26, 15, 30), at(26, 16, 0))
            ], "soon"],
            ["not one that's over", [meeting("over", at(26, 14, 0), at(26, 15, 20))], null],
            ["not one said no to", [
                meeting("declined", at(26, 15, 30), at(26, 16, 0), { answer: "no" }),
                meeting("later", at(26, 16, 0), at(26, 16, 30))
            ], "later"],
            ["not a whole day", [
                meeting("holiday", at(26, 0, 0), at(27, 0, 0), { allDay: true })
            ], null],
            ["one not yet answered", [
                meeting("asked", at(26, 16, 0), at(26, 16, 30), { answer: "waiting" })
            ], "asked"],
            ["one he's not invited to, on his own calendar", [
                meeting("his", at(26, 16, 0), at(26, 16, 30), { answer: null })
            ], "his"],
            ["of two going on, the later started", [
                meeting("long", at(26, 14, 0), at(26, 17, 0)),
                meeting("short", at(26, 15, 0), at(26, 15, 45))
            ], "short"]
        ];
        for (const [what, meetings, meant] of nexts)
            check(`next, ${what}`, nextOf(meetings), meant);

        // When, in words: [the meeting, said]
        const whens = [
            [meeting("on", at(26, 15, 0), at(26, 16, 0)), "now"],
            [meeting("just", at(26, 15, 20), at(26, 16, 0)), "now"],
            [meeting("soon", at(26, 15, 32), at(26, 16, 0)), "in 12 min"],
            [meeting("seconds", now + 20 * 1000, at(26, 16, 0)), "in 1 min"],
            [meeting("hour", at(26, 16, 20), at(26, 17, 0)), "at 16:20"],
            [meeting("evening", at(26, 19, 5), at(26, 20, 0)), "at 19:05"],
            [meeting("tomorrow", at(27, 9, 0), at(27, 9, 30)), "tomorrow at 09:00"],
            [meeting("thursday", at(29, 9, 0), at(29, 9, 30)), "Thursday at 09:00"],
            [meeting("all", at(26, 0, 0), at(27, 0, 0), { allDay: true }), "today"],
            [meeting("all", at(28, 0, 0), at(29, 0, 0), { allDay: true }), "Wednesday"]
        ];
        for (const [one, meant] of whens)
            check(`when ${one.id}`, Calendar.when(one, now), meant);

        check("a span", Calendar.span(meeting("a", at(26, 14, 0), at(26, 14, 30))),
            "14:00 to 14:30");
        check("a whole day's span", Calendar.span(meeting("a", at(26, 0, 0), at(27, 0, 0),
            { allDay: true })), "all day");

        // Wanting Joel from an hour before, or its first reminder when
        // sooner, to five minutes after: [what, the meetings, at, wanted]
        const standup = meeting("standup", at(26, 15, 30), at(26, 15, 45),
            { reminders: [10, 1] });
        const review = meeting("review", at(26, 16, 30), at(26, 17, 0));
        const early = meeting("early", at(27, 9, 0), at(27, 9, 30), { reminders: [1440] });
        const ids = found => found === null ? null : found.id;
        const wants = [
            ["none over an hour off", [review], at(26, 15, 29), null],
            ["one an hour off", [review], at(26, 15, 30), "review"],
            ["the first of two", [review, standup], at(26, 15, 30), "standup"],
            ["one five minutes on", [standup], at(26, 15, 35), "standup"],
            ["not one past five minutes on", [standup], at(26, 15, 35) + 1, null],
            ["the next while one goes on", [standup, review], at(26, 15, 40), "review"],
            ["one a day off, at its reminder", [early], at(26, 9, 0), "early"],
            ["not one a day off before it", [early], at(26, 9, 0) - 1, null],
            ["not one said no to", [meeting("no", at(26, 15, 30), at(26, 16, 0),
                { answer: "no" })], now, null],
            ["not a whole day", [meeting("all", at(26, 0, 0), at(27, 0, 0),
                { allDay: true })], now, null]
        ];
        for (const [what, meetings, time, meant] of wants)
            check(`wanting ${what}`, ids(Calendar.wanting(meetings, time)), meant);

        // In the margin from ten minutes before.
        check("not marked eleven minutes off", Calendar.marked(standup, at(26, 15, 19)), false);
        check("marked ten minutes off", Calendar.marked(standup, at(26, 15, 20)), true);
        check("marked once on", Calendar.marked(standup, at(26, 15, 32)), true);
        check("nothing is marked", Calendar.marked(null, now), false);

        // What its notice tells: [at, told]
        const tells = [
            [at(26, 15, 19), ""], [at(26, 15, 20), "10 min"], [at(26, 15, 28), "10 min"],
            [at(26, 15, 29), "1 min"], [at(26, 15, 30), "start"], [at(26, 15, 34), "start"]
        ];
        for (const [time, meant] of tells)
            check(`telling at ${new Date(time).toTimeString()}`,
                Calendar.telling(standup, time), meant);
        check("one with no reminders tells only its start", [
            Calendar.telling(review, at(26, 16, 29)), Calendar.telling(review, at(26, 16, 30))
        ], ["", "start"]);
        check("nothing tells", Calendar.telling(null, now), "");

        // Told already by another notice since its reminder, at 15:29's:
        // [what, the notices, told]
        const notice = (summary, time) => ({ summary: summary, at: time });
        const reminded = at(26, 15, 29);
        const told = [
            ["by one named for it", [notice("Standup", at(26, 15, 29))], true],
            ["by one named for it, spaced", [notice(" standup ", at(26, 15, 29))], true],
            ["not by one at the reminder before", [notice("standup", at(26, 15, 20))], false],
            ["not by one from yesterday's", [notice("standup", at(25, 15, 29))], false],
            ["not by the shell's own", [notice("standup in 1 min", at(26, 15, 29))], false],
            ["not by another's", [notice("Lunch", at(26, 15, 29))], false],
            ["not by none", [], false]
        ];
        for (const [what, notices, meant] of told)
            check(`told ${what}`, Calendar.toldElsewhere(standup, notices, reminded), meant);
        check("told at its start by one since", Calendar.toldElsewhere(standup,
            [notice("standup", at(26, 15, 30))], at(26, 15, 31)), true);
        check("not told before its first reminder", Calendar.toldElsewhere(standup,
            [notice("standup", at(26, 15, 19))], at(26, 15, 19)), false);
        check("an untitled one isn't told", Calendar.toldElsewhere(
            meeting("", at(26, 15, 30), at(26, 15, 45)), [notice("", at(26, 15, 30))],
            at(26, 15, 30)), false);
        check("nothing isn't told", Calendar.toldElsewhere(null, [], now), false);

        // Whether Return joins it: [what, the meeting, joins]
        const joins = [
            ["one on", meeting("on", at(26, 15, 0), at(26, 16, 0)), true],
            ["one ten minutes off", meeting("ten", at(26, 15, 30), at(26, 16, 0)), true],
            ["one eleven minutes off", meeting("eleven", at(26, 15, 31), at(26, 16, 0)), false],
            ["one tomorrow", meeting("tomorrow", at(27, 9, 0), at(27, 9, 30)), false],
            ["one over", meeting("over", at(26, 14, 0), at(26, 15, 20)), false],
            ["one with no call", meeting("room", at(26, 15, 25), at(26, 16, 0), { link: "" }),
                false],
            ["a whole day", meeting("all", at(26, 0, 0), at(27, 0, 0), { allDay: true }), false],
            ["no meeting", null, false]
        ];
        for (const [what, one, meant] of joins)
            check(`Return joins ${what}`, Calendar.joinable(one, now), meant);

        // On or starting today: [what, the meeting, today]
        const todays = [
            ["one this evening", meeting("evening", at(26, 23, 0), at(26, 23, 30)), true],
            ["one on", meeting("on", at(26, 15, 0), at(26, 16, 0)), true],
            ["one past midnight", meeting("late", at(27, 0, 0), at(27, 0, 30)), false],
            ["one tomorrow", meeting("tomorrow", at(27, 9, 0), at(27, 9, 30)), false],
            ["one over", meeting("over", at(26, 14, 0), at(26, 15, 0)), false],
            ["no meeting", null, false]
        ];
        for (const [what, one, meant] of todays)
            check(`today, ${what}`, Calendar.today(one, now), meant);

        check("its day and times in full", Calendar.dated(
            meeting("m", at(26, 14, 0), at(26, 14, 30))), "Monday 26 October, 14:00 to 14:30");
        check("a whole day's in full", Calendar.dated(
            meeting("m", at(28, 0, 0), at(29, 0, 0), { allDay: true })),
            "Wednesday 28 October, all day");

        // How he answered, and his answer: [answer, said, invited]
        const answers = [
            ["yes", "going", true],
            ["maybe", "maybe", true],
            ["no", "not going", true],
            ["waiting", "not answered", true],
            [null, "", false]
        ];
        for (const [answer, said, invited] of answers) {
            const one = meeting("m", at(26, 14, 0), at(26, 14, 30), { answer: answer });
            check(`answered ${answer}`, Calendar.answered(one), said);
            check(`${answer} is an invitation`, Calendar.invited(one), invited);
        }
        check("an answer beside the times", Calendar.about(
            meeting("m", at(26, 14, 0), at(26, 14, 30), { answer: "maybe" })),
            "14:00 to 14:30, maybe");
        check("a meeting he holds says its times alone", Calendar.about(
            meeting("m", at(26, 14, 0), at(26, 14, 30), { answer: null })), "14:00 to 14:30");

        // Its details, as the drawer shows them beside its row.
        check("an invitation's details in full", Calendar.details(meeting("m", at(26, 14, 0),
            at(26, 14, 30), {
                title: "Standup",
                answer: "maybe",
                place: "Room 4",
                about: "What we did.\nWhat we do.",
                organizer: { name: "Ada Lovelace", email: "ada@example.com" },
                guests: [{ name: "Grace Hopper", email: "grace@example.com", answer: "no" },
                    { name: "", email: "alan@example.com", answer: null }],
                dial: { number: "+1 555-010-0199", pin: "123456789" },
                attachments: [{ title: "Agenda", link: "https://docs.google.com/document/d/1" },
                    { title: "Numbers", link: "https://docs.google.com/presentation/d/3" }]
            }), "Untitled"), {
            title: "Standup",
            facts: [
                { label: "When", value: "Monday 26 October, 14:00 to 14:30" },
                { label: "Where", value: "Room 4" },
                { label: "Call", value: "https://meet.google.com/abc-defg-hij" },
                { label: "Dial In", value: "+1 555-010-0199, PIN 123456789" },
                { label: "Your Answer", value: "maybe" },
                { label: "Holds It", value: "Ada Lovelace" },
                { label: "Answers", value: "1 not going, 1 not answered" },
                { label: "Asked", value: "Grace Hopper, not going; alan@example.com" },
                { label: "Attachments", value: "Agenda, Numbers" }
            ],
            body: "What we did.\nWhat we do."
        });
        check("one file, and a number with no PIN", Calendar.details(meeting("m",
            at(26, 14, 0), at(26, 14, 30), {
                answer: null, link: null, dial: { number: "+1 555-010-0199", pin: null },
                attachments: [{ title: "Agenda", link: "https://docs.google.com/document/d/1" }]
            }), "Untitled").facts.slice(1), [
            { label: "Dial In", value: "+1 555-010-0199" },
            { label: "Attachment", value: "Agenda" }
        ]);
        check("a bare one he holds says only when", Calendar.details(meeting("m", at(26, 14, 0),
            at(26, 14, 30), { title: "", answer: null, link: null }), "Untitled"), {
            title: "Untitled",
            facts: [{ label: "When", value: "Monday 26 October, 14:00 to 14:30" }],
            body: ""
        });
        // How one of a series repeats, said after when it is.
        const day = (n, nth) => ({ day: n, nth: nth ?? null });
        const rule = fields => Object.assign({ every: "week", interval: 1, days: [],
            monthDays: [], times: null, until: null }, fields);
        check("one of a series says how it repeats after when", Calendar.details(meeting("m",
            at(26, 14, 0), at(26, 14, 30), { answer: null, link: null,
                repeats: rule({ days: [day(1)] }) }), "Untitled").facts, [
            { label: "When", value: "Monday 26 October, 14:00 to 14:30" },
            { label: "Repeats", value: "Every week on Monday" }
        ]);
        check("how a series repeats, in words", [
            rule({ days: [1, 2, 3, 4, 5].map(n => day(n)) }),
            rule({ days: [day(1), day(3)], interval: 2 }),
            rule({ every: "day" }),
            rule({ every: "day", interval: 3 }),
            rule({ every: "month", days: [day(5, -1)] }),
            rule({ every: "month", days: [day(2, 1)], times: 6 }),
            rule({ every: "month", monthDays: [1, 22, 13, -1, -2] }),
            rule({ every: "year", until: new Date(2026, 11, 31).getTime() }),
            rule({ days: [day(1), day(3), day(5)] }),
            null,
            rule({ every: "hour" })
        ].map(Calendar.repeated), [
            "Every weekday",
            "Every 2 weeks on Monday and Wednesday",
            "Every day",
            "Every 3 days",
            "Every month on the last Friday",
            "Every month on the first Tuesday, 6 times",
            "Every month on the 1st, the 22nd, the 13th, the last day and the second last day",
            "Every year, until 31 December 2026",
            "Every week on Monday, Wednesday and Friday",
            "",
            ""
        ]);
        // How those asked answered, counted in an invitation's order.
        const answering = answers => Calendar.details(meeting("m", at(26, 14, 0), at(26, 14, 30),
            { answer: null, link: null, guests: answers.map((answer, n) => ({
                name: `Guest ${n}`, email: "", answer: answer })) }), "Untitled").facts
            .find(fact => fact.label === "Answers")?.value ?? "";
        check("how those asked answered, counted", [
            ["waiting", "yes", "no", "yes", "maybe", "yes", null],
            ["yes", "yes", "yes"],
            ["no"],
            []
        ].map(answering), ["3 going, 1 maybe, 1 not going, 2 not answered", "3 going", "", ""]);
        check("the line answering one", Calendar.answer(meeting("abc_1", now, now), "no"),
            '{"answer":{"id":"abc_1","with":"no"}}\n');
        check("the line cancelling one", Calendar.cancel(meeting("abc_1", now, now)),
            '{"cancel":{"id":"abc_1"}}\n');

        // Moving one: [what, the meeting, movable]
        const movables = [
            ["one through his own", meeting("a", now, now, { through: "primary" }), true],
            ["one through the team's", meeting("a", now, now, { through: "team@x" }), true],
            ["one he may not change", meeting("a", now, now, { through: null }), false],
            ["one all day", meeting("a", at(26, 0, 0), at(27, 0, 0),
                { through: "primary", allDay: true }), false],
            ["none", null, false]
        ];
        for (const [what, one, meant] of movables)
            check(`movable: ${what}`, Calendar.movable(one), meant);

        // Later times keep its length, a day on keeps the clock.
        const design = meeting("design", at(26, 16, 0), at(26, 16, 30));
        check("later times", Calendar.later(design).map(time => [time.name,
            Calendar.dated(Object.assign({ allDay: false }, time))]), [
            ["15 Minutes Later", "Monday 26 October, 16:15 to 16:45"],
            ["30 Minutes Later", "Monday 26 October, 16:30 to 17:00"],
            ["An Hour Later", "Monday 26 October, 17:00 to 17:30"],
            ["Tomorrow, Same Time", "Tuesday 27 October, 16:00 to 16:30"],
            ["Next Week, Same Time", "Monday 2 November, 16:00 to 16:30"]
        ]);
        // A week on crosses Toronto's clocks going back, on Sunday 1
        // November, which mustn't move the time on the clock.
        check("a week on, at the same time on the clock", new Date(Calendar.later(design)[4]
            .start).getHours(), 16);

        // What it clashes with: [what, from, to, the clash]
        const busy = [
            meeting("standup", at(26, 16, 30), at(26, 16, 45)),
            meeting("declined", at(26, 17, 0), at(26, 18, 0), { answer: "no" }),
            meeting("holiday", at(26, 0, 0), at(27, 0, 0), { allDay: true }),
            design
        ];
        const clashing = [
            ["one it overlaps", at(26, 16, 15), at(26, 16, 45), "standup"],
            ["not one it ends as it starts", at(26, 16, 0), at(26, 16, 30), null],
            ["not one he said no to", at(26, 17, 0), at(26, 17, 30), null],
            ["not itself, nor one all day", at(26, 16, 0), at(26, 16, 29), null]
        ];
        for (const [what, from, to, meant] of clashing)
            check(`clashes with ${what}`, ids(Calendar.clashes(design, from, to, busy)), meant);

        // A typed time, at 15:20 on Monday: [typed, its day and times, or null]
        const typing = [
            ["16:30", "Monday 26 October, 16:30 to 17:00"],
            ["17", "Monday 26 October, 17:00 to 17:30"],
            ["  5pm ", "Monday 26 October, 17:00 to 17:30"],
            ["tomorrow 10:00", "Tuesday 27 October, 10:00 to 10:30"],
            ["Thursday 9", "Thursday 29 October, 09:00 to 09:30"],
            ["thu 2pm", "Thursday 29 October, 14:00 to 14:30"],
            ["monday 9", "Monday 2 November, 09:00 to 09:30"],
            ["monday 16:00", "Monday 26 October, 16:00 to 16:30"],
            ["today 9:00", null],
            ["12am", null],
            ["tomorrow 12am", "Tuesday 27 October, 00:00 to 00:30"],
            ["25:00", null],
            ["16:75", null],
            ["13pm", null],
            ["th 9", null],
            ["someday 9", null],
            ["stand", null]
        ];
        for (const [text, meant] of typing) {
            const typedTime = Calendar.typed(text, design, now);
            check(`typed "${text}"`, typedTime === null ? null
                : Calendar.dated(Object.assign({ allDay: false }, typedTime)), meant);
        }

        // Who's told of it: [what, the meeting, as said]
        const person = name => ({ name: name, email: null });
        const tellings = [
            ["nobody", meeting("a", now, now), "Telling Nobody Else"],
            ["its organizer", meeting("a", now, now, { organizer: person("Ada Lovelace") }),
                "Telling Ada Lovelace"],
            ["two", meeting("a", now, now, { organizer: person("Ada Lovelace"),
                guests: [person("Grace Hopper")] }), "Telling Ada Lovelace and Grace Hopper"],
            ["three", meeting("a", now, now, { guests: [person("Ada Lovelace"),
                person("Grace Hopper"), person("Alan Turing")] }),
                "Telling Ada Lovelace, Grace Hopper and 1 Other"],
            ["four", meeting("a", now, now, { organizer: person("Ada Lovelace"),
                guests: [person("Grace Hopper"), person("Alan Turing"), person("Edsger")] }),
                "Telling Ada Lovelace, Grace Hopper and 2 Others"]
        ];
        for (const [what, one, meant] of tellings)
            check(`telling ${what}`, Calendar.tellingThem(Calendar.toldOfChanges(one)), meant);
        check("the line moving one", Calendar.move(design, at(26, 17, 0), at(26, 17, 30)),
            `{"move":{"id":"design","start":${at(26, 17, 0)},"end":${at(26, 17, 30)}}}\n`);

        // A Meet code as typed: [what, typed, the code]
        const codes = [
            ["as Meet writes it", "abc-defg-hij", "abc-defg-hij"],
            ["without its dashes, in capitals", "ABCDEFGHIJ", "abc-defg-hij"],
            ["spaced", " abc defg hij ", "abc-defg-hij"],
            ["its whole link", "https://meet.google.com/abc-defg-hij?authuser=0", "abc-defg-hij"],
            ["a letter short", "abc-def-hij", ""],
            ["digits", "abc-1234-hij", ""],
            ["nothing", "", ""]
        ];
        for (const [what, typed, meant] of codes)
            check(`a Meet code ${what}`, Meet.meetCode(typed), meant);

        // Which Meet call a window is, by its app id: [what, the app id, its code]
        const windows = [
            ["a call in Chrome's app window", "chrome-meet.google.com__abc-defg-hij-Default",
                "abc-defg-hij"],
            ["one in another profile", "chrome-meet.google.com__klm-nopq-rst-Profile_1",
                "klm-nopq-rst"],
            ["Meet's front page", "chrome-meet.google.com__-Default", ""],
            ["Meet's page of what's next", "chrome-meet.google.com__landing-Default", ""],
            ["Calendar's app window", "chrome-calendar.google.com__-Default", ""],
            ["the browser", "google-chrome", ""],
            ["a window with no app id", null, ""]
        ];
        for (const [what, appId, code] of windows)
            check(what, Meet.meetCall(appId), code);
        const onMeet = meeting("on meet", now, now + 30 * minute);
        const pasted = meeting("pasted", now, now + 30 * minute, {
            link: "https://meet.google.com/abc-defg-hijk"
        });
        const another = meeting("another", now, now + 30 * minute, {
            link: "https://meet.google.com/klm-nopq-rst?authuser=1"
        });
        check("a call's meeting", (Meet.calledBy([pasted, onMeet], "abc-defg-hij") ?? {}).id,
            "on meet");
        check("a call's meeting by a link that says more",
            (Meet.calledBy([onMeet, another], "klm-nopq-rst") ?? {}).id, "another");
        check("a call on no meeting", Meet.calledBy([onMeet], "xyz-abcd-efg"), null);
        check("no call on a meeting with no link",
            Meet.calledBy([meeting("room", now, now, { link: null })], ""), null);

        for (const failure of failures)
            console.warn(failure);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
