// How the time and the date read, and every figure reckoned from them, in
// one place, so Now's rows, its plate and Time and Date's insides each say
// a thing the same way (Joel, 2026-10-28: "we need to not have different
// versions of these sort of elements"). The time's figures are Time's
// (UTC, Unix time, TAI, GPS time, sidereal time) and the date's are Date's
// (ISO dates, the quarter, Julian days, the season and the moon), and
// neither says the other's (VKCF3Z's ruling, 2026-10-30: "Time holds the
// clock ... Date holds the calendar only ... Nothing repeats"). Names of
// days and months are Qt's, so they follow the locale. The season and the
// moon are Jean Meeus's, from Astronomical Algorithms (second edition,
// chapters 27, 47 and 49), to within minutes, as near as a date needs.
// Plain JavaScript, so tests/answers.qml holds it in Qt's own engine.
.pragma library
.import "Numbers.js" as Numbers

// A second, a minute and a day, in milliseconds.
const SECOND = 1000;
const MINUTE = 60_000;
const DAY = 86_400_000;
const WEEK_SECONDS = 604_800;
// Degrees to radians.
const RADIANS = Math.PI / 180;

// TAI runs ahead of UTC by every leap second since 1972: 37 s since the
// last, at the end of 2016, and IERS has announced none since. GPS time
// kept TAI's count as it stood at its start, 19 s behind it, so it runs
// 18 s ahead of UTC.
const TAI_AHEAD = 37;
const GPS_AHEAD = 18;
// GPS time's start, 6 January 1980, in Unix seconds.
const GPS_EPOCH = 315_964_800;
// The Julian day of the Unix epoch, and of J2000.0, noon on 1 January 2000.
const UNIX_JULIAN = 2_440_587.5;
const J2000 = 2_451_545;
// A Julian century, in days.
const CENTURY = 36_525;
// Terrestrial Time, which Meeus reckons in, ran 69 s ahead of UTC in 2026;
// a minute either way moves no season or moon to another day.
const TT_AHEAD = 69;
// The mean synodic month, new moon to new moon, in days.
const SYNODIC = 29.530_588_861;

function two(number) {
    return String(number).padStart(2, "0");
}

// The time as a row says it, "14:00".
function clock(at) {
    return Qt.formatTime(at, "HH:mm");
}

// The date as Date says it, "Wednesday 4 November", with its year only
// when it's another than `now`'s, as a full date is said everywhere.
function day(at, now) {
    return Qt.formatDate(at, at.getFullYear() === new Date(now).getFullYear() ? "dddd d MMMM"
        : "dddd d MMMM yyyy");
}

// The date to paste, "Wednesday 4 November 2026", whole wherever it lands.
function fullDay(at) {
    return Qt.formatDate(at, "dddd d MMMM yyyy");
}

// The date over a plate or a drawer, "Wednesday 4 November", with its
// year only when it's another than `now`'s, as a seven-day forecast from
// 28 December reaches.
function eyebrow(at, now) {
    return day(at, now);
}

// A day as a row says one past the week around it: "28 Oct", with its year
// only when it's another than `now`'s.
function dayOf(ms, now) {
    const at = new Date(ms);
    return Qt.formatDate(at, at.getFullYear() === new Date(now).getFullYear() ? "d MMM"
        : "d MMM yyyy");
}

// A day with the time the clock reads then: "14 Mar 2027 03:00". The middle
// dot is kept for a time and how far off it is, never a day and its time.
function dayAndClock(ms, now) {
    return `${dayOf(ms, now)} ${clock(new Date(ms))}`;
}

// A moment's time of day in UTC, to the second, "19:04:07", from
// milliseconds.
function utcClock(ms) {
    const at = new Date(ms);
    return `${two(at.getUTCHours())}:${two(at.getUTCMinutes())}:${two(at.getUTCSeconds())}`;
}

// Whole seconds since the Unix epoch.
function unix(ms) {
    return Math.floor(ms / SECOND);
}

