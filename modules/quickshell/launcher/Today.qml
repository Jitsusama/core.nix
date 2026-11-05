// The date and what's known of it, answered in the field: "date" or
// "today", or a part of it by its own words ("week", "julian day",
// "moon"). The headline is the date in full; the rows are the calendar's
// alone, never a time, which is Time's (VKCF3Z's ruling, 2026-10-30:
// "Date holds the calendar only ... Nothing repeats"): the ISO date, the
// ISO week date, the ordinal date, the quarter, the Unix day, the Julian
// day and the modified one, each copied by Return, then the season and the
// moon, each a thing stepping into its figures, with its drawer saying
// them. It's all reckoned from the clock (Moment.js), the season where the
// laptop is, so nothing is asked, but for Calendars: the day in the
// Chinese, Japanese, Hebrew and Islamic calendars, which timekeeping
// reckons with ICU4X (Calendars.qml), asked once a day while Date is
// shown. Each is a thing, its side in Latin letters and its drawer in its
// own script, in that script's cut of the theme's faces (VKCF3Z's design,
// 2026-10-30).
import qs
import QtQuick
import "Moment.js" as Moment
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

    // A thing of the date's, as the season and the moon are: its figures a
    // step in, each copied, and its drawer saying them all.
    function partThing(id, side, note, facts) {
        return thingRow({
            key: `today:${id}`,
            name: Words.name(`today:${id}`),
            side: side,
            note: note,
            glyph: Theme.glyph.date,
            thing: true,
            drawer: () => ({
                title: side,
                facts: facts.map(fact => ({ label: Words.name(`today:${id}:${fact.id}`),
                    value: fact.side }))
            })
        }, () => facts.map(fact => copyRow({
            name: Words.name(`today:${id}:${fact.id}`),
            side: fact.side,
            copy: fact.side,
            note: fact.note ?? "",
            glyph: ""
        })));
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
    // calendars are asked for, once, and the date is answered again as
    // they come.
    readonly property bool shown: launcher !== null && launcher.open
        && (launcher.answer?.live === provider
            || (launcher.held ?? []).some(step => step.row?.live === provider))
    onShownChanged: {
        if (shown)
            Calendars.ask(Moment.iso(new Date()));
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

    function calendarThing(calendar, now) {
        const id = `today:calendars:${calendar.reckoning}`;
        return thingRow({
            key: id,
            name: Words.name(id),
            side: calendar.latin,
            note: Words.of(id).said?.note ?? "",
            glyph: Theme.glyph.date,
            thing: true,
            drawer: () => ({
                eyebrow: Words.name(id),
                title: calendar.native,
                script: scripts[calendar.reckoning],
                // A figure its script has no word for, a count or a day,
                // is said in Latin letters and set in the theme's face.
                facts: calendar.figures.map(figure => {
                    const value = nativeOf(figure, now);
                    return { label: Words.name(`today:calendars:${figure.id}`), value: value,
                        latin: value === latinOf(figure, now) };
                })
            })
        }, () => calendar.figures.map(figure => {
            const side = latinOf(figure, now);
            return copyRow({ name: Words.name(`today:calendars:${figure.id}`), side: side,
                copy: side, note: "", glyph: "" });
        }));
    }

    // Today in every calendar, stepping into each, once they're reckoned;
    // until then it says it's asking, or that the reader isn't answering.
    function calendarsRow(at, now) {
        const reading = Calendars.reading;
        const said = calendarsSaid;
        if (reading?.day !== Moment.iso(at)) {
            const side = Calendars.refused ? said.silent : "\u2026";
            return copyRow({ name: Words.name("today:calendars"), side: side,
                copiesNothing: true, note: "", glyph: Theme.glyph.date });
        }
        return holderRow({
            key: "today:calendars",
            name: Words.name("today:calendars"),
            side: String(reading.calendars.length),
            glyph: Theme.glyph.date
        }, () => reading.calendars.map(calendar => calendarThing(calendar, now)));
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
