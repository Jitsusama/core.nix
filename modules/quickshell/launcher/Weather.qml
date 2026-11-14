// The weather, here or anywhere, answered in the field: "weather" for here,
// "weather tokyo" or "tokyo weather" for anywhere, its parts a step in:
// Conditions, Rain, then the seven days from today (Joel, 2026-11-04:
// "Weather, nerdy, by the hour, stepping into a day"), with elsewhere's
// Local Time after Conditions. Conditions is a thing holding every
// figure of what it's like this moment: the temperature, what it feels
// like, the humidity, the dew point, the precipitation, the wind and its
// gusts, the cloud cover, the UV index, the visibility and the pressure.
// Rain says when it next rains, with a spark of the next twelve hours'
// chance, and holds those hours, shown only while rain is due in them.
// A day is a thing holding its own
// figures, its high and low, the most it rains and blows, its sun, then
// its hours; an hour is a thing holding its figures, each the hour's own,
// so nothing a day says is said again by its hours (Y2YBTS, 2026-11-04).
// An hour is the same thing wherever it's listed, in Rain or in its day.
// "rain", "umbrella", "precipitation" or "when will it rain", here or
// "in" a place, answer with the rain itself, its hours a step in. Here is
// where the laptop is, as the shell's one place for it says
// (../Location.qml), else `place`, a name, or else the city the clock's
// time zone is named for, and the footer says which. The laptop's forecast
// is asked by its position, to a hundredth of a degree, and the footer
// says it by its town, or by that position while the town couldn't be
// asked. Forecasts are Open-Meteo's, one request for all of it, asked only
// when the weather is typed and kept for a quarter of an hour; other
// places are found through its geocoder and kept for a day. `units` is
// "metric" or "imperial". With `lookups` set false it answers nothing. The
// words that ask it, its parts' names and what they say are the
// vocabulary's (launcher/words.json, Words.qml).
import qs
import QtQuick
import "Moment.js" as Moment
import "NowDrawers.js" as NowDrawers
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["weather", "rain", "weather in tokyo"]
    // Their starts ask them too, from three letters: "wea", "rai", "umb".
    questions: Words.started("weather").concat(Words.started("weather:rain"))

    // The words that ask the weather, and the rain, as one pattern each:
    // any of them, in any case, its spaces any run of space.
    function either(id) {
        return Words.answers(id).map(word => word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
            .replace(/ /g, "\\s+")).join("|");
    }
    readonly property string weatherWords: either("weather")
    readonly property string rainWords: either("weather:rain")

    readonly property bool imperial: settings.units === "imperial"

    // How long a place and a forecast are kept, in milliseconds.
    readonly property int placeKept: 24 * 60 * 60 * 1000
    readonly property int forecastKept: 15 * 60 * 1000
    // How many hours ahead the rain looks, and how many days are asked.
    readonly property int hoursAhead: 12
    readonly property int daysAhead: 7
    // The chance, in percent, from which an hour is said to rain, and below
    // it the chance still worth saying rather than calling the hours dry.
    readonly property int likely: 50
    readonly property int possible: 30
    readonly property int percent: 100
    // What the forecast is asked for: everything an hour, a day and this
    // moment can say, in the one request.
    readonly property string instantFields: "temperature_2m,apparent_temperature,"
        + "relative_humidity_2m,dew_point_2m,precipitation,weather_code,is_day,cloud_cover,"
        + "wind_speed_10m,wind_direction_10m,wind_gusts_10m,uv_index,visibility,pressure_msl"
    readonly property string dailyFields: "weather_code,temperature_2m_max,temperature_2m_min,"
        + "apparent_temperature_max,apparent_temperature_min,precipitation_sum,"
        + "precipitation_hours,precipitation_probability_max,wind_speed_10m_max,"
        + "wind_gusts_10m_max,wind_direction_10m_dominant,uv_index_max,sunrise,sunset,"
        + "daylight_duration"
    // The sixteen points of the compass, from the north, as the wind is
    // said to blow from one.
    readonly property var points: ["N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW",
        "SW", "WSW", "W", "WNW", "NW", "NNW"]
    // WMO's weather codes, as Open-Meteo gives them: what each says, and the
    // theme's mark for it by day and by night.
    readonly property var codes: ({
        0: ["clear", "sunny", "clearNight"],
        1: ["mostly clear", "partlyCloudy", "partlyCloudyNight"],
        2: ["partly cloudy", "partlyCloudy", "partlyCloudyNight"],
        3: ["overcast", "cloudy", "cloudy"],
        45: ["fog", "fog", "fog"],
        48: ["freezing fog", "fog", "fog"],
        51: ["light drizzle", "showers", "rain"],
        53: ["drizzle", "rain", "rain"],
        55: ["heavy drizzle", "rain", "rain"],
        56: ["freezing drizzle", "sleet", "sleet"],
        57: ["freezing drizzle", "sleet", "sleet"],
        61: ["light rain", "showers", "rain"],
        63: ["rain", "rain", "rain"],
        65: ["heavy rain", "pouring", "pouring"],
        66: ["freezing rain", "sleet", "sleet"],
        67: ["freezing rain", "sleet", "sleet"],
        71: ["light snow", "snow", "snow"],
        73: ["snow", "snow", "snow"],
        75: ["heavy snow", "heavySnow", "heavySnow"],
        77: ["snow grains", "snow", "snow"],
        80: ["showers", "showers", "rain"],
        81: ["showers", "rain", "rain"],
        82: ["heavy showers", "pouring", "pouring"],
        85: ["snow showers", "snow", "snow"],
        86: ["heavy snow showers", "heavySnow", "heavySnow"],
        95: ["thunderstorms", "thunder", "thunder"],
        96: ["thunderstorms and hail", "hail", "hail"],
        99: ["thunderstorms and hail", "hail", "hail"]
    })

    function said(code) {
        return (codes[code] ?? ["unsettled"])[0];
    }

    function glyph(code, day) {
        const known = codes[code] ?? ["", "cloudy", "cloudy"];
        return Theme.glyph[day ? known[1] : known[2]];
    }

    // What the weather says, as the vocabulary holds it, each `{name}`
    // filled.
    function words(key, fill) {
        let text = String((Words.of("weather").said ?? {})[key] ?? key);
        for (const name of Object.keys(fill ?? {}))
            text = text.replace(`{${name}}`, fill[name]);
        return text;
    }

    function known(value) {
        return value !== undefined && value !== null;
    }

    function number(value, places) {
        return Numbers.format(value, places, settings.group, settings.point);
    }

    // Each figure as it's said, or null when the forecast didn't say it,
    // so a figure it lacks is left out rather than said as nothing.
    function degrees(value) {
        return known(value) ? `${number(Math.round(value), 0)}\u00b0` : null;
    }

    // How much falls, in the units asked for: tenths of a millimetre, or
    // hundredths of an inch.
    function amount(value) {
        if (!known(value))
            return null;
        return imperial ? `${number(value, 2)} in` : `${number(value, 1)} mm`;
    }

    function share(value) {
        return known(value) ? `${Math.round(value)}%` : null;
    }

    function speed(value) {
        return known(value) ? `${Math.round(value)} ${imperial ? "mph" : "km/h"}` : null;
    }

    function compass(bearing) {
        return points[Math.round((((bearing % 360) + 360) % 360) / 22.5) % points.length];
    }

    // The wind, as fast as it blows and where it blows from: "18 km/h from
    // the WSW".
    function wind(value, bearing) {
        if (!known(value))
            return null;
        return known(bearing) ? words("wind", { speed: speed(value), point: compass(bearing) })
            : speed(value);
    }

    // The UV index, and the band the WHO says it's in.
    function ultraviolet(value) {
        if (!known(value))
            return null;
        const band = value < 3 ? "low" : value < 6 ? "moderate" : value < 8 ? "high"
            : value < 11 ? "veryHigh" : "extreme";
        return words("uv", { index: number(value, 1), band: words(band) });
    }

    // How far can be seen, Open-Meteo giving it in metres or in feet as its
    // units say.
    function visibility(value, unit) {
        if (!known(value))
            return null;
        const metres = unit === "ft" ? value * 0.3048 : value;
        return imperial ? `${number(metres / 1609.344, 1)} mi` : `${number(metres / 1000, 1)} km`;
    }

    // The pressure at sea level: hectopascals, or inches of mercury.
    function pressure(value) {
        if (!known(value))
            return null;
        return imperial ? `${number(value * 0.02953, 2)} inHg`
            : `${Numbers.format(Math.round(value), 0, "", settings.point)} hPa`;
    }

    // An hour of the forecast's own clock, "07:32", from its local time.
    function clockOf(time) {
        return known(time) ? String(time).slice(11, 16) : null;
    }

    function daylight(seconds) {
        if (!known(seconds))
            return null;
        const minutes = Math.round(seconds / 60);
        return words("daylight", { hours: String(Math.floor(minutes / 60)),
            minutes: String(minutes % 60) });
    }

    // A date of the forecast's own, "2026-10-24", as the local calendar
    // has it.
    function dateOf(date) {
        const [year, month, day] = String(date).split("-").map(Number);
        return new Date(year, month - 1, day);
    }

    // What it's like at a moment: now, or an hour of the forecast. The
    // sky's words lead, then each figure, the hour's chance among them.
    function sky(at) {
        const warmth = degrees(at.temperature_2m);
        return warmth === null ? null : `${warmth} \u00b7 ${said(at.weather_code)}`;
    }

    function instant(at, unit) {
        return [
            { id: "temperature", side: degrees(at.temperature_2m) },
            { id: "feels", side: degrees(at.apparent_temperature) },
            { id: "humidity", side: share(at.relative_humidity_2m) },
            { id: "dew", side: degrees(at.dew_point_2m) },
            { id: "chance", side: share(at.precipitation_probability) },
            { id: "precipitation", side: amount(at.precipitation) },
            { id: "wind", side: wind(at.wind_speed_10m, at.wind_direction_10m),
                note: known(at.wind_direction_10m) ? `${Math.round(at.wind_direction_10m)}\u00b0`
                    : "" },
            { id: "gusts", side: speed(at.wind_gusts_10m) },
            { id: "cloud", side: share(at.cloud_cover) },
            { id: "uv", side: ultraviolet(at.uv_index) },
            { id: "visibility", side: visibility(at.visibility, unit) },
            { id: "pressure", side: pressure(at.pressure_msl) }
        ].filter(figure => figure.side !== null);
    }

    // A day's own figures, which none of its hours says: the most and the
    // least of it, and its sun.
    function dayFigures(daily, i) {
        const of = field => (daily[field] ?? [])[i];
        const low = degrees(of("apparent_temperature_min"));
        const high = degrees(of("apparent_temperature_max"));
        const wet = of("precipitation_hours");
        return [
            { id: "high", side: degrees(of("temperature_2m_max")) },
            { id: "low", side: degrees(of("temperature_2m_min")) },
            { id: "feels", side: low === null || high === null ? null
                : words("range", { low: low, high: high }) },
            { id: "chance", side: share(of("precipitation_probability_max")) },
            { id: "precipitation", side: amount(of("precipitation_sum")),
                note: known(wet) ? words("wetHours", { hours: number(wet, 0) }) : "" },
            { id: "wind", side: wind(of("wind_speed_10m_max"), of("wind_direction_10m_dominant")) },
            { id: "gusts", side: speed(of("wind_gusts_10m_max")) },
            { id: "uv", side: ultraviolet(of("uv_index_max")) },
            { id: "sunrise", side: clockOf(of("sunrise")) },
            { id: "sunset", side: clockOf(of("sunset")) },
            { id: "daylight", side: daylight(of("daylight_duration")) }
        ].filter(figure => figure.side !== null);
    }

    // A thing's figures a step in, each copied, and its drawer saying them
    // all: Conditions, a day, an hour.
    function figureRows(scope, figures) {
        return figures.map(figure => copyRow({
            name: Words.name(`${scope}:${figure.id}`),
            side: figure.side,
            copy: figure.side,
            note: figure.note ?? "",
            glyph: ""
        }));
    }

    function drawerOf(eyebrow, title, scope, figures) {
        return {
            eyebrow: eyebrow,
            title: title,
            facts: figures.map(figure => ({ label: Words.name(`${scope}:${figure.id}`),
                value: figure.side }))
        };
    }

    // Where the laptop is, as Location says it, or null while it's
    // unknown. A test gives one of its own.
    property var laptop: Location.here
    // A new here answers again what's shown, as a forecast arriving does.
    onLaptopChanged: answered()
    // The zone the clock keeps, as Location asks it, else the one the
    // machine was given.
    readonly property string zone: Location.clockZone || (settings.zone ?? "")

    // A position to a hundredth of a degree, about a kilometre, as near as
    // a forecast needs, so a forecast is kept for it whatever the last
    // digits.
    function hundredths(degrees) {
        return Math.round(degrees * 100) / 100;
    }

    // A position as it's said: "43.65° N 79.38° W".
    function position(latitude, longitude) {
        const part = (degrees, ahead, behind) => `${number(Math.abs(degrees), 2)}\u00b0 `
            + (degrees < 0 ? behind : ahead);
        return `${part(latitude, "N", "S")} ${part(longitude, "E", "W")}`;
    }

    // Where "here" is, and how the footer says it was found: the laptop's
    // position, then the place set, then the time zone's city. Not home(),
    // which is what a provider puts on the launcher's home screen.
    function local() {
        if (laptop) {
            const latitude = hundredths(laptop.latitude);
            const longitude = hundredths(laptop.longitude);
            return {
                name: laptop.place ?? position(latitude, longitude),
                latitude: latitude,
                longitude: longitude,
                from: "from where the laptop is"
            };
        }
        if (settings.place)
            return { name: settings.place, from: "from the place you set" };
        // A zone that names no city, as UTC or Etc/GMT+5, gives no here.
        if (!zone.includes("/") || zone.startsWith("Etc/"))
            return null;
        const city = zone.split("/").pop().replace(/_/g, " ");
        return { name: city, from: "from your time zone" };
    }

    // What's typed, read as a question: the weather or the rain, and the
    // place named, if any. Every word that asked for the weather still
    // does; the rain's words ask for it by name, and a place only after
    // "in" or "for", so "rain" never looks a place up by another word.
    function reading(typed) {
        const text = typed.trim().replace(/\?$/, "").trim();
        const weather = new RegExp(`^(?:${weatherWords})(?:\\s+(?:in\\s+|for\\s+)?(.+))?$`, "i")
            .exec(text) ?? new RegExp(`^(.+?)\\s+(?:${weatherWords})$`, "i").exec(text);
        if (weather)
            return { rain: false, away: weather[1] ? weather[1].trim() : "" };
        const rain = new RegExp(`^(?:${rainWords})(?:\\s+(?:in|for)\\s+(.+))?$`, "i").exec(text);
        if (rain)
            return { rain: true, away: rain[1] ? rain[1].trim() : "" };
        return null;
    }

    // Every hour of the forecast, by its own clock: its time, chance in
    // percent, amount, and every figure it gives, as `at`.
    function hoursOf(forecast) {
        const hourly = forecast.hourly ?? {};
        const unit = (forecast.hourly_units ?? {}).visibility ?? "m";
        return (hourly.time ?? []).map((time, i) => {
            const at = {};
            for (const field of Object.keys(hourly))
                at[field] = hourly[field][i];
            return {
                time: time,
                date: time.slice(0, 10),
                clock: time.slice(11, 16),
                chance: at.precipitation_probability ?? 0,
                falls: at.precipitation ?? 0,
                at: at,
                unit: unit
            };
        });
    }

    // The hours from the one it is now where the forecast is, by its own
    // clock.
    function ahead(forecast, hours) {
        const hour = (forecast.current?.time ?? "").slice(0, 13);
        const from = hours.findIndex(each => each.time.slice(0, 13) >= hour);
        return from < 0 ? [] : hours.slice(from);
    }

    // Whether rain is due in these hours: one of them likely to.
    function due(hours) {
        return hours.some(hour => hour.chance >= likely);
    }

    // When it next rains, in a few words, its chance parted from it as the
    // headline's facts are: the first hour likely to, or the most it might
    // if none is, or that none will.
    function rainSaid(hours) {
        const wet = hours.findIndex(hour => hour.chance >= likely);
        if (wet === 0)
            return `now \u00b7 ${hours[0].chance}%`;
        if (wet > 0)
            return `from ${hours[wet].clock} \u00b7 ${hours[wet].chance}%`;
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

    // An hour, the same thing wherever it's listed: its sky beside it, its
    // rain under it and as a bar, and every figure a step in. `today` is
    // the forecast's own, so its drawer says the year only of next year's
    // hours.
    function hourThing(hour, today) {
        const rain = `${hour.chance}% \u00b7 ${amount(hour.falls)}`;
        const above = sky(hour.at);
        const figures = instant(hour.at, hour.unit);
        return thingRow({
            name: hour.clock,
            side: above ?? rain,
            note: above === null ? "" : rain,
            glyph: known(hour.at.weather_code) ? glyph(hour.at.weather_code, hour.at.is_day !== 0)
                : Theme.glyph.rain,
            graph: { shape: "bar", values: [hour.chance / percent], warn: false },
            thing: true,
            drawer: () => drawerOf(words("at", { day: Moment.eyebrow(dateOf(hour.date), today),
                clock: hour.clock }), above ?? rain, "weather", figures)
        }, () => figureRows("weather", figures));
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
        // Where the laptop is needs no geocoder: its position is known.
        const fixed = away === "" && here.latitude !== undefined;
        const found = fixed ? null : ask("https://geocoding-api.open-meteo.com/v1/search"
            + `?count=1&name=${encodeURIComponent(name)}`, placeKept);
        if (found && !found.value)
            return waiting(found, "Open-Meteo");
        const place = fixed ? here : (found.value.results ?? [])[0];
        if (!place)
            return missed(`Open-Meteo knows no \u201c${name}\u201d.`);
        const units = imperial
            ? "&temperature_unit=fahrenheit&wind_speed_unit=mph&precipitation_unit=inch" : "";
        const forecast = ask("https://api.open-meteo.com/v1/forecast"
            + `?latitude=${place.latitude}&longitude=${place.longitude}`
            + `&current=${instantFields}`
            + `&hourly=${instantFields},precipitation_probability`
            + `&daily=${dailyFields}`
            + `&forecast_days=${daysAhead}&timezone=auto${units}`, forecastKept);
        if (!forecast.value)
            return waiting(forecast, "Open-Meteo");
        const where = away === ""
            ? [place.name, here.from].filter(part => part !== "").join(", ")
            : [place.name, place.country].filter(part => part).join(", ");
        const note = `${where} \u00b7 Open-Meteo`;
        const kind = asked.rain ? "rain" : "weather";
        const last = lastAnswers[kind];
        if (last && last.forecast === forecast.value && last.note === note)
            return last.answer;
        const hours = ahead(forecast.value, hoursOf(forecast.value));
        const answer = asked.rain ? rainAnswer(hours.slice(0, hoursAhead), note)
            : weatherAnswer(forecast.value, hours, away !== "", note);
        lastAnswers[kind] = { forecast: forecast.value, note: note, answer: answer };
        return answer;
    }

    // The last answer of each kind, the weather's and the rain's, with the
    // forecast and the place it said, given again while they're the same.
    // An answer reads its time from the forecast alone, so it's the same
    // until a forecast is heard anew. Now asks the weather at every
    // opening, and each opening made its rows again, though nothing had
    // changed (Joel's journal, 2026-11-12). Plain fields, so keeping one
    // notifies nothing.
    readonly property var lastAnswers: ({})

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
            rows: hours.map(hour => hourThing(hour, dateOf(hours[0].date))),
            note: note
        };
    }

    // The weather, what it's like now as the headline, and its parts in
    // the same order every time, flat: Conditions, elsewhere its Local
    // Time, Rain, then each day from today. Conditions is one noun
    // wherever it's for, since the note says where (VKCF3Z, 2026-11-09).
    function weatherAnswer(forecast, hours, away, note) {
        const now = forecast.current;
        const daily = forecast.daily ?? {};
        const headline = sky(now);
        // The time where it is, from the forecast's own clock, for elsewhere.
        const [date, clock] = (now.time ?? "").split("T");
        const weekday = date => Qt.formatDate(dateOf(date), "dddd");
        const figures = instant(now, (forecast.current_units ?? {}).visibility ?? "m");
        const named = Words.name("weather:now");
        const rows = [thingRow({
            name: named,
            side: headline,
            glyph: glyph(now.weather_code, now.is_day === 1),
            thing: true,
            drawer: () => drawerOf(named, headline, "weather", figures)
        }, () => figureRows("weather", figures))];
        if (away && clock) {
            const there = `${clock} ${weekday(date)}`;
            rows.push(copyRow({
                name: Words.name("weather:local-time"),
                side: there,
                copy: there,
                glyph: ""
            }));
        }
        // Rain holds its hours, a step in by the right arrow, and Return
        // copies when it next rains, as asked alone: a part holding more,
        // as any is. It's there only while rain is due, giving up its
        // place rather than saying there's none (VKCF3Z, 2026-11-04); the
        // days' hours still say each hour's chance.
        const next = hours.slice(0, hoursAhead);
        if (due(next))
            rows.push(holderRow({
                name: Words.name("weather:rain"),
                side: rainSaid(next),
                hue: "voice",
                glyph: Theme.glyph.rain,
                graph: spark(next)
            }, () => next.map(hour => hourThing(hour, dateOf(date)))));
        for (let i = 0; i < (daily.time ?? []).length; i++) {
            const day = daily.time[i];
            const side = `${degrees(daily.temperature_2m_min[i])} to `
                + `${degrees(daily.temperature_2m_max[i])} \u00b7 ${said(daily.weather_code[i])}`;
            const own = dayFigures(daily, i);
            const chance = (daily.precipitation_probability_max ?? [])[i];
            rows.push(thingRow({
                name: i === 0 ? Words.name("weather:today")
                    : i === 1 ? Words.name("weather:tomorrow") : weekday(day),
                side: side,
                note: known(chance) && chance >= possible ? words("dayRain",
                    { chance: share(chance) }) : "",
                glyph: glyph(daily.weather_code[i], true),
                thing: true,
                drawer: () => drawerOf(Moment.eyebrow(dateOf(day), dateOf(date)), side,
                    "weather:day", own)
            }, () => figureRows("weather:day", own).concat(hours.filter(hour => hour.date === day)
                .map(hour => hourThing(hour, dateOf(date))))));
        }
        return {
            headline: headline,
            rows: rows,
            note: note,
            // The weather is a thing with a drawer, as its neighbours in
            // Now are, saying it in brief (NowDrawers.js), made only as
            // it's shown.
            thing: true,
            drawer: () => weatherDrawer(forecast, next, note)
        };
    }

    // Weather's drawer: what it's like now, where, the next hours and the
    // rain when it's due, then each day as its row says it.
    function weatherDrawer(forecast, next, note) {
        const now = forecast.current;
        const daily = forecast.daily ?? {};
        const warmths = next.map(hour => hour.at.temperature_2m).filter(known);
        const wet = next.findIndex(hour => hour.chance >= likely);
        const dayName = i => i === 0 ? Words.name("weather:today")
            : i === 1 ? Words.name("weather:tomorrow")
            : Qt.formatDate(dateOf(daily.time[i]), "dddd");
        return NowDrawers.weather({
            sky: said(now.weather_code),
            warmth: degrees(now.temperature_2m) ?? "",
            feels: degrees(now.apparent_temperature),
            // The place as the footer says it, without who forecast it.
            place: note.split(" \u00b7 ")[0],
            hours: next.length,
            low: warmths.length === 0 ? null : degrees(Math.min(...warmths)),
            high: warmths.length === 0 ? null : degrees(Math.max(...warmths)),
            rainFrom: wet < 0 ? "" : wet === 0 ? "now" : next[wet].clock,
            rain: wet < 0 ? "" : rainSaid(next),
            days: (daily.time ?? []).map((day, i) => ({
                name: dayName(i),
                low: degrees(daily.temperature_2m_min[i]),
                high: degrees(daily.temperature_2m_max[i]),
                sky: said(daily.weather_code[i])
            })),
            said: Words.of("weather").said ?? {},
            names: { title: Words.name("weather"), place: Words.name("weather:place"),
                next: Words.name("weather:next"), rain: Words.name("weather:rain") }
        });
    }

    function waiting(entry, who) {
        if (entry.failed)
            return missed(`${who} didn't answer.`);
        return { headline: "\u2026", copy: "", rows: [], note: `asking ${who}`, asking: true };
    }

    // An answer that came to nothing, saying why in a sentence, once: the
    // launcher shows it as a row of its own, as quietly as nothing found.
    function missed(why) {
        return { headline: why, copy: "", rows: [], missed: true };
    }
}
