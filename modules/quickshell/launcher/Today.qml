// The date and what's known of it, answered in the field: "date", "today",
// "day", "week", "utc", "epoch" or "unix time". The headline is the date
// in full; the rows are the week of the year, the day of the year and how
// many are left, the time in UTC, the zone with its offset, and the time
// as seconds since the epoch, each copied by Return. It's all reckoned from
// the clock, so nothing is asked.
import QtQuick
import "Numbers.js" as Numbers
import "Zones.js" as Zones

Provider {
    id: provider

    examples: ["date", "epoch"]

    // The words it answers whole, and by their start (Provider.qml).
    questions: ["date", "today", "day", "week", "week number", "what day is it", "what's the date",
        "day of the year", "utc", "gmt", "epoch", "unix time", "unix", "timestamp"]

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
        const glyph = String.fromCodePoint(0xf00ed);
        const day = Zones.dayOfYear(now);
        const days = new Date(now.getFullYear(), 1, 29).getMonth() === 1 ? 366 : 365;
        const zone = settings.zone ?? "";
        const two = n => String(n).padStart(2, "0");
        const row = (name, side) => copyRow({ name: name, side: side, glyph: glyph });
        return {
            headline: Qt.formatDate(now, "dddd d MMMM yyyy"),
            rows: [
                row(`week ${week(now)}`, "of the year, Monday first"),
                row(`day ${day}`, `of ${days}, ${days - day} left`),
                row(`${two(now.getUTCHours())}:${two(now.getUTCMinutes())}`, "UTC"),
                // As the clocks write it, "UTC\u22124".
                row(Zones.offset(-now.getTimezoneOffset()), zone !== "" ? zone : "the zone here"),
                row(Numbers.format(Math.floor(now.getTime() / 1000), 0, "", "."), "seconds since 1970")
            ],
            note: "from the clock"
        };
    }
}
