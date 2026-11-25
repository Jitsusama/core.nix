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
    return asked + twoChanges(failures, zones);
}

// A zone's next two changes as its drawer says them, at fixed moments so
// no case depends on the day it runs: one a year ahead, a southern zone's
// order, a zone with none, and one past whose next change the years kept
// hold no more. That last case's year is zones.nix's last, 2045, which
// time.json's "none before 2046" names too: move all three together.
function twoChanges(failures, zones) {
    const quoted = Common.quoted;
    const at = Common.at;
    const iso = change => change && change.when !== null
        ? new Date(change.when).toISOString() : null;
    const turns = change => change ? change.note : null;
    const cases = [
        ["America/Toronto", "2026-11-04T22:00", "2027-03-14T07:00:00.000Z",
            "2027-11-07T06:00:00.000Z", "EDT, UTC\u22124", "EST, UTC\u22125"],
        ["Australia/Sydney", "2026-01-15T12:00", "2026-04-04T16:00:00.000Z",
            "2026-10-03T16:00:00.000Z", "AEST, UTC+10", "AEDT, UTC+11"],
        // Tokyo never changes: Next Change says so, and After That is left
        // out, so it reads as a zone with no changes rather than one more.
        ["Asia/Tokyo", "2026-01-15T12:00", null, "left out", "", "left out"],
        // Past the last change kept, After That says the years end.
        ["America/Toronto", "2045-04-01T12:00", "2045-11-05T06:00:00.000Z", null,
            "EST, UTC\u22125", ""],
        // A zone tzdata names by offset alone says the offset once.
        ["America/Santiago", "2026-01-15T12:00", "2026-04-05T03:00:00.000Z",
            "2026-09-06T04:00:00.000Z", "UTC\u22124", "UTC\u22123"]
    ];
    for (const [id, when, next, after, nextTurns, afterTurns] of cases) {
        const said = Zones.changesSaid(Zones.nextTwo(zones.zones[id], at(when) / 1000));
        const gone = said.after === null ? "left out" : null;
        const got = [iso(said.next), gone ?? iso(said.after), turns(said.next),
            gone ?? turns(said.after)];
        if (JSON.stringify(got) !== JSON.stringify([next, after, nextTurns, afterTurns])) {
            failures.push(`${id}'s two changes from ${when} were ${quoted(got)}`);
        }
    }
    return cases.length;
}
