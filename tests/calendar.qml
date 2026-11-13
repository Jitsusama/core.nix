// Which meeting is next, when it is in words, whether it's about to
// start, said as a notice or told already by another, and joined by
// Return, its day in full, how Joel answered it, its details for the
// drawer, and which Meet call a window is and
// whose, from meetings like the google program's, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-desktop-tells-the-next-meeting` check copies Calendar.js,
// Repeats.js and Meet.js beside this file and runs `qml -platform
// offscreen calendar.qml`. Times are made on the local clock, so the
// cases hold in whatever zone the check runs in.
import QtQuick
import "Calendar.js" as Calendar
import "Repeats.js" as Repeats
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

        // Now's next meeting, on, starting today or within the hour, so one
        // just past midnight is still next: [what, the meeting, when, next]
        const nearly = at(26, 23, 55);
        const todays = [
            ["one this evening", meeting("evening", at(26, 23, 0), at(26, 23, 30)), now, true],
            ["one on", meeting("on", at(26, 15, 0), at(26, 16, 0)), now, true],
            ["one past midnight", meeting("late", at(27, 0, 0), at(27, 0, 30)), now, false],
            ["one past midnight, eight minutes on",
                meeting("soon", at(27, 0, 3), at(27, 0, 33)), nearly, true],
            ["one tomorrow morning, late at night",
                meeting("morning", at(27, 9, 0), at(27, 9, 30)), nearly, false],
            ["one tomorrow", meeting("tomorrow", at(27, 9, 0), at(27, 9, 30)), now, false],
            ["one over", meeting("over", at(26, 14, 0), at(26, 15, 0)), now, false],
            ["no meeting", null, now, false]
        ];
        for (const [what, one, when, meant] of todays)
            check(`next, ${what}`, Calendar.upNext(one, when), meant);

        check("its day and times in full", Calendar.dated(
            meeting("m", at(26, 14, 0), at(26, 14, 30))),
            "Monday 26 October \u00b7 14:00 to 14:30");
        check("a whole day's in full", Calendar.dated(
            meeting("m", at(28, 0, 0), at(29, 0, 0), { allDay: true })),
            "Wednesday 28 October \u00b7 all day");

        // How he answered: [answer, said]
        const answers = [
            ["yes", "going"],
            ["maybe", "maybe"],
            ["no", "not going"],
            ["waiting", "not answered"],
            [null, ""]
        ];
        for (const [answer, said] of answers) {
            const one = meeting("m", at(26, 14, 0), at(26, 14, 30), { answer: answer });
            check(`answered ${answer}`, Calendar.answered(one), said);
        }
        check("an answer beside the times", Calendar.about(
            meeting("m", at(26, 14, 0), at(26, 14, 30), { answer: "maybe" })),
            "14:00 to 14:30 \u00b7 maybe");
        check("a meeting he holds says its times alone", Calendar.about(
            meeting("m", at(26, 14, 0), at(26, 14, 30), { answer: null })), "14:00 to 14:30");
        // A meeting's row notes its times and answer until it's over, then nothing.
        check("a meeting to come notes its times and answer", Calendar.noted(
            meeting("m", at(26, 16, 0), at(26, 16, 30), { answer: "maybe" }), now),
            "16:00 to 16:30 \u00b7 maybe");
        check("a meeting that's over notes nothing", Calendar.noted(
            meeting("m", at(25, 16, 0), at(25, 16, 30), { answer: "maybe" }), now), "");

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
                { label: "When", value: "Monday 26 October \u00b7 14:00 to 14:30" },
                { label: "Where", value: "Room 4" },
                { label: "Call", value: "https://meet.google.com/abc-defg-hij" },
                { label: "Dial In", value: "+1 555-010-0199, PIN 123456789" },
                { label: "Your Answer", value: "maybe" },
                { label: "Holds It", value: "Ada Lovelace" },
                { label: "Answers", value: "1 not going, 1 not answered" },
                { label: "Asked", value: "Grace Hopper \u00b7 not going; alan@example.com" },
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
            facts: [{ label: "When", value: "Monday 26 October \u00b7 14:00 to 14:30" }],
            body: ""
        });
        // How one of a series repeats, said after when it is.
        const day = (n, nth) => ({ day: n, nth: nth ?? null });
        const rule = fields => Object.assign({ every: "week", interval: 1, days: [],
            monthDays: [], times: null, until: null }, fields);
        check("one of a series says how it repeats after when", Calendar.details(meeting("m",
            at(26, 14, 0), at(26, 14, 30), { answer: null, link: null,
                repeats: rule({ days: [day(1)] }) }), "Untitled").facts, [
            { label: "When", value: "Monday 26 October \u00b7 14:00 to 14:30" },
            { label: "Repeats", value: "Every week on Monday" }
        ]);
        check("one of a series a Recent lists once says how many came before",
            Calendar.details(meeting("m", at(26, 14, 0), at(26, 14, 30), { answer: null,
                link: null, repeats: rule({ days: [day(1)] }) }), "Untitled",
                "8 in the past 30 days").facts, [
                { label: "When", value: "Monday 26 October \u00b7 14:00 to 14:30" },
                { label: "Repeats", value: "Every week on Monday" },
                { label: "Earlier", value: "8 in the past 30 days" }
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
        ].map(Repeats.repeated), [
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

        // A call Joel is in, as the google program says Meet told it
        // (src/meet.rs), and the words its row and plate say it in.
        const said = {
            eyebrow: "on Meet \u00b7 call", muted: "muted", live: "live",
            under: "under a minute", minutes: "{minutes} min", hours: "{hours} h",
            hoursMinutes: "{hours} h {minutes} min", inCall: "{count} in the call",
            signIn: "needs Google sign-in", unknown: "not known to Meet yet",
            since: "since {time}", left: "left {time}",
            people: "People", started: "Started", meeting: "Meeting", code: "Code"
        };
        const inCall = {
            code: "abc-defg-hij", granted: true, known: true, started: at(26, 15, 0), people: [
                { name: "Ada Lovelace", joined: at(26, 15, 0), left: at(26, 15, 15) },
                { name: "Grace Hopper", joined: at(26, 15, 2), left: null },
                { name: "Joel", joined: at(26, 15, 3), left: null }
            ]
        };
        check("what Meet said of a call", Meet.heard([inCall], "abc-defg-hij").started,
            at(26, 15, 0));
        check("a call Meet hasn't said", Meet.heard([inCall], "xyz-abcd-efg"), null);
        check("a call begins when its conference did", Meet.began(inCall, now), at(26, 15, 0));
        check("else when its window opened", Meet.began(null, now), now);
        check("a call's side, muted", Meet.side(true, at(26, 14, 57), now, said),
            "muted \u00b7 23 min");
        check("a call's side, heard", Meet.side(false, now - 30000, now, said),
            "live \u00b7 under a minute");
        check("a long call's side", Meet.side(false, at(26, 14, 15), now, said),
            "live \u00b7 1 h 5 min");
        check("those in it, then those who left", Meet.people(inCall).map(one => one.name),
            ["Grace Hopper", "Joel", "Ada Lovelace"]);
        check("its row names those in it", Meet.inIt(inCall), ["Grace Hopper", "Joel"]);
        check("how many are in it", Meet.counted(inCall, said), "2 in the call");
        check("who's in it waits on a sign-in",
            Meet.counted({ code: "abc-defg-hij", granted: false, people: [] }, said),
            "needs Google sign-in");
        check("who's in it before Meet says, or where it keeps no record",
            [Meet.counted(null, said), Meet.counted({ code: "abc-defg-hij", granted: true,
                known: false, people: [] }, said)],
            ["not known to Meet yet", "not known to Meet yet"]);
        check("someone in it, since when", Meet.personSide(inCall.people[1], said),
            "since 15:02");
        check("someone who left, when", Meet.personSide(inCall.people[0], said), "left 15:15");
        check("a call's plate",
            Meet.details({ named: "Standup", code: "abc-defg-hij", heard: inCall,
                since: at(26, 15, 0), meeting: meeting("Standup", at(26, 15, 0), at(26, 15, 30)) },
                said),
            { eyebrow: "on Meet \u00b7 call", eyebrowNames: true, title: "Standup", facts: [
                { label: "People", value: "Grace Hopper, Joel" },
                { label: "Started", value: "15:00" },
                { label: "Meeting", value: "15:00 to 15:30" },
                { label: "Code", value: "abc-defg-hij", identifies: true }
            ], body: "" });
        check("a call on no meeting, nobody heard yet, says when and its code",
            Meet.details({ named: "abc-defg-hij", code: "abc-defg-hij", heard: null, since: now,
                meeting: null }, said).facts
                .map(fact => fact.label), ["Started", "Code"]);

        heldCases(check, at, now, meeting);

        for (const failure of failures)
            console.warn(failure);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }

    // The meetings over, as Calendar's Recent and Meet's say them: when each
    // was held, which of them were calls on Meet, and who each was with.
    function heldCases(check, at, now, meeting) {
        // How the launcher says when, as Mail.qml gives it, and who a call
        // was with, as Meet.qml gives it: the vocabulary's words, and the
        // locale's names, the check's being English.
        const said = {
            now: "just now", minutes: "{minutes} min ago", yesterday: "yesterday",
            tomorrow: "tomorrow", date: (date, format) => Qt.formatDate(date, format),
            two: "{first} and {second}", others: "{first} and {count} others"
        };
        // When one over was held, from its start, as the launcher says when
        // anywhere, never "now", and before today with its time (VKCF3Z,
        // ccccbe5f): [the meeting, said]
        const whens = [
            [meeting("ended now", at(26, 15, 0), now), "20 min ago"],
            [meeting("short", at(26, 15, 10), at(26, 15, 15)), "10 min ago"],
            [meeting("just", now - 30 * 1000, now - 10 * 1000), "just now"],
            [meeting("lunch", at(26, 12, 0), at(26, 13, 0)), "12:00"],
            [meeting("yesterday", at(25, 9, 0), at(25, 9, 30)), "yesterday 09:00"],
            [meeting("thursday", at(22, 9, 0), at(22, 9, 30)), "Thursday 09:00"],
            [meeting("long ago", at(12, 9, 0), at(12, 9, 30)), "12 Oct 09:00"],
            [meeting("holiday", at(25, 0, 0), at(26, 0, 0), { allDay: true }), "yesterday"]
        ];
        for (const [one, meant] of whens)
            check(`when ${one.id} was held`, Calendar.when(one, now, said), meant);

        const ada = { name: "Ada Lovelace", email: "ada@example.com" };
        const grace = { name: "Grace Hopper", email: "grace@example.com", answer: "yes" };
        const theirs = fields => Object.assign({ holds: false, organizer: ada, guests: [] },
            fields);
        // Who a call was with: [what, the meeting's fields, said]
        const withs = [
            ["nobody, his own", { holds: true, organizer: null }, ""],
            ["who holds it", theirs(), "Ada Lovelace"],
            ["who holds it and a guest", theirs({ guests: [grace] }),
                "Ada Lovelace and Grace Hopper"],
            ["a guest of his own", { holds: true, organizer: null, guests: [grace] },
                "Grace Hopper"],
            ["three, one by address, and not one who said no", theirs({ guests: [grace,
                { name: "", email: "alan@example.com", answer: "waiting" },
                { name: "Charles Babbage", email: "charles@example.com", answer: "no" }] }),
                "Ada Lovelace and 2 others"]
        ];
        for (const [what, fields, meant] of withs)
            check(`with ${what}`, Meet.withWhom(meeting("call", at(25, 9, 0), at(25, 9, 30),
                fields), said), meant);
        check("a call's side, who and when", Meet.heldSide(meeting("retro", at(25, 9, 0),
            at(25, 9, 30), theirs({ guests: [grace] })), now, said),
            "Ada Lovelace and Grace Hopper \u00b7 yesterday 09:00");
        check("a call's side with nobody else, when alone", Meet.heldSide(meeting("alone",
            at(26, 12, 0), at(26, 12, 30), { holds: true }), now, said), "12:00");

        // The calls among the meetings held, newest first: each on Meet
        // once by its code, where it was newest, whatever follows the link.
        const standup = "https://meet.google.com/abc-defg-hij";
        const held = [
            meeting("standup today", at(26, 9, 0), at(26, 9, 15), { link: standup }),
            meeting("review", at(25, 16, 0), at(25, 16, 30), { link: null, kind: null }),
            meeting("standup yesterday", at(25, 9, 0), at(25, 9, 15),
                { link: `${standup}?authuser=0` }),
            meeting("pairing", at(24, 9, 0), at(24, 9, 30),
                { link: "https://tuple.app/c/abc", kind: "tuple" }),
            meeting("retro", at(23, 9, 0), at(23, 9, 30),
                { link: "https://meet.google.com/xyz-abcd-efg" })
        ];
        check("Meet's calls, once by code, newest first", Meet.called(held)
            .map(call => [call.code, call.meeting.id]),
            [["abc-defg-hij", "standup today"], ["xyz-abcd-efg", "retro"]]);
        check("no calls among none held", Meet.called([]), []);
        check("at most nine calls", Meet.called(Array.from({ length: 12 }, (_, day) => meeting(
            `call ${day}`, at(26, 9, 0) - day * 86_400_000, at(26, 9, 30) - day * 86_400_000, {
                link: `https://meet.google.com/abc-defg-h${String.fromCharCode(97 + day)}a`
            }))).length, 9);
    }
}
