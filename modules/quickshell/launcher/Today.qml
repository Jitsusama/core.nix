// The date and what's known of it, answered in the field: "date" or
// "today", or a part alone: "week", "day of the year", "utc" or "epoch".
// The headline is the date
// in full; the rows are the week of the year, the day of the year and how
// many are left, the time in UTC, the zone with its offset, and the time
// as seconds since the epoch, each named first and copied by Return. It's
// all reckoned from the clock, so nothing is asked.
import qs
import QtQuick
import "Numbers.js" as Numbers
import "Zones.js" as Zones

Provider {
    id: provider

    examples: ["date", "epoch"]

    // The words it answers whole, and by their start (Provider.qml).
    questions: ["date", "today", "week", "day of the year", "utc", "epoch"]

    function answer(typed) {
        if (asks(typed) === "")
            return null;
        return told(new Date());
    }

    // The ISO week: Monday first, week 1 the one with the year's first
    // Thursday.
    function week(date) {
        const day = new Date(Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()));
        const weekday = day.getUTCDay() || 7;
        day.setUTCDate(day.getUTCDate() + 4 - weekday);
        const first = new Date(Date.UTC(day.getUTCFullYear(), 0, 1));
        return Math.ceil(((day - first) / 86400000 + 1) / 7);
    }

    function told(now) {
        const glyph = Theme.glyph.date;
        const day = Zones.dayOfYear(now);
        const days = new Date(now.getFullYear(), 1, 29).getMonth() === 1 ? 366 : 365;
        // The clock's own zone, which follows the laptop, before the one
        // the machine was built with.
        const zone = Location.clockZone || (settings.zone ?? "");
        const two = n => String(n).padStart(2, "0");
        const row = (id, side, note) => copyRow({ name: Words.name(`today:${id}`), side: side,
            note: note, glyph: glyph });
        // As the clocks write it, "UTC\u22124".
        const offset = Zones.offset(-now.getTimezoneOffset());
        return {
            headline: Qt.formatDate(now, "dddd d MMMM yyyy"),
            rows: [
                row("week", `${week(now)}`, "of the year, Monday first"),
                row("day", `${day}`, `${days - day} left of ${days}`),
                row("utc", `${two(now.getUTCHours())}:${two(now.getUTCMinutes())}`, ""),
                // The zone as the time's answer names it, its offset after.
                row("zone", zone !== "" ? zone : offset, zone !== "" ? offset : ""),
                row("epoch", Numbers.format(Math.floor(now.getTime() / 1000), 0, "", "."),
                    "seconds since 1970")
            ],
            note: "from the clock"
        };
    }
}
