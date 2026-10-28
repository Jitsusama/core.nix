// Which meeting is next, when it is in words, whether it's about to start,
// said as a notice or told already by another, and joined by Return, its day in full, how Joel answered it and
// the line answering it, and which Meet call a window is and whose, from
// meetings like the google program's, run in Qt's own JavaScript engine,
// the one the shell runs it in. The
// `the-shell-tells-the-next-meeting` check copies Calendar.js beside this
// file and runs `qml -platform offscreen calendar.qml`. Times are made on
// the local clock, so the cases hold in whatever zone the check runs in.
import QtQuick
import "Calendar.js" as Calendar

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
        check("the line answering one", Calendar.answer(meeting("abc_1", now, now), "no"),
            '{"answer":{"id":"abc_1","with":"no"}}\n');
        check("the line cancelling one", Calendar.cancel(meeting("abc_1", now, now)),
            '{"cancel":{"id":"abc_1"}}\n');

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
            check(what, Calendar.meetCall(appId), code);
        const onMeet = meeting("on meet", now, now + 30 * minute);
        const pasted = meeting("pasted", now, now + 30 * minute, {
            link: "https://meet.google.com/abc-defg-hijk"
        });
        const another = meeting("another", now, now + 30 * minute, {
            link: "https://meet.google.com/klm-nopq-rst?authuser=1"
        });
        check("a call's meeting", (Calendar.calledBy([pasted, onMeet], "abc-defg-hij") ?? {}).id,
            "on meet");
        check("a call's meeting by a link that says more",
            (Calendar.calledBy([onMeet, another], "klm-nopq-rst") ?? {}).id, "another");
        check("a call on no meeting", Calendar.calledBy([onMeet], "xyz-abcd-efg"), null);
        check("no call on a meeting with no link",
            Calendar.calledBy([meeting("room", now, now, { link: null })], ""), null);

        for (const failure of failures)
            console.warn(failure);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
