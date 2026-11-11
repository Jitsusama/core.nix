// The cases of clock for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Moment.js" as Moment
.import "../Keeping.js" as Keeping
.import "../Warmth.js" as Warmth
.import "../Here.js" as Here

function run(failures) {
    let asked = 0;
    const quoted = Common.quoted;
    const at = Common.at;
    // The clock's and the calendar's figures: the equinoxes, the
    // solstices and the moons against the instants USNO publishes, to
    // the minute, and the rest worked by hand.
    const minute = ms => new Date(ms).toISOString().slice(0, 16);
    // The words Keeping.js is given, as the vocabulary's Clock and
    // Servers say them (launcher/words.json).
    const KeepingCases = {
        clock: { onTime: "on time", fast: "{offset} fast", slow: "{offset} slow",
            gains: "gains {seconds} a day", loses: "loses {seconds} a day" },
        servers: { reach: "{count} of the last 8", unreachable: "out of reach",
            unusable: "set aside", aside: "set aside: {why}",
            "standing:chosen": "chosen", "standing:tooVariable": "set aside: too variable",
            "standing:disagrees": "set aside: disagrees",
            "why:tooVariable": "too variable", "why:disagrees": "disagrees with the others",
            "why:untrusted": "disagrees with the trusted servers",
            "why:tooFewSamples": "too few samples yet",
            "why:chosen": "the clock is kept by it",
            "why:combined": "combined with the chosen one",
            silent: "not answering", unsampled: "no sample yet",
            "kind:reference": "reference clock", interleaved: "interleaved",
            basic: "basic", authenticated: "authenticated",
            unauthenticated: "unauthenticated", exchange: "{mode} \u00b7 {authentication}",
            allGood: "{sent} sent, all good",
            packets: "{sent} sent \u00b7 {received} received \u00b7 {valid} valid \u00b7 "
                + "{good} good",
            offline: "{count} offline", burst: "{count} in a burst",
            unresolved: "{count} unresolved" }
    };
    // A night in Toronto's November as the daemon says it, the clock
    // given as hours and minutes in local time and a kelvin bare, so
    // the cases read the same wherever they run.
    const local = (date, time) =>
        new Date(date[0], date[1] - 1, date[2], time[0], time[1]).getTime();
    const WarmthCases = {
        night: { kelvin: 4000, followsTheRoom: true, seesColour: true, roomWhite: 3200,
            nightLight: true, nightWhite: 4000, sunset: local([2026, 11, 8], [16, 59]),
            sunrise: local([2026, 11, 9], [7, 4]) },
        evening: local([2026, 11, 8], [21, 0]),
        noon: local([2026, 11, 8], [12, 0]),
        said: { until: "night light until {time}", from: "night light from {time}",
            waits: "night light waits for where you are", following: "following the room",
            own: "the screen's own white", tomorrow: "tomorrow", yesterday: "yesterday" },
        told: {
            clock: instant => {
                const moment = new Date(instant);
                const two = value => String(value).padStart(2, "0");
                return `${two(moment.getHours())}:${two(moment.getMinutes())}`;
            },
            kelvin: value => `${value} K`
        }
    };
    const evening = at("2026-11-04T22:00");
    const day = new Date(2026, 10, 4);
    const moon = Moment.moon(evening);
    const season = Moment.season(evening, 43.67);
    // Location's words, as words.json says them, and its numbers in an
    // English locale's marks.
    const hereSaid = { metres: "within {n} m", kilometres: "within {n} km", north: "N",
        south: "S", east: "E", west: "W", nearEdge: "near a time zone's edge",
        foundWithin: "found {within}", via: "found \u00b7 location",
        viaHand: "set by hand \u00b7 location", "via-wifi": "via Wi-Fi \u00b7 location",
        "via-gps": "via GPS \u00b7 location", altitude: "{n} m", speed: "{n} km/h",
        heading: "{n}\u00b0", "no-altitude": "altitude", "no-speed": "speed",
        "no-heading": "heading", or: "or" };
    const hereTells = { leftOut: "Its source gives no {missing}.",
        "leftOut-wifi": "Wi-Fi gives no {missing}.", "leftOut-gps": "GPS gave no {missing}." };
    const leftOut = here => Here.leftOut(here, hereSaid, hereTells);
    // A fix GPS found, with as many of its three figures as given.
    const gps = (altitude, speed, heading) => ({ latitude: 12.3457, longitude: -98.7654,
        accuracy: 5, when: 1000, source: "gps", altitude, speed, heading });
    // A fix in another zone, by its width and whether its zone holds.
    const unfollowed = (accuracy, zoneHolds) => Here.unfollowed({ accuracy, zoneHolds },
        hereSaid, hereTold);
    const hereTold = { number: (value, decimals) => Qt.locale("en_CA").toString(value, "f",
        decimals) };
    // A plate's first two facts, when it was found and how near.
    const nearAndWhen = [{ id: "located", value: "12 min ago" },
        { id: "accuracy", value: "within 26 km" }];
    const figures = [
        ["UTC", Moment.utcClock(evening + 7000), "22:00:07"],
        ["Unix time", Moment.unix(evening), 1793829600],
        ["TAI", Moment.tai(evening), "22:00:37"],
        ["GPS time", quoted(Moment.gps(evening)), quoted({ week: 2443, seconds: 338418 })],
        // As Time says it, in the launcher's own marks (Y2YBTS's review of #2489).
        ["GPS time said", Moment.gpsSaid(evening, "week {week} \u00b7 {seconds} s",
            { group: "\u202f", point: "," }), "week 2443 \u00b7 338\u202f418 s"],
        ["a longitude said", Moment.longitudeSaid(-79.42, ","), "79,42\u00b0 W"],
        ["a longitude east said", Moment.longitudeSaid(10.5, "."), "10.5\u00b0 E"],
        ["Greenwich sidereal time", Moment.sidereal(evening, 0), "00:56:37"],
        ["a date this year, without its year", Moment.day(day, evening),
            "Wednesday 4 November"],
        ["an eyebrow this year, without its year", Moment.eyebrow(day, evening),
            "Wednesday 4 November"],
        // A seven-day forecast from 28 December reaches into next year.
        ["an eyebrow of another year, with its year",
            Moment.eyebrow(new Date(2027, 0, 1), evening), "Friday 1 January 2027"],
        ["a date of another year, with its year", Moment.day(new Date(2027, 0, 1), evening),
            "Friday 1 January 2027"],
        ["ISO date", Moment.iso(day), "2026-11-04"],
        ["ISO week date", Moment.isoWeekDate(day), "2026-W45-3"],
        ["ISO week date across a year", Moment.isoWeekDate(new Date(2027, 0, 1)),
            "2026-W53-5"],
        ["ordinal date", Moment.ordinal(day), 308],
        ["quarter", quoted(Moment.quarter(day)), quoted({ quarter: 4, day: 35, days: 92 })],
        ["Unix day", Moment.unixDay(day), 20761],
        ["Julian day number", Moment.dayNumber(day), 2461349],
        ["modified Julian day", Moment.modified(day), 61348],
        ["March equinox", minute(Moment.solstice(2026, 0)), "2026-03-20T14:45"],
        ["June solstice", minute(Moment.solstice(2026, 1)), "2026-06-21T08:24"],
        ["September equinox", minute(Moment.solstice(2026, 2)), "2026-09-23T00:05"],
        ["December solstice", minute(Moment.solstice(2026, 3)), "2026-12-21T20:50"],
        ["season in the north", season.season, "autumn"],
        ["season in the south", Moment.season(evening, -33.9).season, "spring"],
        ["season's end", minute(season.ends), "2026-12-21T20:50"],
        ["moon's phase", moon.phase, "waningCrescent"],
        ["moon lit", Math.round(moon.lit * 100), 19],
        ["next new moon", minute(moon.new), "2026-11-09T07:02"],
        ["next full moon", minute(moon.full), "2026-11-24T14:53"],
        ["full moon on its day", Moment.moon(at("2026-11-24T12:00")).phase, "full"],
        // How chrony's keeping is worded: seconds in the unit that reads
        // them, signed with a true minus; how far a clock is out and how
        // fast it runs; a span; a hardware clock's drift in a day; how
        // many polls were answered; and where a server stands. Every
        // measure keeps three significant digits, trailing zeros too, in
        // the settings' decimal mark (VKCF3Z, 2026-11-07).
        ["nanoseconds", Keeping.seconds(2.98e-8, false), "29.8 ns"],
        ["microseconds, signed", Keeping.seconds(-0.000388, true), "\u2212388 \u00b5s"],
        ["milliseconds", Keeping.seconds(0.036112, false), "36.1 ms"],
        ["milliseconds, in a comma's mark", Keeping.seconds(0.036112, false, ","),
            "36,1 ms"],
        ["a microsecond short of a millisecond", Keeping.seconds(0.0009996, false),
            "1.00 ms"],
        ["seconds", Keeping.seconds(1.5, true), "+1.50 s"],
        ["under chronyd's finest", Keeping.seconds(1e-10, true), "0 s"],
        ["a clock behind", Keeping.out(-0.000012, KeepingCases.clock), "12.0 \u00b5s slow"],
        ["a clock ahead", Keeping.out(0.31, KeepingCases.clock), "310 ms fast"],
        ["a clock on time", Keeping.out(0, KeepingCases.clock), "on time"],
        ["a clock running fast", Keeping.runs(6.267, KeepingCases.clock), "6.27 ppm fast"],
        ["a clock running slow", Keeping.runs(-3.226, KeepingCases.clock), "3.23 ppm slow"],
        ["a skew of ten", Keeping.ppm(9.861, false), "9.86 ppm"],
        ["a span in minutes", Keeping.span(1024), "17 min 4 s"],
        ["a span in hours", Keeping.span(86400), "24 h"],
        ["a span under a second", Keeping.span(0.5), "0.5 s"],
        ["a drift in a day", Keeping.daily(2.1, KeepingCases.clock), "gains 181 ms a day"],
        ["a loss in a day", Keeping.daily(-0.5, KeepingCases.clock), "loses 43.2 ms a day"],
        ["polls answered", Keeping.reach(0o375, KeepingCases.servers), "7 of the last 8"],
        ["the next poll", Keeping.nextPoll({ poll: 64, lastSample: 12 }), 52],
        ["a chosen server", Keeping.standing({ standing: "chosen", reach: 255 },
            KeepingCases.servers), "chosen"],
        ["a server out of reach", Keeping.standing({ standing: "unusable", reach: 0 },
            KeepingCases.servers), "out of reach"],
        ["a server set aside, and why", Keeping.standing({ standing: "unusable", reach: 3,
            chosen: { why: "tooFewSamples" } }, KeepingCases.servers),
            "set aside: too few samples yet"],
        ["a server too variable", Keeping.standing({ standing: "tooVariable", reach: 255 },
            KeepingCases.servers), "set aside: too variable"],
        // Why is said only where the note doesn't say it (VKCF3Z, 2026-11-08).
        ["a chosen server's why, which its note says", Keeping.whyUnsaid({ standing: "chosen",
            reach: 255, chosen: { why: "chosen" } }, KeepingCases.servers), ""],
        ["a server set aside's why, which its note says", Keeping.whyUnsaid({
            standing: "unusable", reach: 3, chosen: { why: "tooFewSamples" } },
            KeepingCases.servers), ""],
        ["an agreeing server's why, which its note doesn't say", Keeping.whyUnsaid({
            standing: "combined", reach: 255, chosen: { why: "combined" } },
            KeepingCases.servers), "combined with the chosen one"],
        ["a too variable server's why, which its note says", Keeping.whyUnsaid({
            standing: "tooVariable", reach: 255, chosen: { why: "tooVariable" } },
            KeepingCases.servers), ""],
        ["a disagreeing server's why, which its note says", Keeping.whyUnsaid({
            standing: "disagrees", reach: 255, chosen: { why: "disagrees" } },
            KeepingCases.servers), ""],
        ["an untrusted server's why, which says more than its note", Keeping.whyUnsaid({
            standing: "disagrees", reach: 255, chosen: { why: "untrusted" } },
            KeepingCases.servers), "disagrees with the trusted servers"],
        ["a server set aside and out of reach keeps its why", Keeping.whyUnsaid({
            standing: "unusable", reach: 0, chosen: { why: "tooFewSamples" } },
            KeepingCases.servers), "too few samples yet"],
        // The fact the row's side is made from tells only its note, and
        // goes with none; one that only reads the same stays.
        ["a server's facts beside its side", Keeping.besideSide([
            { id: "offset", side: "+12.0 \u00b5s", note: "measured +11.0 \u00b5s" },
            { id: "estimate", side: "+12.0 \u00b5s", note: "" },
            { id: "stratum", side: "2", note: "" }], "offset")
            .map(each => `${each.id} ${each.side}`).join(", "),
            "offset measured +11.0 \u00b5s, estimate +12.0 \u00b5s, stratum 2"],
        ["a server's side made from a fact with no note", Keeping.besideSide([
            { id: "offset", side: "+12.0 \u00b5s", note: "" },
            { id: "stratum", side: "2", note: "" }], "offset")
            .map(each => each.id).join(", "), "stratum"],
        // A server's side is never empty, and its note gives way where
        // the side says it's out of reach.
        ["a server's side, sampled", Keeping.side({ lastSample: 12, offset: 0.000012,
            reach: 255 }, KeepingCases.servers), "+12.0 \u00b5s"],
        ["a server's side, silent", Keeping.side({ lastSample: null, reach: 0 },
            KeepingCases.servers), "not answering"],
        ["a server's side, answering with no sample yet", Keeping.side({ lastSample: null,
            reach: 1 }, KeepingCases.servers), "no sample yet"],
        ["a silent server's note", Keeping.note({ standing: "unusable", lastSample: null,
            reach: 0 }, KeepingCases.servers), ""],
        ["a server once heard, now out of reach", Keeping.note({ standing: "unusable",
            lastSample: 900, reach: 0 }, KeepingCases.servers), "out of reach"],
        ["a server answering with no sample yet still says where it stands", Keeping.note({
            standing: "combined", lastSample: null, reach: 1 },
            { "standing:combined": "agrees" }), "agrees"],
        // The screen's Warmth, from what the backlight daemon says: what
        // holds it, by night, by day, waiting for a place, or the room;
        // and its figures, the room's white where a sensor sees it, then
        // the night's, a day said only when it isn't today (VKCF3Z,
        // 2026-11-08).
        ["nothing warming the screen", Warmth.of([]), null],
        ["a daemon before the night light", quoted(Warmth.of([{ kind: "warmth",
            kelvin: 6500, follows_the_room: true }])), quoted({ kelvin: 6500,
            followsTheRoom: true,
            seesColour: false, roomWhite: null, nightLight: false, nightWhite: null,
            sunset: null, sunrise: null })],
        ["the night light, as the machine daemon serves it", quoted(Warmth.of([
            { kind: "backlight", device: "panel" }, { kind: "warmth", kelvin: 3400,
                follows_the_room: false, sees_colour: true, room_white_kelvin: 5100,
                night_light: true, night_white_kelvin: 3400, sunset_ms: 1000,
                sunrise_ms: 2000 }])), quoted({ kelvin: 3400, followsTheRoom: false,
            seesColour: true, roomWhite: 5100, nightLight: true, nightWhite: 3400,
            sunset: 1000, sunrise: 2000 })],
        ["by night", Warmth.note(WarmthCases.night, WarmthCases.evening, WarmthCases.said,
            WarmthCases.told), "night light until 07:04"],
        ["by day", Warmth.note(WarmthCases.night, WarmthCases.noon, WarmthCases.said,
            WarmthCases.told), "night light from 16:59"],
        ["waiting for a place", Warmth.note(Object.assign({}, WarmthCases.night,
            { sunset: null, sunrise: null }), WarmthCases.noon, WarmthCases.said,
            WarmthCases.told), "night light waits for where you are"],
        ["following the room", Warmth.note(Object.assign({}, WarmthCases.night,
            { nightLight: false }), WarmthCases.evening, WarmthCases.said, WarmthCases.told),
            "following the room"],
        ["the screen's own white", Warmth.note(Object.assign({}, WarmthCases.night,
            { nightLight: false, seesColour: false }), WarmthCases.evening,
            WarmthCases.said, WarmthCases.told), "the screen's own white"],
        ["its figures by night", quoted(Warmth.figures(WarmthCases.night,
            WarmthCases.evening, WarmthCases.said, WarmthCases.told)), quoted([
            { id: "room", side: "3200 K" },
            { id: "sunrise", side: "07:04 \u00b7 tomorrow" },
            { id: "sunset", side: "16:59" }, { id: "night", side: "4000 K" }])],
        ["by day, the sunset comes first, and no room's white without a sensor",
            Warmth.figures(Object.assign({}, WarmthCases.night, { seesColour: false }),
                WarmthCases.noon, WarmthCases.said, WarmthCases.told)
                .map(figure => figure.id).join(" "), "sunset sunrise night"],
        // What a server is asked as, how its exchange went, its packets
        // saying only what differs, its score, its delay's asymmetry,
        // and what the servers' activity says beyond their count, a
        // part with nothing in it giving way (VKCF3Z, 2026-11-07).
        ["a reference clock", Keeping.mode({ kind: "reference" }, KeepingCases.servers),
            "reference clock"],
        ["an interleaved exchange, authenticated", Keeping.exchanged({ interleaved: true,
            authenticated: true }, KeepingCases.servers),
            "interleaved \u00b7 authenticated"],
        ["a basic exchange", Keeping.exchanged({ interleaved: false,
            authenticated: false }, KeepingCases.servers), "basic \u00b7 unauthenticated"],
        ["every packet good", Keeping.packets({ sent: 13, received: 13, valid: 13, good: 13 },
            KeepingCases.servers), "13 sent, all good"],
        ["packets lost", Keeping.packets({ sent: 13, received: 12, valid: 12, good: 11 },
            KeepingCases.servers),
            "13 sent \u00b7 12 received \u00b7 12 valid \u00b7 11 good"],
        ["a score", Keeping.score(1), "1.00"],
        ["no score, as chronyd's NaN", Keeping.score(null), ""],
        ["no asymmetry", Keeping.asymmetry(0), "0"],
        ["an asymmetry", Keeping.asymmetry(-0.125), "\u22120.125"],
        ["activity with nothing beyond the count", Keeping.activity({ online: 2,
            offline: 0, burstOnline: 0, burstOffline: 0, unresolved: 0 },
            KeepingCases.servers).join(", "), ""],
        ["activity beyond the count", Keeping.activity({ online: 1, offline: 1,
            burstOnline: 1, burstOffline: 1, unresolved: 1 }, KeepingCases.servers)
            .join(", "), "1 offline, 2 in a burst, 1 unresolved"],
        // Where the laptop is claims no more places than it knows, the
        // nearest order of magnitude to its accuracy: four under 35 m,
        // three under 350 m, two under 3.5 km, one under 35 km, else
        // none; how near is said to three figures, rounded up; and when
        // it was found is how long ago within a day, else its day and
        // time (VKCF3Z, 2026-11-08).
        ["a position to 5 m", Here.coordinates({ latitude: 12.345_678,
            longitude: -98.765_4, accuracy: 5 }, hereSaid),
            "12.3457\u00b0 N \u00b7 98.7654\u00b0 W"],
        ["a position to 80 m", Here.coordinates({ latitude: 12.345_678,
            longitude: -98.765_4, accuracy: 80 }, hereSaid),
            "12.346\u00b0 N \u00b7 98.765\u00b0 W"],
        ["a position to 100 m", Here.coordinates({ latitude: 12.345_678,
            longitude: -98.765_4, accuracy: 100 }, hereSaid),
            "12.346\u00b0 N \u00b7 98.765\u00b0 W"],
        ["a position to 5 km, south and east", Here.coordinates({ latitude: -12.345_6,
            longitude: 123.456_7, accuracy: 5000 }, hereSaid),
            "12.3\u00b0 S \u00b7 123.5\u00b0 E"],
        ["a position to 500 km, in whole degrees", Here.coordinates({ latitude: 12.35,
            longitude: -98.76, accuracy: 500_000 }, hereSaid), "12\u00b0 N \u00b7 99\u00b0 W"],
        ["no hemisphere at none", Here.coordinates({ latitude: -0.04, longitude: 0.04,
            accuracy: 26_000 }, hereSaid), "0.0\u00b0 \u00b7 0.0\u00b0"],
        ["near, in metres", Here.within(80, hereSaid, hereTold), "within 80 m"],
        ["near, a metre short of a kilometre", Here.within(999.5, hereSaid, hereTold),
            "within 1 km"],
        ["near, a metre past one", Here.within(1001, hereSaid, hereTold), "within 1.01 km"],
        ["near, in kilometres", Here.within(26_000, hereSaid, hereTold), "within 26 km"],
        ["near, in metres to three figures", Here.within(100.2, hereSaid, hereTold),
            "within 101 m"],
        ["near, a thousand kilometres on, to three figures",
            Here.within(1_234_000, hereSaid, hereTold), "within 1,240 km"],
        ["located within the day", Here.located(1000, 1000 * 1000 + 720_000,
            { ago: (since, now) => `${(now - since) / 60_000} min ago` }), "12 min ago"],
        ["located days ago", Here.located(1000, 1000 * 1000 + 3 * 86_400_000,
            { day: () => "Saturday", clock: () => "06:46" }), "Saturday 06:46"],
        ["how near goes from the plate while the note says it",
            JSON.stringify(Here.besideNote(nearAndWhen, true).map(fact => fact.id)),
            JSON.stringify(["located"])],
        ["and stays while the note says the zone instead",
            JSON.stringify(Here.besideNote(nearAndWhen, false).map(fact => fact.id)),
            JSON.stringify(["located", "accuracy"])],
        ["a circle a zone's line crosses isn't followed", unfollowed(3000, false),
            "near a time zone's edge"],
        ["one wider than a region says its width", unfollowed(80_000, true),
            "found within 80 km"],
        ["one that's followed, or held put back, says nothing more",
            unfollowed(26_000, true), ""],
        ["one unsaid round its circle is followed only within 10 km",
            `${unfollowed(9000, null)}|${unfollowed(26_000, null)}`, "|found within 26 km"],
        ["one of no width says nothing", unfollowed(null, true), ""],
        // Its plate's eyebrow says what found it, its facts the
        // figures only some sources give, and its body what its
        // source couldn't (VKCF3Z, 2026-11-09).
        ["found by Wi-Fi", Here.eyebrow({ source: "wifi" }, hereSaid),
            "via Wi-Fi \u00b7 location"],
        ["found by a source unknown", Here.eyebrow({ source: "galileo" }, hereSaid),
            "found \u00b7 location"],
        ["found by none named", Here.eyebrow({ source: null }, hereSaid),
            "found \u00b7 location"],
        ["set by hand, though a source is named",
            Here.eyebrow({ source: "wifi", hand: true }, hereSaid),
            "set by hand \u00b7 location"],
        ["GPS's figures follow where on a map, speed in km/h",
            Here.facts(gps(212.4, 4.26, 274.6), 1000, hereSaid,
                Object.assign({ ago: () => "now" }, hereTold))
                .map(fact => `${fact.id} ${fact.value}`).slice(2).join("|"),
            "coordinates 12.3457\u00b0 N \u00b7 98.7654\u00b0 W|altitude 212 m|speed 15.3 km/h|"
                + "heading 275\u00b0"],
        ["a speed of none is held, and said", Here.facts(gps(null, 0, null), 1000,
            hereSaid, Object.assign({ ago: () => "now" }, hereTold))
            .filter(fact => ["altitude", "speed", "heading"].includes(fact.id))
            .map(fact => fact.value).join("|"), "0 km/h"],
        ["a walk's speed keeps three figures, a highway's none past them, a "
            + "round one its zeros", [1.2, 27.8, 10].map(speed => Here.facts(gps(null, speed, null), 1000, hereSaid,
                Object.assign({ ago: () => "now" }, hereTold))
                .find(fact => fact.id === "speed").value).join("|"),
            "4.32 km/h|100 km/h|36.0 km/h"],
        ["Wi-Fi gives none of the three", leftOut({ source: "wifi" }),
            "Wi-Fi gives no altitude, speed or heading."],
        ["GPS gave two of them not", leftOut(gps(212, null, null)),
            "GPS gave no speed or heading."],
        ["GPS gave one of them not", leftOut(gps(212, 4, null)),
            "GPS gave no heading."],
        ["all three given leaves nothing to say", leftOut(gps(212, 4, 275)),
            ""],
        ["a source unnamed is said as its source", leftOut({ source: null }),
            "Its source gives no altitude, speed or heading."],
        ["one set by hand was found by no source",
            leftOut({ source: null, hand: true }), ""]
    ];
    for (const [what, got, expected] of figures) {
        asked++;
        if (got !== expected) {
            failures.push(`${what} was ${quoted(got)}, not ${quoted(expected)}`);
        }
    }
    return asked;
}
