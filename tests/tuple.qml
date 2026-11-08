// How the shell puts a Tuple call in words, finds a call's link in text,
// and orders and words Tuple's Recent, run in Qt's own JavaScript engine,
// the one the shell runs it in. The `the-shell-follows-tuple-calls` check
// copies Tuple.js and When.js beside this file and runs `qml -platform
// offscreen tuple.qml`.
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

        // Recent's words, as words.json's tuple:call says them, with the
        // locale's day names, as Calls.qml gives them.
        const said = {
            someone: "Someone",
            missed: "missed",
            two: "{first} and {second}",
            other: "{first} and one other",
            others: "{first} and {count} others",
            yesterday: "yesterday",
            under: "under a minute",
            minutes: "{minutes} min",
            hours: "{hours} h",
            hoursMinutes: "{hours} h {minutes} min",
            seconds: "{seconds} s",
            with: "With",
            email: "Email",
            emails: "Emails",
            length: "Length",
            rang: "Rang For",
            ended: "Ended",
            date: (date, format) => Qt.formatDate(date, format)
        };
        const named = (...names) => names.map((name, at) => ({ id: String(at), name: name,
            email: name === "" ? "" : `${name.toLowerCase()}@example.com` }));
        const nobody = { id: "", name: "", email: "" };

        // [people, said]
        const whos = [
            [named("Ana"), "Ana"],
            [named("Ana", "Ben"), "Ana and Ben"],
            [named("Ana", ""), "Ana and one other"],
            [named("", "Ben", "Cy"), "Ben and 2 others"],
            [[{ id: "7", name: "", email: "ada@example.com" }], "ada@example.com"],
            [[nobody], "Someone"],
            [[], "Someone"],
            [[nobody, nobody, nobody], "Someone and 2 others"]
        ];
        for (const [people, expected] of whos) {
            asked++;
            const got = Tuple.who(people, said);
            if (got !== expected)
                failures.push(`${JSON.stringify(people)} was named "${got}", not "${expected}"`);
        }

        // Local times, so the days read the same in any zone the check
        // runs in: now is Tuesday 3 November 2026 at 16:00.
        const on = (month, day, hour, minute) => new Date(2026, month, day, hour, minute)
            .getTime();
        const at = (day, hour, minute) => on(10, day, hour, minute);
        const now = at(3, 16, 0);
        // [what it was, since, ended, missed, said]
        const notes = [
            ["42 min yesterday", at(2, 14, 10), at(2, 14, 52), false,
                "42 min \u00b7 yesterday 14:10"],
            ["a missed call this morning", at(3, 9, 12), at(3, 9, 13), true,
                "missed \u00b7 09:12"],
            ["under a minute", at(3, 15, 0), at(3, 15, 0) + 59_000, false,
                "under a minute \u00b7 15:00"],
            ["an hour on Sunday", at(1, 10, 0), at(1, 11, 0), false, "1 h \u00b7 Sunday 10:00"],
            ["an hour and a bit in September", on(8, 21, 8, 5), on(8, 21, 9, 10), false,
                "1 h 5 min \u00b7 21 Sep 08:05"],
            ["a year ago", new Date(2025, 10, 3, 12, 0).getTime(),
                new Date(2025, 10, 3, 12, 30).getTime(), false, "30 min \u00b7 3 Nov 2025 12:00"]
        ];
        for (const [what, since, ended, missed, expected] of notes) {
            asked++;
            const got = Tuple.note({ people: [], since: since, ended: ended, missed: missed },
                now, said);
            if (got !== expected)
                failures.push(`${what} said "${got}", not "${expected}"`);
        }

        // A drawer holds every figure kept: the day it began, everyone by
        // name and address, how long it went on or rang, and its end.
        // [what it was, past, drawer]
        const drawers = [
            ["a call with two", { people: named("Ana", "Ben"), since: at(2, 14, 10),
                ended: at(2, 14, 52), missed: false }, {
                eyebrow: "Monday 2 November \u00b7 14:10", title: "Ana and Ben",
                facts: [{ label: "With", value: "Ana, Ben" },
                    { label: "Emails", value: "ana@example.com, ben@example.com" },
                    { label: "Length", value: "42 min" },
                    { label: "Ended", value: "yesterday 14:52" }],
                body: ""
            }],
            ["a missed call last year", { people: named("Ana"),
                since: new Date(2025, 10, 3, 9, 12, 0).getTime(),
                ended: new Date(2025, 10, 3, 9, 12, 42).getTime(), missed: true }, {
                eyebrow: "Monday 3 November 2025 \u00b7 09:12", title: "Ana",
                facts: [{ label: "With", value: "Ana" },
                    { label: "Email", value: "ana@example.com" },
                    { label: "Rang For", value: "42 s" },
                    { label: "Ended", value: "3 Nov 2025 09:12" }],
                body: ""
            }],
            ["a missed call from nobody known", { people: [], since: at(3, 9, 12),
                ended: at(3, 9, 13), missed: true }, {
                eyebrow: "Tuesday 3 November \u00b7 09:12", title: "Someone",
                facts: [{ label: "Rang For", value: "60 s" },
                    { label: "Ended", value: "09:13" }],
                body: ""
            }]
        ];
        for (const [what, past, expected] of drawers) {
            asked++;
            const got = JSON.stringify(Tuple.details(past, now, said));
            if (got !== JSON.stringify(expected))
                failures.push(`${what}'s drawer held ${got}, not ${JSON.stringify(expected)}`);
        }

        // Newest first by when each began, as their times read down, a
        // missed call that rang during a call before the call, and what
        // isn't a call left out.
        asked++;
        const past = (since, missed) => ({ people: named("Ana"), since: since,
            ended: since + 600_000, missed: missed });
        const order = Tuple.newestFirst([past(1000, false), null, past(3000, true),
            { people: [], since: "soon" }, past(2000, false), { since: 4000, ended: 5000 }])
            .map(one => `${one.since}${one.missed ? " missed" : ""}`).join(", ");
        if (order !== "3000 missed, 2000, 1000")
            failures.push(`Recent read "${order}", not "3000 missed, 2000, 1000"`);
        asked++;
        if (Tuple.newestFirst(undefined).length !== 0)
            failures.push("a record with no Recent listed some");

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
