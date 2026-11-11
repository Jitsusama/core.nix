// The cases of times for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Zones.js" as Zones

// What's typed, when (in UTC), and the time there, from the zones
// zones.py built, told against Toronto.
const times = [
    ["time in tokyo", "2026-01-15T12:00", "21:00"],
    ["london time", "2026-01-15T12:00", "12:00"],
    ["london time", "2026-07-15T12:00", "13:00"],
    ["time in the uk", "2026-07-15T12:00", "13:00"],
    ["time", "2026-01-15T12:00", "07:00"],
    ["time", "2026-07-15T12:00", "08:00"],
    ["what time is it in india?", "2026-01-15T12:00", "17:30"],
    ["kathmandu time", "2026-01-15T12:00", "17:45"],
    ["time in New York", "2026-01-15T12:00", "07:00"],
    ["pst time", "2026-01-15T12:00", "04:00"],
    ["time in south korea", "2026-01-15T12:00", "21:00"],
    ["time in utc", "2026-01-15T12:00", "12:00"],
    ["time in auckland", "2026-01-15T12:00", "01:00"],
    // Toronto's clocks go forward at 02:00 on 8 March 2026, 07:00 UTC.
    ["time", "2026-03-08T06:59", "01:59"],
    ["time", "2026-03-08T07:00", "03:00"]
];

const notTimes = ["bedtime", "uptime", "timer", "bed time", "kitty",
    "time out"];

function run(failures, fromRoot) {
    let asked = 0;
    const quoted = Common.quoted;
    const at = Common.at;
    const read = new XMLHttpRequest();
    read.open("GET", fromRoot.url("zones.jsonl"), false);
    read.send();
    const zones = Zones.prepare(read.responseText);
    for (const [typed, when, time] of times) {
        asked++;
        const got = Zones.answer(typed, zones, at(when), "America/Toronto", 0);
        if (!got || got.headline !== time) {
            failures.push(`"${typed}" at ${when} said ${quoted(got?.headline)}, `
                + `not "${time}"`);
        }
    }
    for (const typed of notTimes) {
        asked++;
        const got = Zones.answer(typed, zones, at("2026-01-15T12:00"), "America/Toronto", 0);
        if (got) {
            failures.push(`"${typed}" told ${quoted(got)}, but nothing should`);
        }
    }
    // The day of the year, in a zone with summer time (the check runs
    // in Toronto's): either side of the spring change, and the last.
    for (const [year, month, date, day] of [[2026, 0, 1, 1], [2026, 2, 7, 66],
            [2026, 2, 9, 68], [2026, 9, 22, 295], [2024, 11, 31, 366]]) {
        asked++;
        const got = Zones.dayOfYear(new Date(year, month, date));
        if (got !== day) {
            failures.push(`${year}-${month + 1}-${date} was day ${got}, not ${day}`);
        }
    }
    const told = Zones.answer("time in auckland", zones, at("2026-01-15T12:00"),
        "America/Toronto", 0);
    asked++;
    // Each row a part, its value, then its note.
    const lines = answer => answer.rows.map(r => [r.part, r.text, r.note].filter(s => s)
        .join(" "));
    if (!told || quoted(lines(told)) !== quoted(["day Friday 16 January tomorrow",
            "apart 18 h ahead", "zone Pacific/Auckland NZDT, UTC+13"])) {
        failures.push(`"time in auckland" gave the rows ${quoted(told?.rows)}`);
    }
    // Here, the day needs no "today", and UTC is said once.
    const cases = [
        ["time", ["day Thursday 15 January", "zone America/Toronto EST, UTC\u22125"]],
        ["time in utc", ["day Thursday 15 January today", "apart 5 h ahead", "zone UTC"]],
        ["time in kathmandu", ["day Thursday 15 January today", "apart 10 h 45 min ahead",
            "zone Asia/Kathmandu UTC+5:45"]]
    ];
    for (const [typed, expected] of cases) {
        asked++;
        const got = Zones.answer(typed, zones, at("2026-01-15T12:00"), "America/Toronto", 0);
        if (!got || quoted(lines(got)) !== quoted(expected)) {
            failures.push(`"${typed}" gave the rows ${quoted(got && lines(got))}`);
        }
    }
    asked++;
    const away = Zones.answer("time in narnia", zones, at("2026-01-15T12:00"),
        "America/Toronto", 0);
    if (away?.unknown !== "narnia") {
        failures.push(`"time in narnia" gave ${quoted(away)}, not a place to look up`);
    }
    asked++;
    const change = Zones.nextChange(zones.zones["America/Toronto"],
        at("2026-11-04T22:00") / 1000);
    if (!change || new Date(change.when).toISOString() !== "2027-03-14T07:00:00.000Z"
        || change.abbreviation !== "EDT" || change.minutes !== -240) {
        failures.push(`Toronto next changed at ${quoted(change)}`);
    }
    return asked;
}
