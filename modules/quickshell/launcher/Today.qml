// The date and what's known of it, answered in the field: "date" or
// "today", or a part of it by its own words ("week", "julian day",
// "moon"). The headline is the date in full; the rows are the calendar's
// alone, never a time, which is Time's (VKCF3Z's ruling, 2026-10-30:
// "Date holds the calendar only ... Nothing repeats"): the ISO date, the
// ISO week date, the ordinal date, the quarter, the Unix day, the Julian
// day and the modified one, each copied by Return, then the season and the
// moon, each copied by Return too, with its drawer saying its figures,
// since a step of copies alone is no step (Joel, 2026-11-11). The date's
// own drawer says the week and the day of the year, the season and the
// moon (NowDrawers.js). It's all reckoned from the clock (Moment.js), the season where the
// laptop is, so nothing is asked, but for Calendars: the day in the
// Chinese, Japanese, Hebrew and Islamic calendars, which timekeeping
// reckons with ICU4X (Calendars.qml), held ready from the shell's start,
// and asked again as Date is shown only if it went unanswered. Each is a
// thing, its side in Latin letters and its drawer in its own script, in
// that script's cut of the theme's faces (VKCF3Z's design, 2026-10-30).
import qs
import QtQuick
import "Moment.js" as Moment
import "NowDrawers.js" as NowDrawers
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["date", "moon"]

    // The words it answers whole, and by their start (Provider.qml).
    questions: Words.answers("today")

    function answer(typed) {
        if (asks(typed) === "")
            return null;
        return told(Date.now());
    }

    // Words the date's parts say, as the vocabulary holds them.
    function said(id, key, fill) {
        let words = String((Words.of(id).said ?? {})[key] ?? key);
        for (const name of Object.keys(fill ?? {}))
            words = words.replace(`{${name}}`, fill[name]);
        return words;
    }

    function grouped(number) {
        return Numbers.format(number, 0, settings.group ?? ",", settings.point ?? ".");
    }

    // A day's number, grouped to read and copied whole, as a figure to
    // paste is (Y2YBTS's review, 2026-11-04).
    function dayNumber(id, number, note) {
        return copyRow({ name: Words.name(`today:${id}`), side: grouped(number),
            copy: String(number), note: note, glyph: "" });
    }

    // A thing of the date's, as the season and the moon are: Return copies
    // what it is, and its drawer says its figures.
    function partThing(id, side, note, facts) {
        return copyRow({
            key: `today:${id}`,
            name: Words.name(`today:${id}`),
            side: side,
            copy: side,
            note: note,
            glyph: Theme.glyph.date,
            thing: true,
            drawer: () => ({
                title: side,
                facts: facts.map(fact => ({ label: Words.name(`today:${id}:${fact.id}`),
                    value: fact.side }))
            })
        });
    }

    // Date's drawer: the week, the day of the year and the days left in
    // it, the season's days to go, then the date, the season, the moon and
    // the quarter, as VKCF3Z's mock has them (NowDrawers.js).
    function dateDrawer(now) {
        const at = new Date(now);
        const season = Moment.season(now, Location.here?.latitude);
        const moon = Moment.moon(now);
        const quarter = Moment.quarter(at);
        return NowDrawers.date({
            iso: Moment.iso(at),
            week: Moment.isoWeek(at).week,
            day: Moment.ordinal(at),
            days: Moment.daysIn(at.getFullYear()),
            season: said("today:season", season.season),
            seasonLeft: Math.ceil((season.ends - now) / 86400000),
            moon: said("today:moon", moon.phase),
            lit: said("today:moon", "lit", { percent: `${Math.round(moon.lit * 100)}%` }),
            quarter: quarter.quarter,
            quarterDay: quarter.day,
            quarterDays: quarter.days,
            weekDate: Moment.isoWeekDate(at),
            dayNumber: String(Moment.dayNumber(at)),
            said: Words.of("today").said ?? {},
            names: { title: Words.name("today"), iso: Words.name("today:iso"),
                season: Words.name("today:season"), moon: Words.name("today:moon"),
                quarter: Words.name("today:quarter"), weekDate: Words.name("today:week"),
                julian: Words.name("today:julian") }
        });
    }

    // The season where the laptop is, the north's while that's unknown.
    function seasonThing(now) {
        const latitude = Location.here?.latitude;
        const season = Moment.season(now, latitude);
        const length = Math.round((season.ends - season.began) / 86400000);
        const into = Math.floor((now - season.began) / 86400000) + 1;
        const name = said("today:season", season.season);
        return partThing("season", name, said("today:season", "left",
            { days: String(Math.ceil((season.ends - now) / 86400000)) }), [
            { id: "began", side: Moment.dayOf(season.began, now) },
            { id: "ends", side: Moment.dayOf(season.ends, now) },
            { id: "day", side: said("today:season", "of", { day: String(into),
                days: String(length) }) },
            { id: "hemisphere", side: said("today:season", typeof latitude !== "number"
                ? "unknown" : latitude < 0 ? "south" : "north") }
        ]);
    }

    function moonThing(now) {
        const moon = Moment.moon(now);
        const percent = `${Math.round(moon.lit * 100)}%`;
        return partThing("moon", said("today:moon", moon.phase),
            said("today:moon", "lit", { percent: percent }), [
            { id: "lit", side: percent },
            { id: "age", side: said("today:moon", "days", { days: moon.age.toFixed(1) }) },
            { id: "full", side: Moment.dayOf(moon.full, now) },
            { id: "new", side: Moment.dayOf(moon.new, now) }
        ]);
    }

    // While the date is shown, as the answer or a step held in it, today's
    // calendars are asked for from where the laptop is, as the weather's
    // here reads it, which is nothing unless the answer held went missing
    // or the reader refused, and the date is answered again as they come.
    readonly property bool shown: launcher !== null && launcher.open
        && (launcher.answer?.live === provider
            || (launcher.held ?? []).some(step => step.row?.live === provider))
    // Asked once the change has been told, never inside it: asking again
    // after a refusal clears it at once, which answers the date again,
    // which changes the launcher's answer `shown` reads, so asked here it
    // would loop back into `shown` while it's still being set.
    onShownChanged: {
        if (shown)
            Qt.callLater(provider.askCalendars);
    }
    function askCalendars() {
        if (shown)
            Calendars.ask(Moment.iso(new Date()), Location.here);
    }

    Connections {
        target: Calendars
        function onReadingChanged() {
            if (provider.shown)
                provider.answered();
        }
        function onRefusedChanged() {
            if (provider.shown)
                provider.answered();
        }
    }

    readonly property var calendarsSaid: Words.of("today:calendars").said ?? {}

    // The script each calendar is written in, by the theme's script names.
    readonly property var scripts: ({
        chinese: "chinese",
        japanese: "japanese",
        hebrew: "hebrew",
        islamic: "arabic"
    })

    // The words a count is said in, by the figure it is.
    readonly property var counts: ({ monthDays: "days", yearDays: "days", months: "months" })

    // A calendar's figure in Latin letters: a day it's reckoned from as
    // the launcher says a day, and a count in its words.
    function latinOf(figure, now) {
        if (figure.day) {
            const [year, month, day] = figure.day.split("-").map(Number);
            return Moment.dayOf(new Date(year, month - 1, day).getTime(), now);
        }
        const counted = calendarsSaid[counts[figure.id] ?? ""];
        return counted ? counted.replace("{value}", figure.latin) : figure.latin;
    }

    // The same in the calendar's own script where it has its own, as a
    // Chinese month's length, 大月 for a long one, and otherwise as in Latin
    // letters.
    function nativeOf(figure, now) {
        return figure.native !== figure.latin && !figure.day ? figure.native
            : latinOf(figure, now);
    }

    // When a day that begins at sunset begins, where Date is seen from: the
    // time today's sunset begins the next, "Day Began" once it has, in the
    // figures' face. Seen from nowhere it's said to begin at sunset, and on
    // a day the sun doesn't set that it doesn't, as words, each with the
    // civil day kept (VKCF3Z, 2026-11-05).
    function dayBegins(evening, now) {
        const sunset = evening?.sunset ?? null;
        const name = Words.name(sunset !== null && now >= sunset ? "today:calendars:dayBegan"
            : "today:calendars:dayBegins");
        if (sunset !== null)
            return { name: name, value: Moment.clock(new Date(sunset)), words: false };
        return { name: name, value: evening ? calendarsSaid.noSunset : calendarsSaid.atSunset,
            words: true };
    }

    // A calendar's figures, each as its row says it in Latin letters and its
    // plate in its script, and when its day begins just after its Day where
    // it begins at sunset.
    function figuresOf(calendar, now, begins) {
        const figures = calendar.figures.map(figure => {
            const latin = latinOf(figure, now);
            const value = nativeOf(figure, now);
            return { name: Words.name(`today:calendars:${figure.id}`), side: latin, value: value,
                latin: value === latin, words: false };
        });
        if (calendar.fromSunset)
            figures.splice(calendar.figures.findIndex(figure => figure.id === "day") + 1, 0,
                { name: begins.name, side: begins.value, value: begins.value, latin: true,
                    words: begins.words });
        return figures;
    }

    function calendarThing(calendar, now, begins) {
        const id = `today:calendars:${calendar.reckoning}`;
        const figures = figuresOf(calendar, now, begins);
        return thingRow({
            key: id,
            name: Words.name(id),
            side: calendar.latin,
            glyph: Theme.glyph.date,
            thing: true,
            drawer: () => ({
                eyebrow: Words.name(id),
                title: calendar.native,
                script: scripts[calendar.reckoning],
                // A figure its script has no word for, a count or a day,
                // is said in Latin letters and set in the theme's face, and
                // words with no figure in them in the face words are read in.
                facts: figures.map(each => ({ label: each.name, value: each.value,
                    latin: each.latin, words: each.words }))
            })
        // Each figure copies what it says, when a day begins as words
        // ("at sunset") too: that's its value, and copying nothing left
        // Return nothing to do on it (Joel, 2026-10-26: every row has a
        // Return act, and one that only shows a value copies it).
        }, () => figures.map(each => copyRow({ name: each.name, side: each.side,
            copy: each.side, note: "", glyph: "" })));
    }

    // Today in every calendar, stepping into each, once they're reckoned;
    // until then it says it's asking, or that the reader isn't answering.
    // Asking, it's keyed as the answer will be, so the step holding Date
    // takes the answer in its place: keyed apart, the asking row stayed
    // where it was, dimmed and doing nothing, and the answer came in at
    // the end (Joel, 2026-11-07: "Now -> Date -> Calendars does not work
    // from the launcher").
    function calendarsRow(at, now) {
        const reading = Calendars.reading;
        const said = calendarsSaid;
        if (reading?.day !== Moment.iso(at)) {
            const side = Calendars.refused ? said.silent : "\u2026";
            return copyRow({ key: "today:calendars", name: Words.name("today:calendars"),
                side: side,
                copiesNothing: true, note: "", glyph: Theme.glyph.date });
        }
        // From sunset, a calendar whose day begins then is on the day begun;
        // before the day before's sunset, where it comes after midnight far
        // north in summer, it's still on the day before's. It's reckoned
        // with the day, so a plate open across sunset moves the next time
        // Date is shown, and nothing waits on the sun.
        const has = sunset => sunset !== null && sunset !== undefined;
        const eve = reading.eve ?? null;
        const unended = has(eve?.sunset) && now < eve.sunset;
        const evening = unended ? eve : reading.evening ?? null;
        const begins = dayBegins(evening, now);
        const begun = !unended && has(evening?.sunset) && now >= evening.sunset;
        const onIt = calendar => !calendar.fromSunset || !(begun || unended) ? calendar
            : evening.calendars.find(each => each.reckoning === calendar.reckoning) ?? calendar;
        return holderRow({
            key: "today:calendars",
            name: Words.name("today:calendars"),
            side: String(reading.calendars.length),
            glyph: Theme.glyph.date
        }, () => reading.calendars.map(calendar => calendarThing(onIt(calendar), now, begins)));
    }

    function told(now) {
        const at = new Date(now);
        const row = (id, side, note) => copyRow({ name: Words.name(`today:${id}`), side: side,
            copy: side, note: note ?? "", glyph: "" });
        const week = Moment.isoWeek(at);
        const days = Moment.daysIn(at.getFullYear());
        const day = Moment.ordinal(at);
        const quarter = Moment.quarter(at);
        return {
            headline: Moment.day(at, now),
            // Copied, it's pasted somewhere this year isn't plain, so it
            // keeps its year.
            copy: Moment.fullDay(at),
            // Its place's glyph, as Now's rows each wear theirs, rather
            // than its first row's.
            glyph: Theme.glyph.date,
            // It's Date's, so Date knows when it's shown and asks for the
            // day's calendars.
            live: provider,
            // A thing with a drawer, a little more than its line, made
            // only as it's shown (Joel, 2026-11-11).
            thing: true,
            drawer: () => provider.dateDrawer(now),
            rows: [
                row("iso", Moment.iso(at)),
                row("week", Moment.isoWeekDate(at), said("today:week", "week",
                    { week: String(week.week) })),
                row("ordinal", `${at.getFullYear()}-${String(day).padStart(3, "0")}`,
                    said("today:ordinal", "left", { left: String(days - day),
                        days: String(days) })),
                row("quarter", `Q${quarter.quarter}`, said("today:quarter", "day",
                    { day: String(quarter.day), days: String(quarter.days) })),
                dayNumber("unix-day", Moment.unixDay(at), said("today:unix-day", "since")),
                dayNumber("julian", Moment.dayNumber(at), said("today:julian", "noon")),
                dayNumber("mjd", Moment.modified(at), said("today:mjd", "since")),
                seasonThing(now),
                moonThing(now),
                calendarsRow(at, now)
            ],
            note: said("today", "from")
        };
    }
}
