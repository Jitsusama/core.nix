// The drawers of Now's Time and Date, worked out apart from their places
// (Clocks.qml, Today.qml) so tests/answers/now-drawers.js holds them in
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
// "" while it's asked); `clockSide`, the Clock row's own side; and
// `names`, each fact's label and the title.
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
        ])
    };
}

// Date's drawer, from the calendar: `iso`; `week`; `day` of the year's
// `days`; `season` and `seasonLeft`, its days to go; `moon` and `lit`;
// `quarter`, `quarterDay` and `quarterDays`; and `said` and `names`.
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
        ])
    };
}