// TAI's time of day, to the second.
function tai(ms) {
    return utcClock(ms + TAI_AHEAD * SECOND);
}

// GPS time as receivers give it: { week, seconds } into that week.
function gps(ms) {
    const seconds = unix(ms) - GPS_EPOCH + GPS_AHEAD;
    return { week: Math.floor(seconds / WEEK_SECONDS), seconds: seconds % WEEK_SECONDS };
}

// GPS time as Time says it, in the vocabulary's words, `template`
// ("week {week} \u00b7 {seconds} s"), its seconds grouped and pointed in
// the launcher's own number marks (Y2YBTS's review of #2489).
function gpsSaid(ms, template, marks) {
    const reckoned = gps(ms);
    return template.replace("{week}", String(reckoned.week))
        .replace("{seconds}", Numbers.format(reckoned.seconds, 0, marks.group, marks.point));
}

// A longitude as sidereal time names it, "79.42\u00b0 W", in the
// launcher's decimal mark.
function longitudeSaid(longitude, point) {
    return `${Numbers.format(Math.abs(longitude), 2, "", point)}\u00b0 `
        + (longitude < 0 ? "W" : "E");
}

// The Julian day of a moment, with its fraction.
function julian(ms) {
    return ms / DAY + UNIX_JULIAN;
}

// Mean sidereal time, in hours, at Greenwich or `longitude` degrees east
// (Meeus 12.4), as "19:04:07".
function sidereal(ms, longitude) {
    const days = julian(ms) - J2000;
    const t = days / CENTURY;
    const degrees = 280.460_618_37 + 360.985_647_366_29 * days + 0.000_387_933 * t * t
        - t * t * t / 38_710_000 + (longitude ?? 0);
    const seconds = Math.floor(((degrees % 360) + 360) % 360 * 240);
    return `${two(Math.floor(seconds / 3600))}:${two(Math.floor(seconds / 60) % 60)}:`
        + two(seconds % 60);
}

// The Julian day number of a local date: the day beginning at noon of it,
// counted from 1 January 4713 BC in the Julian calendar.
function dayNumber(at) {
    return Math.round(Date.UTC(at.getFullYear(), at.getMonth(), at.getDate()) / DAY
        + UNIX_JULIAN + 0.5);
}

// The modified Julian day of a local date, counted from midnight, 17
// November 1858.
function modified(at) {
    return dayNumber(at) - 2_400_001;
}

// The days since 1 January 1970 to a local date.
function unixDay(at) {
    return dayNumber(at) - Math.round(UNIX_JULIAN + 0.5);
}

// Which day of its year a local date is, 1 for the first of January,
// counted in UTC so a change of summer time moves nothing.
function ordinal(at) {
    return (Date.UTC(at.getFullYear(), at.getMonth(), at.getDate())
        - Date.UTC(at.getFullYear(), 0, 1)) / DAY + 1;
}

function daysIn(year) {
    return new Date(year, 1, 29).getMonth() === 1 ? 366 : 365;
}

// The date as ISO 8601 writes it, "2026-11-04".
function iso(at) {
    return `${at.getFullYear()}-${two(at.getMonth() + 1)}-${two(at.getDate())}`;
}

// The ISO week date: { year, week, weekday }, Monday 1, the week holding
// its year's first Thursday the first, so its year can be the one beside.
function isoWeek(at) {
    const thursday = new Date(Date.UTC(at.getFullYear(), at.getMonth(), at.getDate()));
    const weekday = thursday.getUTCDay() || 7;
    thursday.setUTCDate(thursday.getUTCDate() + 4 - weekday);
    const first = Date.UTC(thursday.getUTCFullYear(), 0, 1);
    return {
        year: thursday.getUTCFullYear(),
        week: Math.ceil(((thursday.getTime() - first) / DAY + 1) / 7),
        weekday
    };
}

