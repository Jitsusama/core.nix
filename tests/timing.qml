// What Timers works out, run in Qt's own JavaScript engine, the one the
// shell runs it in. The `a-timer-is-read-from-what-is-typed` check copies
// modules/quickshell/launcher/Timing.js beside this file and runs
// `qml -platform offscreen timing.qml`: a span and a name are read from
// what's typed after "timer" in every way Joel might type them, the time
// left is told as a clock and a span in its largest units, and timers
// are listed rung first, then by what they have left; and the stopwatch
// counts on while it runs, shows its count to the tenth, keeps room for
// the widest it reads until it grows a figure, and lists its laps newest
// first; and Now says timers are on while any runs, is
// paused or has rung, or the stopwatch runs, told by the one Timers
// lists first and naming them in that order, the stopwatch last.
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

        check(failures, "a stopwatch unstarted counts nothing",
            Timing.counted({ state: "unstarted", counted: 0, laps: [] }, 9000), 0);
        check(failures, "a stopped one holds its count",
            Timing.counted({ state: "stopped", counted: 4200, laps: [] }, 9000), 4200);
        check(failures, "a running one counts on from what it held",
            Timing.counted({ state: "running", counted: 4200, since: 5000, laps: [] }, 9000),
            8200);
        check(failures, "tenths round down", Timing.tenths(42199), "0:42.1");
        check(failures, "tenths past a minute", Timing.tenths(83400), "1:23.4");
        check(failures, "tenths from nothing", Timing.tenths(0), "0:00.0");
        check(failures, "tenths past an hour with hours",
            Timing.tenths(hour + 2 * minute + 3 * second + 450), "1:02:03.4");
        check(failures, "laps newest first, each its own stretch",
            Timing.laps({ laps: [42100, 80000, 125500] }), [
                { number: 3, lap: 45500, at: 125500 },
                { number: 2, lap: 37900, at: 80000 },
                { number: 1, lap: 42100, at: 42100 }
            ]);
        check(failures, "no laps", Timing.laps({ laps: [] }), []);
        check(failures, "the widest below ten minutes",
            Timing.widest(9 * minute + 59 * second + 900), "9:59.9");
        check(failures, "from ten minutes", Timing.widest(10 * minute), "59:59.9");
        check(failures, "the widest below an hour",
            Timing.widest(59 * minute + 59 * second + 900), "59:59.9");
        check(failures, "from an hour", Timing.widest(hour), "9:59:59.9");
        check(failures, "from ten hours", Timing.widest(10 * hour), "99:59:59.9");
        check(failures, "it grows a figure at ten minutes",
            Timing.grows(9 * minute + 59 * second + 900), 10 * minute);
        check(failures, "and at an hour", Timing.grows(59 * minute + 59 * second + 900), hour);

        const said = { pausedSide: "paused \u00b7 {left}", rung: "rang {clock}",
            late: "rang late, {clock}" };
        const rangAt = new Date(2026, 10, 8, 3, 27).getTime();
        check(failures, "a time of day", Timing.timeOfDay(rangAt), "03:27");
        check(failures, "a running timer's side counts down",
            Timing.side({ state: "running", ends: 9 * minute + 50 * second }, 0, said), "9:50");
        check(failures, "a paused one's says so",
            Timing.side({ state: "paused", left: 9 * minute + 50 * second }, 0, said),
            "paused \u00b7 9:50");
        check(failures, "a rung one's says when",
            Timing.side({ state: "rung", rang: rangAt }, 0, said), "rang 03:27");
        check(failures, "and that it was late",
            Timing.side({ state: "rung", rang: rangAt, late: true }, 0, said),
            "rang late, 03:27");

        const tea = { key: "timer:1", state: "running", ends: 10 * minute, set: 1 };
        const bread = { key: "timer:2", state: "running", ends: 25 * minute, set: 2 };
        const eggs = { key: "timer:3", state: "rung", rang: 100, set: 3 };
        const held = { key: "timer:4", state: "paused", left: minute, set: 4 };
        const running = { key: "stopwatch", state: "running", counted: 0, since: 0, laps: [] };
        const stopped = { key: "stopwatch", state: "stopped", counted: 500, laps: [] };
        const onNow = (timers, watch) => {
            const said = Timing.onNow(timers, watch, 0);
            return [said.on, said.rung, said.first?.key ?? null,
                said.named.map(record => record.key)];
        };
        check(failures, "nothing on", onNow([], stopped), [false, false, null, []]);
        check(failures, "the soonest leads, named soonest first", onNow([bread, tea], null),
            [true, false, "timer:1", ["timer:1", "timer:2"]]);
        check(failures, "a rung one leads them all", onNow([bread, eggs, tea], stopped),
            [true, true, "timer:3", ["timer:3", "timer:1", "timer:2"]]);
        check(failures, "a paused one alone is on", onNow([held], null),
            [true, false, "timer:4", ["timer:4"]]);
        check(failures, "and comes after one running", onNow([held, bread], null),
            [true, false, "timer:2", ["timer:2", "timer:4"]]);
        check(failures, "the stopwatch alone running is on", onNow([], running),
            [true, false, null, ["stopwatch"]]);
        check(failures, "and is named last", onNow([tea], running),
            [true, false, "timer:1", ["timer:1", "stopwatch"]]);
        check(failures, "a running timer ticks Now's row while it's open",
            Timing.ticks([tea], true), true);
        check(failures, "and never while the launcher is closed", Timing.ticks([tea], false),
            false);
        check(failures, "nor while nothing counts down", Timing.ticks([held, eggs], true), false);
        check(failures, "nor for the stopwatch alone, which the launcher draws",
            Timing.ticks([], true), false);

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            Qt.exit(1);
            return;
        }
        Qt.exit(0);
    }
}
