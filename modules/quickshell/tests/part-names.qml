// Drives the launcher's answers that name their parts from ids they build
// as they answer (Calculator.qml, Clocks.qml, Today.qml, Waits.qml,
// Vendors.qml, Networks.qml and Characters.qml) through Quickshell itself,
// so every part each can show is named as the vocabulary names it
// (launcher/words.json, Words.qml): an id the vocabulary doesn't know shows
// as the id itself, as "calculator:hex" would, so a typo on either side
// fails here rather than on screen. The time is told with tzdata's zones
// kept beside it, a character with Unicode's names, a network with no
// lookups, and Waits hears what the machine daemon would tell of lag-watch. Currency names its
// parts by the currencies alone, so it builds no id to check. The time
// here and the date share no part, since the clock is Time's and the
// calendar Date's (VKCF3Z's ruling, 2026-10-30: "Nothing repeats"), and
// each says its figures as Now does. It
// exits 0 once every case holds, and otherwise 1, saying which case
// didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    Launcher.Calculator {
        id: calculator
    }

    Launcher.Clocks {
        id: clocks

        settings: ({ list: Quickshell.shellPath("zones.jsonl"), zone: "America/Toronto" })
    }

    Launcher.Today {
        id: today

        settings: ({ zone: "America/Toronto" })
    }

    Launcher.Waits {
        id: waits
    }

    Launcher.Vendors {
        id: vendors
    }

    Launcher.Networks {
        id: networks

        settings: ({ lookups: false })
    }

    Launcher.Characters {
        id: characters

        settings: ({ names: Quickshell.shellPath("names.txt") })
    }

    // Each answer, what's typed to show every part it can, and the ids of
    // those parts within its scope.
    readonly property var asked: [
        { scope: "calculator", from: calculator, typed: ["0x1f", "255 in hex"],
            ids: ["binary", "decimal", "hex", "octal"] },
        { scope: "time", from: clocks, typed: ["time in tokyo", "time"],
            ids: ["apart", "clock", "day", "gps", "servers", "sidereal", "tai", "unix",
                "utc", "zone"] },
        { scope: "today", from: today, typed: ["date"],
            ids: ["calendars", "iso", "julian", "mjd", "moon", "ordinal", "quarter", "season",
                "unix-day", "week"] },
        { scope: "waits", from: waits, typed: ["lag"], ids: ["typical", "work"] },
        { scope: "vendors", from: vendors, typed: ["3c:22:fb:12:34:56"],
            ids: ["colons", "dotted", "kind"] },
        { scope: "networks", from: networks,
            typed: ["10.0.0.5/22", "2001:db8::1/48", "/22", "is 10.0.0.1 in 10.0.0.0/8"],
            ids: ["broadcast", "first", "full", "hosts", "kind", "last", "mask", "network",
                "range", "size", "subnets", "wildcard"] },
        { scope: "characters", from: characters, typed: ["ascii 65", "utf8 \u00e9"],
            ids: ["binary", "bits", "decimal", "escaped", "hex", "length", "name", "percent",
                "point", "points", "utf8"] }
    ]

    // Quickshell hears Qt.exit only once the shell has loaded, so the cases
    // run just after.
    Component.onCompleted: Qt.callLater(root.cases)

    function cases() {
        let status = 0;
        try {
            run();
        } catch (error) {
            console.warn(`FAIL ${error.message ?? error}`);
            status = 1;
        }
        Qt.callLater(() => Qt.exit(status));
    }

    function run() {
        // A thing the vocabulary knows is made once, in each of its states,
        // since rows' bindings ask it about a thousand times a key; an id it
        // doesn't know is said as itself.
        const radio = Words.of("bluetooth:radio");
        expect("a known thing is made once, however often it's asked",
            Words.of("bluetooth:radio") === radio, "it was made again");
        const off = Words.of("bluetooth:radio", "off");
        expect("a thing in a state is made once, and is its own",
            Words.of("bluetooth:radio", "off") === off && off !== radio, "made again or shared");
        expect("a state it hasn't is the thing itself",
            Words.of("bluetooth:radio", "floating") === radio, "made apart");
        const unknown = Words.of("nothing:known");
        expect("an id it doesn't know is said as itself",
            unknown.name === "nothing:known" && unknown.parts.length === 0, unknown.name);
        for (const each of asked) {
            // What the vocabulary names each of its parts, or nothing for an
            // id it doesn't know.
            const named = each.ids.map(id => Words.things[`${each.scope}:${id}`]?.name ?? "");
            expect(`${each.scope}'s parts in the vocabulary`, named.every(name => name !== ""),
                each.ids.filter((id, at) => named[at] === "").join(", "));
            const shown = [];
            for (const typed of each.typed) {
                const said = each.from.answer(typed);
                expect(`"${typed}" answers`, said !== null && said.rows.length > 0,
                    JSON.stringify(said));
                for (const row of said.rows) {
                    if (!shown.includes(row.name))
                        shown.push(row.name);
                }
            }
            expect(`${each.scope}'s parts named as the vocabulary says`,
                shown.slice().sort().join(", ") === named.slice().sort().join(", "),
                `${shown.join(", ")}, not ${named.join(", ")}`);
        }
        const parts = said => said.rows.map(row => row.name);
        const clock = parts(clocks.answer("time"));
        const calendar = parts(today.answer("date"));
        expect("the time here holds no date", !clock.includes(Words.name("time:day")),
            clock.join(", "));
        expect("the time here and the date share no part",
            !clock.some(name => calendar.includes(name)),
            clock.filter(name => calendar.includes(name)).join(", "));
        asNowShows();
        asKept();
        asUnkept();
        const stoodIn = Calendars.reading;
        // Its day as it is before sunset, whenever the check runs.
        Calendars.reading = evening(stoodIn, Date.now() + 3600000);
        asReckoned();
        asTheDayEnds(stoodIn);
        Calendars.reading = stoodIn;
    }

    // A reading of the stood-in day whose sunset is at this moment, or
    // asked from nowhere, or with no sunset at all; the day before's sunset
    // a day before now unless it's said, or none from nowhere.
    function evening(reading, sunset, eve) {
        const asked = sunset === undefined ? null
            : { sunset: sunset, calendars: sunset === null ? [] : reading.evening.calendars };
        const before = sunset === undefined ? null
            : { sunset: eve ?? Date.now() - 86400000, calendars: reading.eve.calendars };
        return Object.assign({}, reading, { evening: asked, eve: before });
    }

    // The Hebrew and Islamic days begin at sunset where Date is seen from:
    // each plate says so just after its Day, "Day Begins \u00b7 17:42" in
    // the figures' face, and from sunset "Day Began" with the plate on the
    // day begun then; with nowhere to see it from, "at sunset", and on a day
    // with none, "no sunset today", each in the face words are read in
    // with the civil day kept. The other calendars' days begin at midnight.
    // Where the day before's sunset comes after midnight, far north in
    // summer, the day before's days are now's until it comes.
    function asTheDayEnds(stoodIn) {
        const said = Words.of("today:calendars").said ?? {};
        const dayEnds = (sunset, eve) => {
            Calendars.reading = evening(stoodIn, sunset, eve);
            const held = today.answer("date").rows
                .find(row => row.name === Words.name("today:calendars"));
            return held.verbs().map(each => {
                const plate = each.drawer();
                const at = plate.facts.findIndex(fact => fact.label
                    === Words.name("today:calendars:day"));
                const fact = plate.facts[at + 1];
                const begins = [Words.name("today:calendars:dayBegins"),
                    Words.name("today:calendars:dayBegan")].includes(fact?.label);
                const row = each.verbs()[at + 1];
                return {
                    side: `${each.name} ${each.side}`,
                    title: plate.title,
                    fact: begins ? `${fact.label} ${fact.value}` : "",
                    words: begins && fact.words === true,
                    row: begins ? `${row.name} ${row.side} ${row.copiesNothing === true}` : ""
                };
            });
        };
        const sides = days => days.map(day => day.side).join(", ");
        const facts = days => days.map(day => day.fact).join(", ");
        const clock = ms => Qt.formatTime(new Date(ms), "HH:mm");
        const later = Date.now() + 3600000;
        const before = dayEnds(later);
        expect("before sunset, each day that begins then says when",
            facts(before) === `, , ${Words.name("today:calendars:dayBegins")} ${clock(later)}, `
                + `${Words.name("today:calendars:dayBegins")} ${clock(later)}`, facts(before));
        expect("its row says it too, copying the time",
            before[2].row === `${Words.name("today:calendars:dayBegins")} ${clock(later)} false`,
            before[2].row);
        const earlier = Date.now() - 3600000;
        const after = dayEnds(earlier);
        expect("from sunset, the Hebrew and Islamic are on the day begun then",
            sides(after) === "Chinese 27 Ninth Month, Japanese Reiwa 8, Hebrew 26 Heshvan 5787, "
                + "Islamic 26 Jumada I 1448 AH", sides(after));
        expect("its plate is titled so",
            after[2].title === "\u05db\u05f4\u05d5 \u05d1\u05d7\u05e9\u05d5\u05d5\u05df "
                + "\u05d4\u05f3\u05ea\u05e9\u05e4\u05f4\u05d6", after[2].title);
        expect("and says the day began at sunset",
            after[3].fact === `${Words.name("today:calendars:dayBegan")} ${clock(earlier)}`,
            after[3].fact);
        const soon = Date.now() + 1800000;
        const unended = dayEnds(Date.now() + 86400000, soon);
        expect("before the day before's sunset after midnight, its days are still now's",
            sides(unended) === "Chinese 27 Ninth Month, Japanese Reiwa 8, Hebrew 24 Heshvan "
                + "5787, Islamic 24 Jumada I 1448 AH", sides(unended));
        expect("and they say they begin at that sunset",
            unended[2].fact === `${Words.name("today:calendars:dayBegins")} ${clock(soon)}`
                && unended[2].title === "\u05db\u05f4\u05d3 \u05d1\u05d7\u05e9\u05d5\u05d5\u05df "
                    + "\u05d4\u05f3\u05ea\u05e9\u05e4\u05f4\u05d6", JSON.stringify(unended[2]));
        const nowhere = dayEnds(undefined);
        // Said as words, it copies them all the same, since every row has a
        // Return act (Joel, 2026-10-26).
        expect("seen from nowhere, the day begins at sunset, the civil day kept",
            nowhere[2].side === "Hebrew 25 Heshvan 5787" && nowhere[3].fact
                === `${Words.name("today:calendars:dayBegins")} ${said.atSunset}`
                && nowhere[3].words && nowhere[3].row.endsWith(" false"),
            JSON.stringify(nowhere[3]));
        const none = dayEnds(null);
        expect("on a day with no sunset, it says so, the civil day kept",
            none[3].side === "Islamic 25 Jumada I 1448 AH" && none[2].fact
                === `${Words.name("today:calendars:dayBegins")} ${said.noSunset}`
                && none[2].words && none[2].row.endsWith(" false"),
            JSON.stringify(none[2]));
    }

    // The day in other calendars, from a reading as timekeeping gives it
    // (modules/quickshell/timekeeping/calendars-reading.json): Date's Calendars steps into the
    // Chinese, Japanese, Hebrew and Islamic, each a thing saying its date
    // in Latin letters, stepping into its figures, each named by the
    // vocabulary, and its plate in its own script: titled with its date
    // there, its figures there where it has them, a day it's reckoned from
    // as the launcher says a day, and a count in its words.
    function asReckoned() {
        const date = today.answer("date");
        // Date knows it's shown, and so asks for its calendars, by finding
        // itself in the answer or a step held in it.
        expect("the date says it's Date's, so it's known shown", date.live === today,
            String(date.live));
        const held = date.rows.find(row => row.name === Words.name("today:calendars"));
        expect("Date holds Calendars", held !== undefined && typeof held.verbs === "function",
            JSON.stringify(held));
        const calendars = held.verbs();
        expect("Calendars holds the Chinese, Japanese, Hebrew and Islamic, by their dates",
            calendars.map(each => `${each.name} ${each.side}`).join(", ")
                === "Chinese 27 Ninth Month, Japanese Reiwa 8, Hebrew 25 Heshvan 5787, "
                    + "Islamic 25 Jumada I 1448 AH",
            calendars.map(each => `${each.name} ${each.side}`).join(", "));
        const plates = calendars.map(each => each.drawer());
        expect("each plate has an eyebrow naming its calendar",
            plates.map(plate => plate.eyebrow ?? "").join(", ") === calendars.map(each => each.name)
                .join(", "), plates.map(plate => plate.eyebrow ?? "").join(", "));
        // A figure the calendar's script has no word for, as a count or a
        // day, is in Latin letters, so it's set in the theme's own face
        // rather than the script's, whose Latin is another's.
        const unmarked = [].concat(...plates.map(plate => plate.facts))
            .filter(fact => /^[\x20-\x7e]+$/.test(fact.value) !== (fact.latin === true));
        expect("a figure in Latin letters is marked so", unmarked.length === 0,
            JSON.stringify(unmarked));
        expect("each plate is in its own script",
            plates.map(plate => plate.script).join(", ") === "chinese, japanese, hebrew, arabic",
            plates.map(plate => plate.script).join(", "));
        expect("each plate is titled with its date there",
            plates[2].title === "\u05db\u05f4\u05d4 \u05d1\u05d7\u05e9\u05d5\u05d5\u05df "
                + "\u05d4\u05f3\u05ea\u05e9\u05e4\u05f4\u05d6", plates[2].title);
        // Which reckoning the Islamic months are by is the plate's to say:
        // a cursor row's note shows in the place's head, which isn't a row's.
        const reckoned = plates[3].facts[plates[3].facts.length - 1];
        expect("the Islamic plate ends with its reckoning",
            `${reckoned.label} ${reckoned.value}` === "Reckoning Umm al-Qura",
            JSON.stringify(reckoned));
        expect("no calendar says a note", calendars.every(each => each.note === ""),
            calendars.map(each => each.note).join(", "));
        const names = Object.keys(Words.things)
            .filter(id => Words.things[id].in === "today:calendars")
            .map(id => Words.things[id].name);
        const named = calendars.every(each => each.verbs().every(row => names.includes(row.name)));
        expect("each calendar's figures are named by the vocabulary", named,
            calendars.map(each => each.verbs().map(row => row.name).join(" ")).join(", "));
        const chinese = calendars[0];
        const figures = chinese.verbs().map(row => `${row.name} ${row.side}`);
        const term = Qt.formatDate(new Date(2026, 9, 23),
            new Date().getFullYear() === 2026 ? "d MMM" : "d MMM yyyy");
        expect("the Chinese figures are said in Latin letters, a count in its words",
            figures.join(", ") === "Year B\u01d0ngw\u01d4, Animal Horse, Month Ninth Month, "
                + `Day 27, Month Length 30 days, Solar Term Shu\u0101ngji\u00e0ng, Term Began ${
                    term}`, figures.join(", "));
        const facts = plates[0].facts.map(fact => `${fact.label} ${fact.value}`);
        expect("its plate says them in characters where they have them",
            facts.join(", ") === "Year \u4e19\u5348, Animal \u9a6c, Month \u4e5d\u6708, "
                + "Day \u5eff\u4e03\u65e5, Month Length \u5927\u6708, "
                + `Solar Term \u971c\u964d, Term Began ${term}`, facts.join(", "));
    }

    // How chrony keeps the clock, from a reading as timekeeping gives it
    // (tests/timekeeping-reading.json): Clock steps into every figure the
    // vocabulary names for it, Servers into each server, the chosen first,
    // and a server into every figure it has, each named as its drawer
    // names it; one that's answered none of its polls is out of reach.
    // Before chrony answers, and when nothing keeps the time, Clock is the
    // same thing, stepped into on Return, its side saying how it stands and
    // its inside why it holds nothing yet: a row that only said so did
    // nothing on Return, and changed kind under the cursor as the reading
    // came (Joel, 2026-10-26: every row has a Return act; VKCF3Z,
    // 2026-11-06).
    function asUnkept() {
        const reading = Timekeeping.reading;
        const kept = clocks.answer("time").rows.find(each => each.key === "time:clock");
        Timekeeping.reading = null;
        Timekeeping.watching = 1;
        const asked = clocks.answer("time").rows.find(each => each.key === "time:clock");
        Timekeeping.refused = true;
        const refused = clocks.answer("time").rows.find(each => each.key === "time:clock");
        Timekeeping.reading = reading;
        Timekeeping.watching = 0;
        Timekeeping.refused = false;
        for (const [when, clock, side, empty] of [
            ["asked", asked, "\u2026", "\u2026"],
            ["refused", refused, "nothing keeps the time", "Nothing keeps the time."]
        ]) {
            const said = `${clock?.verb} ${clock?.glyph === kept.glyph} ${clock?.side}`
                + ` ${clock?.verbs?.().length} ${clock?.empty}`;
            expect(`the clock ${when} steps in, saying why it holds nothing`,
                said === `more true ${side} 0 ${empty}`, said);
        }
    }

    function asKept() {
        const time = clocks.answer("time");
        const row = id => time.rows.find(each => each.name === Words.name(id));
        const ids = parent => Object.keys(Words.things)
            .filter(id => Words.things[id].in === parent).map(id => Words.things[id].name);
        const clock = row("time:clock");
        expect("the clock says it's synchronised", clock?.side === "synchronised",
            clock?.side);
        const inside = clock.verbs().map(each => each.name);
        expect("the clock holds each of its figures, in their order",
            inside.join(", ") === ids("time:clock").join(", "), inside.join(", "));
        // Its plate says where it's kept from, is named as itself, and
        // lists the same figures, never its side (VKCF3Z, 2026-11-07).
        const clockPlate = clock.thing === true ? clock.drawer() : null;
        expect("the clock's plate is via the chosen server, titled Clock",
            clockPlate?.eyebrow === "via time.harbour.test" && clockPlate?.title === "Clock",
            JSON.stringify(clockPlate));
        expect("the clock's plate says the same figures",
            clockPlate.facts.map(fact => fact.label).join(", ") === inside.join(", "),
            JSON.stringify(clockPlate.facts));
        const figure = id => clock.verbs().find(each => each.name === Words.name(id));
        const told = ["time:clock:rms-offset", "time:clock:stratum",
            "time:clock:update-interval"].map(id => figure(id)?.side).join(", ");
        expect("the clock says its RMS offset, stratum and update interval",
            told === "200 \u00b5s, 3, 1 min 4 s", told);
        const servers = row("time:servers").verbs();
        // A server never heard from says so on its side, so its note
        // doesn't say it again.
        const plates = servers.map(each => `${each.name} [${each.side}] ${each.note}`);
        expect("the servers are each a thing, the chosen first",
            plates[0].startsWith("time.harbour.test [") && plates[0].endsWith("] chosen")
                && plates[1] === "far.harbour.test [not answering] ",
            plates.join(", "));
        // The launcher knows a row by what it is, or else its key, keeping
        // the selection on it as the reading changes and taking two it knows
        // alike as one, so two servers known alike would pull it back to the
        // first.
        const known = servers.map(each => each.is ?? each.key);
        expect("each server is known as itself", new Set(known).size === servers.length,
            known.join(", "));
        const chosen = servers[0];
        const offset = chosen.verbs().find(each => each.name === Words.name("time:server:offset"));
        expect("a server's offset notes what was measured",
            offset?.note === "measured \u2212391 \u00b5s \u00b7 within 20.0 \u00b5s", offset?.note);
        const figures = chosen.verbs().map(each => each.name);
        const plate = chosen.drawer();
        // All but Why, which its note, "chosen", already says.
        const why = Words.name("time:server:why");
        expect("the chosen server holds every figure the vocabulary names for one but Why",
            figures.join(", ") === ids("time:server").filter(name => name !== why).join(", "),
            figures.join(", "));
        expect("its plate says the same figures",
            plate.facts.map(fact => fact.label).join(", ") === figures.join(", "),
            JSON.stringify(plate.facts));
        // Its row's side is its offset, so the plate's Offset says only
        // what was measured (Y2YBTS's review of #2489).
        const offsetFact = plate.facts.find(fact => fact.label === offset?.name);
        expect("its plate's offset doesn't say the side again",
            offsetFact?.value === offset?.note, JSON.stringify(offsetFact));
    }

    // The time here and the date as Now shows them: each wearing its own
    // place's glyph, as every row of Now does; the zone a figure copying
    // its name, never a step of copies alone (Joel, 2026-11-11), its
    // drawer's next change said as the launcher says a day with its time
    // ("14 Mar 2027 03:00"), then what it turns to after a middle dot; the
    // zone's plate with an eyebrow, as every plate has; and a day's number
    // copied as a figure to paste, with nothing grouping it.
    function asNowShows() {
        const time = clocks.answer("time");
        const date = today.answer("date");
        expect("Time and Date wear their places' glyphs",
            !!Theme.glyph.clock && time.glyph === Theme.glyph.clock
                && !!Theme.glyph.date && date.glyph === Theme.glyph.date,
            `${time.glyph}, ${date.glyph}`);
        const zone = time.rows.find(row => row.name === Words.name("time:zone"));
        expect("the zone copies its name on Return",
            typeof zone.verbs !== "function" && zone.verb === "copy"
                && zone.copies() === zone.side, `${zone.verb} ${zone.side}`);
        const plate = typeof zone.drawer === "function" ? zone.drawer() : zone.drawer;
        const next = plate.facts.find(fact => fact.label === Words.name("time:zone:next"));
        expect("the zone's next change is a day and its time, then what it turns to",
            /^\d{1,2} [A-Z][a-z]{2}( \d{4})? \d{2}:\d{2}( \u00b7 .+)?$/.test(next?.value),
            JSON.stringify(next));
        expect("the zone's plate has an eyebrow", (plate.eyebrow ?? "") !== "",
            JSON.stringify(plate));
        const numbers = ["today:unix-day", "today:julian", "today:mjd"].map(id => Words.name(id));
        const days = date.rows.filter(row => numbers.includes(row.name));
        expect("a day's number copies ungrouped",
            days.length === numbers.length && days.every(row => /^\d+$/.test(row.copy)),
            days.map(row => row.copy).join(", "));
    }
}
