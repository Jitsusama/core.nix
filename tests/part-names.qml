// Drives the launcher's answers that name their parts from ids they build
// as they answer (Calculator.qml, Clocks.qml, Today.qml, Waits.qml,
// Vendors.qml, Networks.qml and Characters.qml) through Quickshell itself,
// so every part each can show is named as the vocabulary names it
// (launcher/words.json, Words.qml): an id the vocabulary doesn't know shows
// as the id itself, as "calculator:hex" would, so a typo on either side
// fails here rather than on screen. The time is told with tzdata's zones
// kept beside it, a character with Unicode's names, a network with no
// lookups, and Waits reads what lag-watch would write. Currency names its
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
        asReckoned();
    }

    // The day in other calendars, from a reading as timekeeping gives it
    // (tests/calendars-reading.json): Date's Calendars steps into the
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
        expect("each plate is in its own script",
            plates.map(plate => plate.script).join(", ") === "chinese, japanese, hebrew, arabic",
            plates.map(plate => plate.script).join(", "));
        expect("each plate is titled with its date there",
            plates[2].title === "\u05db\u05f4\u05d4 \u05d1\u05d7\u05e9\u05d5\u05d5\u05df "
                + "\u05d4\u05f3\u05ea\u05e9\u05e4\u05f4\u05d6", plates[2].title);
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
        const servers = row("time:servers").verbs();
        expect("the servers are each a thing, the chosen first",
            servers.map(each => `${each.name} ${each.note}`).join(", ")
                === "time.harbour.test chosen, far.harbour.test out of reach",
            servers.map(each => `${each.name} ${each.note}`).join(", "));
        // The launcher knows a row by what it is, or else its key, keeping
        // the selection on it as the reading changes and taking two it knows
        // alike as one, so two servers known alike would pull it back to the
        // first.
        const known = servers.map(each => each.is ?? each.key);
        expect("each server is known as itself", new Set(known).size === servers.length,
            known.join(", "));
        const chosen = servers[0];
        const figures = chosen.verbs().map(each => each.name);
        const plate = chosen.drawer();
        expect("the chosen server holds every figure the vocabulary names for one",
            figures.join(", ") === ids("time:server").join(", "), figures.join(", "));
        expect("its plate says the same figures",
            plate.facts.map(fact => fact.label).join(", ") === figures.join(", "),
            JSON.stringify(plate.facts));
    }

    // The time here and the date as Now shows them: each wearing its own
    // place's glyph, as every row of Now does; the zone's next change said
    // as the launcher says a day with its time ("14 Mar 2027 03:00"), the
    // middle dot being kept for a time and how far off it is; the zone's
    // plate with an eyebrow, as every plate has; and a day's number copied
    // as a figure to paste, with nothing grouping it.
    function asNowShows() {
        const time = clocks.answer("time");
        const date = today.answer("date");
        expect("Time and Date wear their places' glyphs",
            !!Theme.glyph.clock && time.glyph === Theme.glyph.clock
                && !!Theme.glyph.date && date.glyph === Theme.glyph.date,
            `${time.glyph}, ${date.glyph}`);
        const zone = time.rows.find(row => row.name === Words.name("time:zone"));
        const next = zone.verbs().find(row => row.name === Words.name("time:zone:next"));
        expect("the zone's next change is a day and its time",
            /^\d{1,2} [A-Z][a-z]{2}( \d{4})? \d{2}:\d{2}$/.test(next.side), next.side);
        const plate = typeof zone.drawer === "function" ? zone.drawer() : zone.drawer;
        expect("the zone's plate has an eyebrow", (plate.eyebrow ?? "") !== "",
            JSON.stringify(plate));
        const numbers = ["today:unix-day", "today:julian", "today:mjd"].map(id => Words.name(id));
        const days = date.rows.filter(row => numbers.includes(row.name));
        expect("a day's number copies ungrouped",
            days.length === numbers.length && days.every(row => /^\d+$/.test(row.copy)),
            days.map(row => row.copy).join(", "));
    }
}
