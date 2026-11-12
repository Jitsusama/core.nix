// The cases of Now's Time and Date drawers (launcher/NowDrawers.js), which
// ../answers.qml runs with every other place's; its opening comment says how.
// The words are words.json's for Time and Date.
.import "common.js" as Common
.import "../NowDrawers.js" as NowDrawers

const TIME_SAID = {
    inZone: "{time} in {zone}'s zone, {offset}",
    here: "{time}, {offset}",
    kept: "the clock is kept true",
    unkept: "the clock isn't kept true yet",
    none: "nothing keeps the clock true"
};
const TIME_NAMES = {
    title: "Time", zone: "Zone", utc: "UTC", clock: "Clock", unix: "Unix Time",
    julian: "Julian Day"
};
const DATE_SAID = {
    lead: "week {week}, day {day} of {days}, with {left} left in the year; "
        + "{season} has {seasonLeft} to go.",
    day: "{count} day",
    days: "{count} days",
    left: "{days} left",
    quarterDay: "day {day} of {days}"
};
const DATE_NAMES = {
    title: "Date", iso: "ISO Date", season: "Season", moon: "Moon", quarter: "Quarter",
    weekDate: "ISO Week Date", julian: "Julian Day"
};

function toronto(clock, clockSide) {
    return NowDrawers.time({
        time: "18:59", zone: "America/Toronto", zoneNote: "EST, UTC\u22125",
        offset: "UTC\u22125", utc: "23:59", clock, clockSide,
        unix: "1794441540", julian: "2461356.49931",
        said: TIME_SAID, names: TIME_NAMES
    });
}

function facts(drawer) {
    return drawer.facts.map(fact => `${fact.label}: ${fact.value}`);
}

function nerd(drawer) {
    return drawer.nerd.map(bit => `${bit.label}: ${bit.value}`);
}

function run(failures) {
    let asked = 0;
    const same = Common.checker(failures, () => {
        asked += 1;
    });

    // VKCF3Z's mock: the time in the zone's own city, how far from UTC,
    // and whether the clock is kept true, then the zone, UTC and the clock.
    const unkept = toronto("none", "nothing keeps the time");
    same("Time's drawer's sentence", unkept.body,
        "18:59 in Toronto's zone, UTC\u22125; nothing keeps the clock true.");
    same("Time's drawer's facts", facts(unkept), [
        "Zone: America/Toronto \u00b7 EST, UTC\u22125",
        "UTC: 23:59",
        "Clock: nothing keeps the time"
    ]);
    same("Time's drawer is prose under its title", [unkept.title, unkept.prose], ["Time", true]);
    // Its raw figures are its nerdy bits, never its facts.
    same("Time's nerdy bits", nerd(unkept),
        ["Unix Time: 1794441540", "Julian Day: 2461356.49931"]);
    same("a kept clock says so", toronto("kept", "synchronised").body,
        "18:59 in Toronto's zone, UTC\u22125; the clock is kept true.");

    // While timekeeping is still asked, the clock is said nowhere, never
    // as a "…" fact.
    const asking = toronto("", "\u2026");
    same("a clock still asked says nothing of it", asking.body,
        "18:59 in Toronto's zone, UTC\u22125.");
    same("a clock still asked has no fact", facts(asking).length, 2);

    // A zone named for no place says only the time and the offset.
    const utc = NowDrawers.time({
        time: "23:59", zone: "UTC", zoneNote: "", offset: "UTC", utc: "23:59",
        clock: "kept", clockSide: "synchronised", said: TIME_SAID, names: TIME_NAMES
    });
    same("a zone of no place says no city", utc.body, "23:59, UTC; the clock is kept true.");
    same("a zone with no note is its name alone", utc.facts[0].value, "UTC");

    // The mock's Wednesday 11 November 2026: week 46, day 315 of 365, 40
    // days of autumn to go, a waxing crescent 7% lit, Q4's day 42 of 92.
    const wednesday = {
        iso: "2026-11-11", week: 46, day: 315, days: 365, season: "autumn", seasonLeft: 40,
        moon: "waxing crescent", lit: "7% lit", quarter: 4, quarterDay: 42, quarterDays: 92,
        weekDate: "2026-W46-3", dayNumber: "2461356",
        said: DATE_SAID, names: DATE_NAMES
    };
    const date = NowDrawers.date(wednesday);
    same("Date's drawer's sentence", date.body,
        "Week 46, day 315 of 365, with 50 days left in the year; autumn has 40 days to go.");
    same("Date's drawer's facts", facts(date), [
        "ISO Date: 2026-11-11",
        "Season: autumn \u00b7 40 days left",
        "Moon: waxing crescent \u00b7 7% lit",
        "Quarter: Q4 \u00b7 day 42 of 92"
    ]);
    same("Date's nerdy bits", nerd(date), ["ISO Week Date: 2026-W46-3", "Julian Day: 2461356"]);

    // One day is said as one, in the sentence and the facts alike.
    const last = NowDrawers.date(Object.assign({}, wednesday, { day: 364, seasonLeft: 1 }));
    same("one day left is a day", last.body,
        "Week 46, day 364 of 365, with 1 day left in the year; autumn has 1 day to go.");
    same("one day of the season is a day", last.facts[1].value, "autumn \u00b7 1 day left");
    return asked;
}
