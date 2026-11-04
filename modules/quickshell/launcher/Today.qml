// The date and what's known of it, answered in the field: "date" or
// "today", or a part of it by its own words ("week", "julian day",
// "moon"). The headline is the date in full; the rows are the calendar's
// alone, never a time, which is Time's (VKCF3Z's ruling, 2026-10-30:
// "Date holds the calendar only ... Nothing repeats"): the ISO date, the
// ISO week date, the ordinal date, the quarter, the Unix day, the Julian
// day and the modified one, each copied by Return, then the season and the
// moon, each a thing stepping into its figures, with its drawer saying
// them. It's all reckoned from the clock (Moment.js), the season where the
// laptop is, so nothing is asked.
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

    function told(now) {
        const at = new Date(now);
        const row = (id, side, note) => copyRow({ name: Words.name(`today:${id}`), side: side,
            copy: side, note: note ?? "", glyph: "" });
        const week = Moment.isoWeek(at);
        const days = Moment.daysIn(at.getFullYear());
        const day = Moment.ordinal(at);
        const quarter = Moment.quarter(at);
        return {
            headline: Moment.day(at),
            // Its place's glyph, as Now's rows each wear theirs, rather
            // than its first row's.
            glyph: Theme.glyph.date,
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
                moonThing(now)
            ],
            note: said("today", "from")
        };
    }
}
