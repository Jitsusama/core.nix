// Whether Joel's in a Tuple call, from lines like the daemon's log, run in
// Qt's own JavaScript engine, the one the shell runs it in. The
// `the-shell-follows-tuple-calls` check copies Tuple.js beside this file and
// runs `qml -platform offscreen tuple.qml`. The lines of a call's start and
// end are the daemon's own, as a call with nobody else in it wrote them;
// those of other people, a ring and a mute are its messages with their
// blanks filled in, as nobody has yet seen them filled.
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
        const none = Tuple.none;
        const heard = lines => lines.reduce((call, each) => Tuple.after(call, each, now), none);

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
            const call = heard(lines);
            if (call.calling !== calling || call.since !== since)
                failures.push(`${what}: calling ${call.calling} since ${call.since}, `
                    + `not ${calling} since ${since}`);
        }

        const connected = line("02:05:41.512", "call connected");
        // [what happens, the lines written after the call connects, what's
        // asked of the call after, what it should say]
        const people = [
            ["someone joining", [line("02:06:00.000", "peer joined call: 117559")],
                call => Tuple.others(call), ["117559"]],
            ["someone joining and leaving", [
                line("02:06:00.000", "peer joined call: 117559"),
                line("02:07:00.000", "peer left call: 117559")
            ], call => Tuple.others(call), []],
            ["the engine's way of saying it", [
                line("02:06:00.000", "Call.cpp:1|EngineEvent: OnPeerAdded (peer 117559 cursor_mode=1 persist_paint=0)"),
                line("02:06:01.000", "EngineEvent: OnPeerAdded (peer 165856 cursor_mode=1 persist_paint=0)"),
                line("02:07:00.000", "EngineEvent: OnPeerRemoved (peer 117559 reason=0 'left')")
            ], call => Tuple.others(call), ["165856"]],
            ["both ways at once count each person once", [
                line("02:06:00.000", "peer joined call: Aadi"),
                line("02:06:00.010", "EngineEvent: OnPeerAdded (peer 117559 cursor_mode=1 persist_paint=0)")
            ], call => Tuple.others(call).length, 1],
            ["the call ending lets everyone go", [
                line("02:06:00.000", "peer joined call: 117559"),
                line("02:16:00.000", "call ended")
            ], call => Tuple.others(call), []],
            ["connecting again keeps who's there", [
                line("02:06:00.000", "peer joined call: 117559"),
                line("02:10:41.512", "call connected")
            ], call => Tuple.others(call), ["117559"]],
            ["Tuple keeping Joel's voice from the call", [
                line("02:06:00.000", "local user 132786 sending audio: false")
            ], call => call.muted, true],
            ["Tuple sending it again", [
                line("02:06:00.000", "local user 132786 sending audio: false"),
                line("02:07:00.000", "local user 132786 sending audio: true")
            ], call => call.muted, false],
            ["a mute not yet said", [], call => call.muted, null],
            ["someone sharing their screen", [
                line("02:06:00.000", "peer started sharing screen: 117559")
            ], call => call.showing, ["117559"]],
            ["and stopping", [
                line("02:06:00.000", "peer started sharing screen: 117559"),
                line("02:07:00.000", "peer stopped sharing screen: 117559")
            ], call => call.showing, []]
        ];
        for (const [what, lines, asking, expected] of people) {
            asked++;
            const got = asking(heard([connected].concat(lines)));
            if (JSON.stringify(got) !== JSON.stringify(expected))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(expected)}`);
        }

        // [what happens, the lines written, who's ringing after]
        const rings = [
            ["a call ringing", [line("02:17:00.000", "received incoming call: 117559")], "117559"],
            ["Joel answering it", [
                line("02:17:00.000", "received incoming call: 117559"),
                line("02:17:05.000", "accepting call")
            ], ""],
            ["Tuple answering it with others there", [
                line("02:17:00.000", "received incoming call: 117559"),
                line("02:17:05.000", "answering call with 2 peers")
            ], ""],
            ["it connecting", [
                line("02:17:00.000", "received incoming call: 117559"),
                line("02:17:05.000", "call connected")
            ], ""],
            ["the caller giving up", [
                line("02:17:00.000", "received incoming call: 117559"),
                line("02:17:20.000", "invalidating call")
            ], ""],
            ["Tuple turning it away", [
                line("02:17:00.000", "received incoming call: 117559"),
                line("02:17:00.100", "ignoring incoming call: 117559")
            ], ""]
        ];
        for (const [what, lines, ringing] of rings) {
            asked++;
            const call = heard(lines);
            if (call.ringing !== ringing)
                failures.push(`${what}: ringing "${call.ringing}", not "${ringing}"`);
        }
        // [a ring's last line, whether it was answered rather than missed]
        const endings = [
            ["accepting call", true],
            ["answering call with 2 peers", true],
            ["auto-answering offer (with 1 others)", true],
            ["call connected", true],
            ["ignoring incoming call: 117559", false],
            ["invalidating call", false],
            ["call ended", false]
        ];
        for (const [said, answered] of endings) {
            asked++;
            if (Tuple.answering(line("02:17:05.000", said)) !== answered)
                failures.push(`"${said}" ${answered ? "didn't answer" : "answered"} a ring`);
        }

        asked++;
        const rang = heard([line("02:17:00.000", "received incoming call: 117559")]).rang;
        if (rang !== now - 41512)
            failures.push(`a ring rang at ${rang}, not ${now - 41512}`);

        // [what fills a line's blank, the name tuple ls gives it]
        const listing = "  117559 Aadi Sanghani <aadi@example.com> [offline]\n"
            + "  165856 Aadya Garg <aadya@example.com> [online]\n";
        const names = [
            ["117559", "Aadi Sanghani"],
            [" 165856 ", "Aadya Garg"],
            ["User { id: 165856, name: ... }", "Aadya Garg"],
            ["1175", ""],
            ["99999", ""],
            ["Aadi", ""],
            ["", ""]
        ];
        for (const [who, expected] of names) {
            asked++;
            const got = Tuple.named(listing, who);
            if (got !== expected)
                failures.push(`"${who}" was named "${got}", not "${expected}"`);
        }

        // [names, "" for one not known, said]
        const companies = [
            [[], "on your own"],
            [["Aadi"], "with Aadi"],
            [[""], "with someone"],
            [["Aadi", "Ben"], "with Aadi and Ben"],
            [["Aadi", ""], "with Aadi and one other"],
            [["", "Ben", "Cy"], "with Ben and 2 others"],
            [["", "", ""], "with 3 others"]
        ];
        for (const [names, expected] of companies) {
            asked++;
            const got = Tuple.company(names);
            if (got !== expected)
                failures.push(`${JSON.stringify(names)} said "${got}", not "${expected}"`);
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