// "2026-W45-3".
function isoWeekDate(at) {
    const said = isoWeek(at);
    return `${said.year}-W${two(said.week)}-${said.weekday}`;
}

// The quarter of the year: { quarter, day, days }, its day counted from 1.
function quarter(at) {
    const first = Math.floor(at.getMonth() / 3) * 3;
    const from = Date.UTC(at.getFullYear(), first, 1);
    return {
        quarter: first / 3 + 1,
        day: (Date.UTC(at.getFullYear(), at.getMonth(), at.getDate()) - from) / DAY + 1,
        days: (Date.UTC(at.getFullYear(), first + 3, 1) - from) / DAY
    };
}

// Meeus's table 27.C: each term's amplitude, phase and rate, the periodic
// terms that move an equinox or a solstice off its mean instant.
const SOLAR_TERMS = [
    [485, 324.96, 1934.136], [203, 337.23, 32_964.467], [199, 342.08, 20.186],
    [182, 27.85, 445_267.112], [156, 73.14, 45_036.886], [136, 171.52, 22_518.443],
    [77, 222.54, 65_928.934], [74, 296.72, 3034.906], [70, 243.58, 9037.513],
    [58, 119.81, 33_718.147], [52, 297.17, 150.678], [50, 21.02, 2281.226],
    [45, 247.54, 29_929.562], [44, 325.15, 31_555.956], [29, 60.93, 4443.417],
    [18, 155.12, 67_555.328], [17, 288.79, 4562.452], [16, 198.04, 62_894.029],
    [14, 199.76, 31_436.921], [12, 95.39, 14_577.848], [12, 287.11, 31_931.756],
    [12, 320.81, 34_777.259], [9, 227.73, 1222.114], [8, 15.45, 16_859.074]
];

// Meeus's table 27.B, for the years 2000 to 3000: the mean instants of the
// March equinox, the June solstice, the September equinox and the December
// solstice, each a polynomial in thousands of years from 2000.
const SOLAR_MEANS = [
    [2_451_623.809_84, 365_242.374_04, 0.051_69, -0.004_11, -0.000_57],
    [2_451_716.567_67, 365_241.626_03, 0.003_25, 0.008_88, -0.000_3],
    [2_451_810.217_15, 365_242.017_67, -0.115_75, 0.003_37, 0.000_78],
    [2_451_900.059_52, 365_242.740_49, -0.062_23, -0.008_23, 0.000_32]
];

function polynomial(coefficients, x) {
    return coefficients.reduceRight((sum, each) => sum * x + each, 0);
}

// When an equinox or a solstice falls, in milliseconds: `which` 0 for
// March's, 1 June's, 2 September's and 3 December's.
function solstice(year, which) {
    const mean = polynomial(SOLAR_MEANS[which], (year - 2000) / 1000);
    const t = (mean - J2000) / CENTURY;
    const w = (35_999.373 * t - 2.47) * RADIANS;
    const stretch = 1 + 0.0334 * Math.cos(w) + 0.0007 * Math.cos(2 * w);
    const terms = SOLAR_TERMS.reduce((sum, [a, b, c]) => sum + a * Math.cos((b + c * t)
        * RADIANS), 0);
    const ephemeris = mean + 0.000_01 * terms / stretch;
    return (ephemeris - UNIX_JULIAN) * DAY - TT_AHEAD * SECOND;
}

// The seasons from March's equinox, in the north; the south has them the
// other way round.
const SEASONS = ["spring", "summer", "autumn", "winter"];

