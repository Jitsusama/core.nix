// The cases of Now's Time, Date and Weather drawers (launcher/NowDrawers.js),
// which ../answers.qml runs with every other place's; its opening comment
// says how. The words are words.json's for Time, Date and Weather.
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

const WEATHER_SAID = {
    range: "{low} to {high}",
    lead: "{sky} at {warmth}, feeling like {feels}",
    leadPlain: "{sky} at {warmth}",
    rainFrom: "rain likely from {clock}",
    rainNow: "raining now",
    dry: "no rain due in the next {hours} hours"
};
const WEATHER_NAMES = {
    title: "Weather", place: "Forecast For", next: "Next {hours} Hours", rain: "Rain"
};

// A Saturday afternoon in Toronto, rain due from three, and its week.
function afternoon(change) {
    return NowDrawers.weather(Object.assign({
        sky: "mostly clear", warmth: "12\u00b0", feels: "10\u00b0",
        place: "Toronto, from your time zone", hours: 12, low: "6\u00b0", high: "12\u00b0",
        rainFrom: "15:00", rain: "from 15:00 \u00b7 70%",
        days: [
            { name: "Today", low: "6\u00b0", high: "14\u00b0", sky: "light rain" },
            { name: "Tomorrow", low: "3\u00b0", high: "9\u00b0", sky: "overcast" },
            { name: "Monday", low: "2\u00b0", high: "11\u00b0", sky: "clear" }
        ],
        said: WEATHER_SAID, names: WEATHER_NAMES
    }, change));
}

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

    // Weather's: what it's like now and the rain to come in a sentence,
    // then where, the next hours, the rain while it's due, and each day
    // as its row says it.
    const wet = afternoon({});
    same("Weather's drawer's sentence", wet.body,
        "Mostly clear at 12\u00b0, feeling like 10\u00b0; rain likely from 15:00.");
    same("Weather's drawer's facts", facts(wet), [
        "Forecast For: Toronto, from your time zone",
        "Next 12 Hours: 6\u00b0 to 12\u00b0",
        "Rain: from 15:00 \u00b7 70%",
        "Today: 6\u00b0 to 14\u00b0 \u00b7 light rain",
        "Tomorrow: 3\u00b0 to 9\u00b0 \u00b7 overcast",
        "Monday: 2\u00b0 to 11\u00b0 \u00b7 clear"
    ]);
    same("Weather's drawer is prose under its title", [wet.title, wet.prose, wet.nerd.length],
        ["Weather", true, 0]);
    same("rain already falling says so", afternoon({ rainFrom: "now" }).body,
        "Mostly clear at 12\u00b0, feeling like 10\u00b0; raining now.");

    // With no rain due, Rain gives up its fact, as its row gives up its
    // place, and the sentence says none is due.
    const dry = afternoon({ rainFrom: "", rain: "" });
    same("no rain due is said", dry.body,
        "Mostly clear at 12\u00b0, feeling like 10\u00b0; no rain due in the next 12 hours.");
    same("no rain due has no Rain fact", facts(dry).some(fact => fact.startsWith("Rain")),
        false);

    // A forecast with no feeling or no hours says what it has.
    same("no feeling says the warmth alone", afternoon({ feels: null }).body,
        "Mostly clear at 12\u00b0; rain likely from 15:00.");
    same("no hours have no range", facts(afternoon({ low: null, high: null }))[1],
        "Rain: from 15:00 \u00b7 70%");
    return asked;
}
