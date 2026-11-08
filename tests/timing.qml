// What Timers works out, run in Qt's own JavaScript engine, the one the
// shell runs it in. The `a-timer-is-read-from-what-is-typed` check copies
// modules/quickshell/launcher/Timing.js beside this file and runs
// `qml -platform offscreen timing.qml`: a span and a name are read from
// what's typed after "timer" in every way Joel might type them, the time
// left is told as a clock and a span in its largest units, and timers
// are listed rung first, then by what they have left.
import QtQuick
import "Timing.js" as Timing

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

    function check(failures, what, got, want) {
        if (JSON.stringify(got) !== JSON.stringify(want))
            failures.push(`${what}: got ${JSON.stringify(got)}, wanted ${JSON.stringify(want)}`);
    }

    function run() {
        const failures = [];
        const second = 1000;
        const minute = 60 * second;
        const hour = 60 * minute;
        const read = typed => {
            const found = Timing.read(typed);
            return found ? [found.span, found.name, found.unfit] : null;
        };

        check(failures, "minutes and a name", read("10m tea"), [10 * minute, "tea", false]);
        check(failures, "a unit spaced from its number", read("10 min tea"),
            [10 * minute, "tea", false]);
        check(failures, "a unit in full", read("5 minutes"), [5 * minute, "", false]);
        check(failures, "a bare number is minutes", read("10"), [10 * minute, "", false]);
        check(failures, "seconds", read("90s"), [90 * second, "", false]);
        check(failures, "hours and minutes run together", read("1h30m pasta"),
            [90 * minute, "pasta", false]);
        check(failures, "hours and minutes apart", read("1 h 30 min"), [90 * minute, "", false]);
        check(failures, "a fraction", read("1.5h"), [90 * minute, "", false]);
        check(failures, "a clock's minutes and seconds", read("1:30 eggs"),
            [90 * second, "eggs", false]);
        check(failures, "a clock with hours", read("1:02:03"),
            [hour + 2 * minute + 3 * second, "", false]);
        check(failures, "the name before the span", read("Tea 10m"),
            [10 * minute, "Tea", false]);
        check(failures, "a name keeps its case and its words", read("10m Green Tea"),
            [10 * minute, "Green Tea", false]);
        check(failures, "a unit running into a word is a name", read("tea 10 mice"), null);
        // A name that starts as a unit does leaves the number bare, in
        // minutes (Y2YBTS's review of #2238).
        check(failures, "a name starting as seconds do", read("10 soup"),
            [10 * minute, "soup", false]);
        check(failures, "a name starting as hours do", read("5 hot dogs"),
            [5 * minute, "hot dogs", false]);
        check(failures, "a name starting as minutes do", read("10 mussels"),
            [10 * minute, "mussels", false]);
        check(failures, "a bare number after hours is minutes", read("1h 30"),
            [90 * minute, "", false]);
        check(failures, "but after minutes it's the name's", read("10m 2 eggs"),
            [10 * minute, "2 eggs", false]);
        check(failures, "a clock past its minutes is unfit", read("1:75"), [0, "", true]);
        check(failures, "no span", read("tea"), null);
        check(failures, "nothing typed", read(""), null);
        check(failures, "nothing is unfit", read("0m"), [0, "", true]);
        check(failures, "past a week is unfit", read("200h"), [200 * hour, "", true]);

        check(failures, "a running timer has its end less now left",
            Timing.left({ state: "running", ends: 5000 }, 1000), 4000);
        check(failures, "and none once past it",
            Timing.left({ state: "running", ends: 5000 }, 9000), 0);
        check(failures, "a paused one what it held",
            Timing.left({ state: "paused", left: 7000 }, 1000), 7000);
        check(failures, "a rung one nothing", Timing.left({ state: "rung", rang: 1 }, 9), 0);

        check(failures, "a clock starts on its whole span", Timing.clock(10 * minute), "10:00");
        check(failures, "counts whole seconds up", Timing.clock(251 * second + 1), "4:12");
        check(failures, "shows its last second", Timing.clock(1), "0:01");
        check(failures, "ends at nothing", Timing.clock(0), "0:00");
        check(failures, "past an hour with hours", Timing.clock(hour + 4 * minute + 12 * second),
            "1:04:12");

        const units = { hours: "h", minutes: "min", seconds: "s" };
        check(failures, "a span in minutes", Timing.said(10 * minute, units), "10 min");
        check(failures, "in hours and minutes", Timing.said(90 * minute, units), "1 h 30 min");
        check(failures, "in minutes and seconds", Timing.said(90 * second, units), "1 min 30 s");

        const listed = Timing.ordered([
            { key: "timer:1", state: "paused", left: 5 * minute, set: 1 },
            { key: "timer:2", state: "running", ends: 9 * minute, set: 2 },
            { key: "timer:3", state: "rung", rang: 100, set: 3 },
            { key: "timer:4", state: "running", ends: 3 * minute, set: 4 },
            { key: "timer:5", state: "rung", rang: 200, set: 5 }
        ], 0).map(record => record.key);
        check(failures, "rung newest first, then running by the soonest end, then paused",
            listed, ["timer:5", "timer:3", "timer:4", "timer:2", "timer:1"]);

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            Qt.exit(1);
            return;
        }
        Qt.exit(0);
    }
}