// The astronomical season at a moment, where `latitude` is, the north's
// when it's unknown: { season, began, ends } in milliseconds, the season
// one of SEASONS.
function season(ms, latitude) {
    const year = new Date(ms).getUTCFullYear();
    const instants = [solstice(year - 1, 3)];
    for (let which = 0; which < 4; which++) {
        instants.push(solstice(year, which));
    }
    instants.push(solstice(year + 1, 0));
    let at = instants.length - 2;
    while (at > 0 && instants[at] > ms) {
        at -= 1;
    }
    // instants[0] is last December's solstice, winter's start in the north.
    const north = (at + 3) % 4;
    const south = latitude !== undefined && latitude !== null && latitude < 0;
    return {
        season: SEASONS[south ? (north + 2) % 4 : north],
        began: instants[at],
        ends: instants[at + 1]
    };
}

// Meeus's table 49: the periodic terms that move a new moon (`sun`, `moon`
// and `node` multiply the Sun's and the Moon's mean anomalies, the Moon's
// argument of latitude and its node's longitude; `e` how many times the
// Earth's eccentricity weighs in), and the full moon's amplitudes beside
// the new's.
const PHASE_TERMS = [
    // [new, full, e, sun, moon, latitude, node]
    [-0.4072, -0.40614, 0, 0, 1, 0, 0], [0.17241, 0.17302, 1, 1, 0, 0, 0],
    [0.01608, 0.01614, 0, 0, 2, 0, 0], [0.01039, 0.01043, 0, 0, 0, 2, 0],
    [0.00739, 0.00734, 1, -1, 1, 0, 0], [-0.00514, -0.00515, 1, 1, 1, 0, 0],
    [0.00208, 0.00209, 2, 2, 0, 0, 0], [-0.00111, -0.00111, 0, 0, 1, -2, 0],
    [-0.00057, -0.00057, 0, 0, 1, 2, 0], [0.00056, 0.00056, 1, 1, 2, 0, 0],
    [-0.00042, -0.00042, 0, 0, 3, 0, 0], [0.00042, 0.00042, 1, 1, 0, 2, 0],
    [0.00038, 0.00038, 1, 1, 0, -2, 0], [-0.00024, -0.00024, 1, -1, 2, 0, 0],
    [-0.00017, -0.00017, 0, 0, 0, 0, 1], [-0.00007, -0.00007, 0, 2, 1, 0, 0],
    [0.00004, 0.00004, 0, 0, 2, -2, 0], [0.00004, 0.00004, 0, 3, 0, 0, 0],
    [0.00003, 0.00003, 0, 1, 1, -2, 0], [0.00003, 0.00003, 0, 0, 2, 2, 0],
    [-0.00003, -0.00003, 0, 1, 1, 2, 0], [0.00003, 0.00003, 0, -1, 1, 2, 0],
    [-0.00002, -0.00002, 0, -1, 1, -2, 0], [-0.00002, -0.00002, 0, 1, 3, 0, 0],
    [0.00002, 0.00002, 0, 0, 4, 0, 0]
];

// The planets' small pulls on every phase: each term's amplitude and its
// argument's start and rate per lunation, the first with a square of its
// own.
const PLANET_TERMS = [
    [0.000_325, 299.77, 0.107_408], [0.000_165, 251.88, 0.016_321],
    [0.000_164, 251.83, 26.651_886], [0.000_126, 349.42, 36.412_478],
    [0.000_11, 84.66, 18.206_239], [0.000_062, 141.74, 53.303_771],
    [0.000_06, 207.14, 2.453_732], [0.000_056, 154.84, 7.306_86],
    [0.000_047, 34.52, 27.261_239], [0.000_042, 207.19, 0.121_824],
    [0.000_04, 291.34, 1.844_379], [0.000_037, 161.72, 24.198_154],
    [0.000_035, 239.56, 25.513_099], [0.000_023, 331.55, 3.592_518]
];

