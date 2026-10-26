// Which meeting is next, when it is in words and whether it's about to
// start, from meetings like the google program's, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
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

        // About to start, from two minutes before to five after.
        const standup = meeting("standup", at(26, 15, 30), at(26, 15, 45));
        const about = [
            [at(26, 15, 27), false], [at(26, 15, 28), true], [at(26, 15, 30), true],
            [at(26, 15, 35), true], [at(26, 15, 35) + 1, false]
        ];
        for (const [time, meant] of about)
            check(`about to start at ${new Date(time).toTimeString()}`,
                Calendar.aboutToStart(standup, time), meant);
        check("nothing is about to start", Calendar.aboutToStart(null, now), false);

        // What the socket says.
        check("a state line", Calendar.heard(
            '{"account":"ready","meetings":[],"heard":1793042446024}'),
            { account: "ready", meetings: [], heard: 1793042446024 });
        check("a state with nothing heard yet", Calendar.heard(
            '{"account":"none","meetings":[],"heard":null}'),
            { account: "none", meetings: [], heard: null });
        check("half a line", Calendar.heard('{"account":"rea'), null);
        check("a line that isn't the state", Calendar.heard('{"hello":1}'), null);

        for (const failure of failures)
            console.warn(failure);
        console.log(`${asked - failures.length} of ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
