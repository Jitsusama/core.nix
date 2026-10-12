// Whether Joel's in a Tuple call, from lines like the daemon's log, run in
// Qt's own JavaScript engine, the one the shell runs it in. The
// `the-shell-follows-tuple-calls` check copies Tuple.js beside this file and
// runs `qml -platform offscreen tuple.qml`. The lines are the daemon's own,
// as a call with nobody else in it wrote them.
import QtQuick
import "Tuple.js" as Tuple

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
        // 22:17:41 in Toronto, 02:17:41 in UTC, as the log writes it.
        const now = Date.UTC(2026, 9, 12, 2, 17, 41, 512);
        const minute = 60 * 1000;
        const line = (time, message) => `${time}|940703|940703|info|${message}`;
        const none = { calling: false, since: 0 };

        // [what happens, the lines written, in a call after, since when]
        const cases = [
            ["a call connecting", [line("02:05:41.512", "call connected")],
                true, now - 12 * minute],
            ["the lines around it aren't calls", [
                line("02:05:40.002", "Engine.cpp:719|cli: new"),
                line("02:05:40.104", "Engine.cpp:876|CallerQueue: URL"),
                line("02:05:41.400", "user 132786 color Green (local)")
            ], false, 0],
            ["Joel ending it", [
                line("02:05:41.512", "call connected"),
                line("02:16:00.000", "Engine.cpp:552|cli: end"),
                line("02:16:00.010", "Caller.cpp:985|invalidating call")
            ], false, 0],
            ["the other side ending it", [
                line("02:05:41.512", "call connected"),
                line("02:16:00.000", "call ended")
            ], false, 0],
            ["the daemon starting again", [
                line("02:05:41.512", "call connected"),
                line("02:16:00.000", "daemon loop started, waiting for events...")
            ], false, 0],
            ["the daemon stopping", [
                line("02:05:41.512", "call connected"),
                line("02:16:00.000", "Engine.cpp:337|received 'off' message, quitting")
            ], false, 0],
            ["connecting again keeps when it began", [
                line("02:05:41.512", "call connected"),
                line("02:10:41.512", "call connected")
            ], true, now - 12 * minute],
            ["a call from before midnight in UTC", [line("23:59:41.512", "call connected")],
                true, now - (2 * 60 + 18) * minute],
            ["a line ending in a carriage return", [line("02:05:41.512", "call connected\r")],
                true, now - 12 * minute],
            ["a line with no time", ["call connected"], true, now]
        ];
        for (const [what, lines, calling, since] of cases) {
            asked++;
            const call = lines.reduce((call, each) => Tuple.after(call, each, now), none);
            if (call.calling !== calling || call.since !== since)
                failures.push(`${what}: calling ${call.calling} since ${call.since}, `
                    + `not ${calling} since ${since}`);
        }

        // [text, the link in it]
        const links = [
            ["can you pair? https://tuple.app/c/abcDEF123 thanks", "https://tuple.app/c/abcDEF123"],
            ["https://production.tuple.app/c/x_y-z", "https://production.tuple.app/c/x_y-z"],
            ["personal url: https://tuple.app/c/room1\r", "https://tuple.app/c/room1"],
            ["https://tuple.app/pricing", ""],
            ["http://tuple.app/c/abc", ""],
            ["", ""],
            [undefined, ""]
        ];
        for (const [text, expected] of links) {
            asked++;
            const got = Tuple.link(text);
            if (got !== expected)
                failures.push(`"${text}" gave "${got}", not "${expected}"`);
        }

        // [minutes, said]
        const lengths = [
            [0, "under a minute"],
            [1, "a minute"],
            [12, "12 minutes"],
            [59, "59 minutes"],
            [60, "1 h"],
            [125, "2 h 5 min"]
        ];
        for (const [minutes, expected] of lengths) {
            asked++;
            const got = Tuple.length(minutes);
            if (got !== expected)
                failures.push(`${minutes} minutes said "${got}", not "${expected}"`);
        }

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
