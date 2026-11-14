// The drawers of Now's Time, Date and Weather, worked out apart from their
// places (Clocks.qml, Today.qml, Weather.qml) so tests/answers/now-drawers.js holds them in
// Qt's own engine. Each gives a little more than its row's line, never the
// whole place (Joel, 2026-11-11: "side drawers for just about every item
// giving me more details, not as much as if i opened it up"): a sentence
// saying what it means, then the few facts that decide it, most decisive
// first, as VKCF3Z's mock has them (launcher-shape, the Reading kind).
.pragma library

// Words with each {name} filled from `fill`.
function filled(words, fill) {
    let said = String(words ?? "");
    for (const name of Object.keys(fill)) {
        said = said.replace(`{${name}}`, String(fill[name]));
    }
    return said;
}

// Words begun as a sentence: the vocabulary starts no value with a capital.
function sentence(words) {
    return words.charAt(0).toUpperCase() + words.slice(1);
}

// A count of days in the vocabulary's words, one day said as one.
function days(count, said) {
    return filled(count === 1 ? said.day : said.days, { count });
}

// The facts that have a value, a "…" still asking being none yet.
function valued(facts) {
    return facts.filter(fact => fact.value !== "" && fact.value !== "\u2026");
}

// The place a zone is named for, as "America/Toronto" is Toronto's, or ""
// for one named for no place, as "UTC" is.
function cityOf(zone) {
    const named = String(zone ?? "");
    return named.includes("/") ? named.split("/").pop().replace(/_/g, " ") : "";
}

// Time's drawer, from what the clock says: `time`, the time here; `zone`
// and `zoneNote`, the zone the clock keeps and how it stands ("EST,
// UTC−5"); `offset` ("UTC−5"); `utc`, the time there; `clock`, which of
// `said`'s words say how the clock is kept ("kept", "unkept", "none", or
// "" while it's asked); `clockSide`, the Clock row's own side; `unix`
// and `julian`, the moment's raw figures, its nerdy bits; and `names`,
// each fact's label and the title.
function time(at) {
    const said = at.said;
    const city = cityOf(at.zone);
    const where = city === "" ? filled(said.here, { time: at.time, offset: at.offset })
        : filled(said.inZone, { time: at.time, zone: city, offset: at.offset });
    const clock = at.clock === "" ? "" : said[at.clock] ?? "";
    return {
        title: at.names.title,
        body: `${[where, clock].filter(part => part !== "").join("; ")}.`,
        prose: true,
        facts: valued([
            { label: at.names.zone,
                value: [at.zone, at.zoneNote].filter(part => part).join(" \u00b7 ") },
            { label: at.names.utc, value: at.utc },
            { label: at.names.clock, value: at.clockSide }
        ]),
        nerd: [
            { label: at.names.unix, value: at.unix },
            { label: at.names.julian, value: at.julian }
        ]
    };
}

// Date's drawer, from the calendar: `iso`; `week`; `day` of the year's
// `days`; `season` and `seasonLeft`, its days to go; `moon` and `lit`;
// `quarter`, `quarterDay` and `quarterDays`; `weekDate` and `dayNumber`,
// the ISO week date and the Julian day, its nerdy bits; and `said` and
// `names`.
function date(at) {
    const said = at.said;
    const left = at.days - at.day;
    const seasonLeft = days(at.seasonLeft, said);
    return {
        title: at.names.title,
        body: sentence(filled(said.lead, {
            week: at.week, day: at.day, days: at.days,
            left: days(left, said),
            season: at.season,
            seasonLeft
        })),
        prose: true,
        facts: valued([
            { label: at.names.iso, value: at.iso },
            { label: at.names.season,
                value: `${at.season} \u00b7 ${filled(said.left, { days: seasonLeft })}` },
            { label: at.names.moon, value: `${at.moon} \u00b7 ${at.lit}` },
            { label: at.names.quarter,
                value: `Q${at.quarter} \u00b7 ${filled(said.quarterDay,
                    { day: at.quarterDay, days: at.quarterDays })}` }
        ]),
        nerd: [
            { label: at.names.weekDate, value: at.weekDate },
            { label: at.names.julian, value: at.dayNumber }
        ]
    };
}

// What the rain does in the next hours, in the sentence: from when it's
// likely, now if it already is, or that none is due.
function rainWords(at) {
    const said = at.said;
    if (at.rainFrom === "") {
        return filled(said.dry, { hours: at.hours });
    }
    return at.rainFrom === "now" ? said.rainNow : filled(said.rainFrom, { clock: at.rainFrom });
}

// Weather's drawer, from the forecast as Weather.qml says it: `sky`,
// `warmth` and `feels`, what it's like now (`feels` null where the
// forecast gives none); `place`, where it's for and how that was found;
// `hours`, how many are ahead, and `low` and `high`, their warmth;
// `rainFrom`, the first hour likely to rain, "now", or "" with none
// due, and `rain`, the Rain row's own side; `days`, each `{ name, low,
// high, sky }` from today; and `said` and `names`. It says what Weather
// holds in brief: now, the place, the next hours and the rain when it's
// due, then each coming day as its row does, never every figure, which
// Conditions and each day hold a step in (Joel, 2026-11-14: "weather has
// no drawer thing giving any details while all of its neighbours do").
function weather(at) {
    const said = at.said;
    const now = at.feels === null ? filled(said.leadPlain, { sky: at.sky, warmth: at.warmth })
        : filled(said.lead, { sky: at.sky, warmth: at.warmth, feels: at.feels });
    return {
        title: at.names.title,
        body: `${sentence(now)}; ${rainWords(at)}.`,
        prose: true,
        facts: valued([
            { label: at.names.place, value: at.place },
            { label: filled(at.names.next, { hours: at.hours }),
                value: at.low === null || at.high === null ? ""
                    : filled(said.range, { low: at.low, high: at.high }) },
            { label: at.names.rain, value: at.rainFrom === "" ? "" : at.rain }
        ].concat(at.days.map(day => ({
            label: day.name,
            value: `${filled(said.range, { low: day.low, high: day.high })} \u00b7 ${day.sky}`
        })))),
        nerd: []
    };
}
