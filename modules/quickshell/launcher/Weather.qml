// The weather, here or anywhere, answered in the field: "weather" for here,
// "weather tokyo" or "tokyo weather" for anywhere, now and for the next
// days. Here is `place`, a name, or else the city the machine's time zone
// is named for (`zone`, which NixOS gives), and the footer says which.
// Forecasts are Open-Meteo's, asked only when the weather is typed and kept
// for a quarter of an hour; places are found through its geocoder and kept
// for a day. `units` is "metric" or "imperial". With `lookups` set false it
// answers nothing.
import QtQuick
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["weather", "weather in tokyo"]

    readonly property bool imperial: settings.units === "imperial"
    readonly property var days: ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday",
        "Saturday"]

    // WMO's weather codes, as Open-Meteo gives them: what each says, and its
    // glyph by day and by night.
    readonly property var codes: ({
        0: ["clear", 0xf0599, 0xf0594],
        1: ["mostly clear", 0xf0595, 0xf0f31],
        2: ["partly cloudy", 0xf0595, 0xf0f31],
        3: ["overcast", 0xf0590, 0xf0590],
        45: ["fog", 0xf0591, 0xf0591],
        48: ["freezing fog", 0xf0591, 0xf0591],
        51: ["light drizzle", 0xf0f33, 0xf0597],
        53: ["drizzle", 0xf0597, 0xf0597],
        55: ["heavy drizzle", 0xf0597, 0xf0597],
        56: ["freezing drizzle", 0xf067f, 0xf067f],
        57: ["freezing drizzle", 0xf067f, 0xf067f],
        61: ["light rain", 0xf0f33, 0xf0597],
        63: ["rain", 0xf0597, 0xf0597],
        65: ["heavy rain", 0xf0596, 0xf0596],
        66: ["freezing rain", 0xf067f, 0xf067f],
        67: ["freezing rain", 0xf067f, 0xf067f],
        71: ["light snow", 0xf0598, 0xf0598],
        73: ["snow", 0xf0598, 0xf0598],
        75: ["heavy snow", 0xf0f36, 0xf0f36],
        77: ["snow grains", 0xf0598, 0xf0598],
        80: ["showers", 0xf0f33, 0xf0597],
        81: ["showers", 0xf0597, 0xf0597],
        82: ["heavy showers", 0xf0596, 0xf0596],
        85: ["snow showers", 0xf0598, 0xf0598],
        86: ["heavy snow showers", 0xf0f36, 0xf0f36],
        95: ["thunderstorms", 0xf0593, 0xf0593],
        96: ["thunderstorms and hail", 0xf067e, 0xf067e],
        99: ["thunderstorms and hail", 0xf067e, 0xf067e]
    })

    function said(code) {
        return (codes[code] ?? ["unsettled"])[0];
    }

    function glyph(code, day) {
        const known = codes[code] ?? ["", 0xf0590, 0xf0590];
        return String.fromCodePoint(day ? known[1] : known[2]);
    }

    function degrees(value) {
        return `${Numbers.format(Math.round(value), 0, settings.group, settings.point)}\u00b0`;
    }

    // Where "here" is, and how the footer says it was found. Not home(),
    // which is what a provider puts on the launcher's home screen.
    function local() {
        if (settings.place)
            return { name: settings.place, from: "" };
        if (!settings.zone)
            return null;
        const city = settings.zone.split("/").pop().replace(/_/g, " ");
        return { name: city, from: "from your time zone" };
    }

    function answer(typed) {
        if (settings.lookups === false)
            return null;
        const text = typed.trim();
        const asked = /^(?:weather|forecast)(?:\s+(?:in\s+|for\s+)?(.+))?$/i.exec(text)
            ?? /^(.+?)\s+(?:weather|forecast)$/i.exec(text);
        if (!asked)
            return null;
        const away = asked[1] ? asked[1].trim() : "";
        const here = local();
        if (away === "" && !here)
            return {
                headline: "where?",
                copy: "",
                rows: [],
                note: "say where here is with the weather provider's place"
            };
        const name = away === "" ? here.name : away;
        const found = ask("https://geocoding-api.open-meteo.com/v1/search?count=1&name="
            + encodeURIComponent(name), 24 * 60 * 60 * 1000);
        if (!found.value)
            return waiting(found, "Open-Meteo");
        const place = (found.value.results ?? [])[0];
        if (!place)
            return {
                headline: "no such place",
                copy: "",
                rows: [],
                note: `Open-Meteo knows no \u201c${name}\u201d`
            };
        const units = imperial ? "&temperature_unit=fahrenheit&wind_speed_unit=mph" : "";
        const forecast = ask("https://api.open-meteo.com/v1/forecast"
            + `?latitude=${place.latitude}&longitude=${place.longitude}`
            + "&current=temperature_2m,weather_code,is_day,wind_speed_10m"
            + "&daily=weather_code,temperature_2m_max,temperature_2m_min"
            + `&forecast_days=3&timezone=auto${units}`, 15 * 60 * 1000);
        if (!forecast.value)
            return waiting(forecast, "Open-Meteo");
        const now = forecast.value.current;
        const daily = forecast.value.daily;
        const day = now.is_day === 1;
        const speed = imperial ? "mph" : "km/h";
        const headline = `${degrees(now.temperature_2m)} ${said(now.weather_code)}`;
        // The time where it is, from the forecast's own clock, for elsewhere.
        const [date, clock] = (now.time ?? "").split("T");
        const weekday = date => {
            const [y, m, d] = date.split("-").map(Number);
            return days[new Date(y, m - 1, d).getDay()];
        };
        // The headline says what it's like now, so its row adds the wind.
        const rows = [copyRow({
            name: `wind ${Math.round(now.wind_speed_10m)} ${speed}`,
            copy: headline,
            side: away === "" || !clock ? "now" : `${clock} ${weekday(date)} there`,
            glyph: glyph(now.weather_code, day)
        })];
        for (let i = 0; i < (daily.time ?? []).length; i++) {
            const words = `${degrees(daily.temperature_2m_min[i])} to `
                + `${degrees(daily.temperature_2m_max[i])} \u00b7 ${said(daily.weather_code[i])}`;
            rows.push(copyRow({
                name: words,
                side: i === 0 ? "today" : i === 1 ? "tomorrow" : weekday(daily.time[i]),
                glyph: glyph(daily.weather_code[i], true)
            }));
        }
        const where = away === ""
            ? [place.name, here.from].filter(part => part !== "").join(", ")
            : [place.name, place.country].filter(part => part).join(", ");
        return {
            headline: headline,
            rows: rows,
            note: `${where} \u00b7 Open-Meteo`
        };
    }

    function waiting(entry, who) {
        return {
            headline: entry.failed ? "no forecast" : "\u2026",
            copy: "",
            rows: [],
            note: entry.failed ? `${who} didn't answer` : `asking ${who}`,
            asking: !entry.failed
        };
    }
}