// When lunation `k` is new, or full when `full`, in milliseconds: `k`
// counts new moons from that of 6 January 2000.
function phase(k, full) {
    const lunation = full ? k + 0.5 : k;
    const t = lunation / 1236.85;
    const mean = 2_451_550.097_66 + SYNODIC * lunation + 0.000_154_37 * t * t
        - 0.000_000_15 * t * t * t + 0.000_000_000_73 * t ** 4;
    const e = 1 - 0.002_516 * t - 0.000_007_4 * t * t;
    const sun = (2.5534 + 29.105_356_7 * lunation - 0.000_001_4 * t * t) * RADIANS;
    // The Moon's mean anomaly.
    const anomaly = (201.5643 + 385.816_935_28 * lunation + 0.010_758_2 * t * t
        + 0.000_012_38 * t * t * t) * RADIANS;
    const latitude = (160.7108 + 390.670_502_84 * lunation - 0.001_611_8 * t * t) * RADIANS;
    const node = (124.7746 - 1.563_755_88 * lunation + 0.002_067_2 * t * t) * RADIANS;
    const lunar = PHASE_TERMS.reduce((sum, [onNew, onFull, weight, s, m, f, n]) => sum
        + (full ? onFull : onNew) * e ** weight * Math.sin(s * sun + m * anomaly + f * latitude
            + n * node), 0);
    const planets = PLANET_TERMS.reduce((sum, [amplitude, start, rate], at) => sum
        + amplitude * Math.sin((start + rate * lunation - (at === 0 ? 0.009_173 * t * t : 0))
            * RADIANS), 0);
    return (mean + lunar + planets - UNIX_JULIAN) * DAY - TT_AHEAD * SECOND;
}

// The phases by how far through its month the moon is, each to the next's
// start: a principal phase is named for a day either side of its instant.
const PHASES = [
    [1 / SYNODIC, "new"], [0.25 - 1 / SYNODIC, "waxingCrescent"],
    [0.25 + 1 / SYNODIC, "firstQuarter"], [0.5 - 1 / SYNODIC, "waxingGibbous"],
    [0.5 + 1 / SYNODIC, "full"], [0.75 - 1 / SYNODIC, "waningGibbous"],
    [0.75 + 1 / SYNODIC, "lastQuarter"], [1 - 1 / SYNODIC, "waningCrescent"], [1, "new"]
];

// How much of the moon is lit, from 0 to 1, from where the Sun and the
// Moon are (Meeus 48.4, from chapter 47's mean elements).
function lit(ms) {
    const t = (julian(ms + TT_AHEAD * SECOND) - J2000) / CENTURY;
    const elongation = (297.850_192_1 + 445_267.111_403_4 * t - 0.001_881_9 * t * t) * RADIANS;
    const sun = (357.529_109_2 + 35_999.050_290_9 * t - 0.000_153_6 * t * t) * RADIANS;
    const anomaly = (134.963_396_4 + 477_198.867_505_5 * t + 0.008_741_4 * t * t) * RADIANS;
    const angle = Math.PI - elongation - (6.289 * Math.sin(anomaly) - 2.1 * Math.sin(sun)
        + 1.274 * Math.sin(2 * elongation - anomaly) + 0.658 * Math.sin(2 * elongation)
        + 0.214 * Math.sin(2 * anomaly) + 0.11 * Math.sin(elongation)) * RADIANS;
    return (1 + Math.cos(angle)) / 2;
}

// The moon at a moment: { phase, lit, age, new, full }, the phase one of
// PHASES' names, how much is lit from 0 to 1, its age in days since it
// was new, and when it's next new and next full, in milliseconds.
function moon(ms) {
    let k = Math.floor((ms / DAY + UNIX_JULIAN - 2_451_550.097_66) / SYNODIC);
    while (phase(k + 1, false) <= ms) {
        k += 1;
    }
    while (phase(k, false) > ms) {
        k -= 1;
    }
    const was = phase(k, false);
    const age = (ms - was) / DAY;
    const through = (ms - was) / (phase(k + 1, false) - was);
    const full = phase(k, true) > ms ? phase(k, true) : phase(k + 1, true);
    return {
        phase: PHASES.find(([upTo]) => through < upTo)[1],
        lit: lit(ms),
        age,
        new: phase(k + 1, false),
        full
    };
}
