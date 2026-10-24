// The weather, here or anywhere, answered in the field: "weather" for here,
// "weather tokyo" or "tokyo weather" for anywhere, its parts a step in:
// Now, Rain, Wind, Today, Tomorrow and the day after. Rain says when it
// next rains, with a spark of the next twelve hours' chance, and holds
// those hours, each its chance and how much. "rain", "umbrella",
// "precipitation" or "when will it rain", here or "in" a place, answer
// with the rain itself, its hours a step in. Here is `place`, a name, or
// else the city the machine's time zone is named for (`zone`, which NixOS
// gives), and the footer says which. Forecasts are Open-Meteo's, one
// request for all of it, asked only when the weather is typed and kept
// for a quarter of an hour; places are found through its geocoder and kept
// for a day. `units` is "metric" or "imperial". With `lookups` set false it
// answers nothing.
import QtQuick
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["weather", "rain", "weather in tokyo"]
    // Their starts ask them too, from three letters: "wea", "rai", "umb".
    questions: ["weather", "rain", "umbrella"]

    readonly property bool imperial: settings.units === "imperial"
    readonly property var days: ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday",
        "Saturday"]

    // How long a place and a forecast are kept, in milliseconds.
    readonly property int placeKept: 24 * 60 * 60 * 1000
    readonly property int forecastKept: 15 * 60 * 1000
    // How many hours ahead the rain looks.
    readonly property int hoursAhead: 12
    // The chance, in percent, from which an hour is said to rain, and below
    // it the chance still worth saying rather than calling the hours dry.
    readonly property int likely: 50
    readonly property int possible: 30
    readonly property int percent: 100
    // The glyphs of the parts that aren't a weather code's.
    readonly property int windy: 0xf059d
    readonly property int rainy: 0xf0597

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

    // How much falls in an hour, in the units asked for: tenths of a
    // millimetre, or hundredths of an inch.
    function amount(value) {
        return imperial
            ? `${Numbers.format(value, 2, settings.group, settings.point)} in`
            : `${Numbers.format(value, 1, settings.group, settings.point)} mm`;
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

    // What's typed, read as a question: the weather or the rain, and the
    // place named, if any. Every word that asked for the weather still
    // does; the rain's words ask for it by name, and a place only after
    // "in" or "for", so "rain" never looks a place up by another word.
    function reading(typed) {
        const text = typed.trim().replace(/\?$/, "").trim();
        const weather = /^(?:weather|forecast)(?:\s+(?:in\s+|for\s+)?(.+))?$/i.exec(text)
            ?? /^(.+?)\s+(?:weather|forecast)$/i.exec(text);
        if (weather)
            return { rain: false, away: weather[1] ? weather[1].trim() : "" };
        const rain = new RegExp("^(?:rain|raining|rainy|umbrella|precipitation"
            + "|(?:when\\s+)?will\\s+it\\s+rain|(?:when\\s+)?is\\s+it\\s+going\\s+to\\s+rain)"
            + "(?:\\s+(?:in|for)\\s+(.+))?$", "i").exec(text);
        if (rain)
            return { rain: true, away: rain[1] ? rain[1].trim() : "" };
        return null;
    }

    // The next hours from the one it is now where the forecast is, by the
    // forecast's own clock: each its time, chance in percent and amount.
    function hoursOf(forecast) {
        const hourly = forecast.hourly ?? {};
        const times = hourly.time ?? [];
        const hour = (forecast.current?.time ?? "").slice(0, 13);
        const from = times.findIndex(time => time.slice(0, 13) >= hour);
        if (from < 0)
            return [];
        return times.slice(from, from + hoursAhead).map((time, i) => ({
            clock: time.slice(11, 16),
            chance: (hourly.precipitation_probability ?? [])[from + i] ?? 0,
            falls: (hourly.precipitation ?? [])[from + i] ?? 0
        }));
    }

    // When it next rains, in a few words: the first hour likely to, or the
    // most it might if none is, or that none will.
    function rainSaid(hours) {
        const wet = hours.findIndex(hour => hour.chance >= likely);
        if (wet === 0)
            return `now, ${hours[0].chance}%`;
        if (wet > 0)
            return `from ${hours[wet].clock}, ${hours[wet].chance}%`;
        const most = Math.max(0, ...hours.map(hour => hour.chance));
        if (most >= possible)
            return `unlikely, at most ${most}%`;
        return `none in the next ${hours.length} hours`;
    }

    // The next hours' chance as a spark, the latest last.
    function spark(hours) {
        return {
            shape: "spark",
            values: hours.map(hour => hour.chance / percent),
            warn: false
        };
    }

    // An hour a row, its chance a bar: a figure, so Return copies nothing.
    function hourRows(hours) {
        return hours.map(hour => copyRow({
            name: hour.clock,
            copy: "",
            side: `${hour.chance}%, ${amount(hour.falls)}`,
            glyph: String.fromCodePoint(rainy),
            graph: { shape: "bar", values: [hour.chance / percent], warn: false }
        }));
    }

    function answer(typed) {
        if (settings.lookups === false)
            return null;
        const asked = reading(typed);
        if (!asked)
            return null;
        const away = asked.away;
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
            + encodeURIComponent(name), placeKept);
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
        const units = imperial
            ? "&temperature_unit=fahrenheit&wind_speed_unit=mph&precipitation_unit=inch" : "";
        const forecast = ask("https://api.open-meteo.com/v1/forecast"
            + `?latitude=${place.latitude}&longitude=${place.longitude}`
            + "&current=temperature_2m,weather_code,is_day,wind_speed_10m"
            + "&hourly=precipitation_probability,precipitation"
            + "&daily=weather_code,temperature_2m_max,temperature_2m_min"
            + `&forecast_days=3&timezone=auto${units}`, forecastKept);
        if (!forecast.value)
            return waiting(forecast, "Open-Meteo");
        const where = away === ""
            ? [place.name, here.from].filter(part => part !== "").join(", ")
            : [place.name, place.country].filter(part => part).join(", ");
        const note = `${where} \u00b7 Open-Meteo`;
        const hours = hoursOf(forecast.value);
        if (asked.rain)
            return rainAnswer(hours, note);
        return weatherAnswer(forecast.value, hours, away !== "", note);
    }

    // The rain found on its own: when it next rains as the headline, with
    // the spark beside it, and its hours a step in.
    function rainAnswer(hours, note) {
        if (hours.length === 0)
            return {
                headline: "no hourly forecast",
                copy: "",
                rows: [],
                note: note
            };
        const when = rainSaid(hours);
        return {
            headline: when.startsWith("none") ? `no rain in the next ${hours.length} hours`
                : `rain ${when}`,
            graph: spark(hours),
            rows: hourRows(hours),
            note: note
        };
    }

    // The weather, what it's like now as the headline, and its parts in
    // the same order every time, flat: Now, Rain, Wind, Today, Tomorrow,
    // then the day after.
    function weatherAnswer(forecast, hours, away, note) {
        const now = forecast.current;
        const daily = forecast.daily;
        const day = now.is_day === 1;
        const speed = imperial ? "mph" : "km/h";
        const headline = `${degrees(now.temperature_2m)} ${said(now.weather_code)}`;
        // The time where it is, from the forecast's own clock, for elsewhere.
        const [date, clock] = (now.time ?? "").split("T");
        const weekday = date => {
            const [y, m, d] = date.split("-").map(Number);
            return days[new Date(y, m - 1, d).getDay()];
        };
        const rows = [copyRow({
            name: away && clock ? `Now, ${clock} ${weekday(date)} there` : "Now",
            copy: headline,
            side: headline,
            glyph: glyph(now.weather_code, day)
        })];
        // Rain holds its hours, and Return steps into them, since copying
        // them would be copying a table.
        if (hours.length > 0) {
            const inside = hourRows(hours);
            rows.push(thing({
                name: "Rain",
                side: rainSaid(hours),
                verb: "more",
                hue: "voice",
                plain: true,
                glyph: String.fromCodePoint(rainy),
                graph: spark(hours),
                verbs: () => inside
            }));
        }
        const wind = `${Math.round(now.wind_speed_10m)} ${speed}`;
        rows.push(copyRow({
            name: "Wind",
            copy: wind,
            side: wind,
            glyph: String.fromCodePoint(windy)
        }));
        for (let i = 0; i < (daily.time ?? []).length; i++) {
            const words = `${degrees(daily.temperature_2m_min[i])} to `
                + `${degrees(daily.temperature_2m_max[i])} \u00b7 ${said(daily.weather_code[i])}`;
            rows.push(copyRow({
                name: i === 0 ? "Today" : i === 1 ? "Tomorrow" : weekday(daily.time[i]),
                copy: words,
                side: words,
                glyph: glyph(daily.weather_code[i], true)
            }));
        }
        return {
            headline: headline,
            rows: rows,
            note: note
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
