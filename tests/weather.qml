// Drives the launcher's weather (modules/quickshell/launcher/Weather.qml)
// through Quickshell itself, with what Open-Meteo would say already kept,
// as if just asked, so nothing leaves the sandbox: "weather" answers what
// it's like now, its parts Conditions, Rain, then seven days from today, all from
// the one request, which asks every figure of now, each hour and each day;
// Conditions steps into every figure of this moment, and elsewhere Local Time
// follows it with the clock there; a day steps into its own
// figures, then its hours, sharing none of them; an hour steps into its
// figures, and is the same thing in Rain as in its day; Rain says when it
// next rains, with a spark of the next twelve hours, and Return steps into
// them, an hour a row, each its sky, its chance, how much and a bar; the
// rain's own words answer with the next rain, here or "in" a place, and
// say when it's raining now, might, or won't; the weather's one word asks
// it, here or anywhere, and an old word doesn't; and with lookups off it
// answers nothing.
// Here is where the laptop is once Location has said, its forecast asked
// by its position with no geocoder and said by its town, or by its
// position while the town is unknown, ahead of the place set and the time
// zone; then the place set, ahead of the time zone; and the footer says
// which.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import Quickshell
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    // What the geocoder says of a place.
    function place(name, country, latitude, longitude) {
        return { results: [{ name: name, country: country, latitude: latitude, longitude: longitude }] };
    }

    // What the forecast says, seven days from midnight on Saturday 24
    // October 2026, its clock at `now`, with the hourly chances and amounts
    // given by hour of the first day and none after, an hour raining
    // lightly where its chance is likely, and every other figure the same
    // each hour but its warmth.
    function forecast(now, chances, amounts) {
        const hourly = { time: [], precipitation_probability: [], precipitation: [],
            temperature_2m: [], apparent_temperature: [], relative_humidity_2m: [],
            dew_point_2m: [], weather_code: [], is_day: [], cloud_cover: [], wind_speed_10m: [],
            wind_direction_10m: [], wind_gusts_10m: [], uv_index: [], visibility: [],
            pressure_msl: [] };
        for (let hour = 0; hour < 168; hour++) {
            const day = 24 + Math.floor(hour / 24);
            const chance = chances[hour] ?? 0;
            hourly.time.push(`2026-10-${day}T${String(hour % 24).padStart(2, "0")}:00`);
            hourly.precipitation_probability.push(chance);
            hourly.precipitation.push(amounts[hour] ?? 0);
            hourly.temperature_2m.push(hour % 24 / 2);
            hourly.apparent_temperature.push(hour % 24 / 2 - 3);
            hourly.relative_humidity_2m.push(80);
            hourly.dew_point_2m.push(3);
            hourly.weather_code.push(chance >= 50 ? 61 : 2);
            hourly.is_day.push(hour % 24 >= 7 && hour % 24 < 18 ? 1 : 0);
            hourly.cloud_cover.push(60);
            hourly.wind_speed_10m.push(12);
            hourly.wind_direction_10m.push(180);
            hourly.wind_gusts_10m.push(25);
            hourly.uv_index.push(1.5);
            hourly.visibility.push(16000);
            hourly.pressure_msl.push(1009);
        }
        const week = values => values.concat(values.slice(-1).concat(Array(4).fill(values[2])))
            .slice(0, 7);
        return {
            current: { time: now, temperature_2m: 12.4, apparent_temperature: 9.6,
                relative_humidity_2m: 71, dew_point_2m: 7.3, precipitation: 0, weather_code: 1,
                is_day: 1, cloud_cover: 22, wind_speed_10m: 17.6, wind_direction_10m: 247.5,
                wind_gusts_10m: 33.1, uv_index: 3.4, visibility: 24140, pressure_msl: 1013.2 },
            current_units: { visibility: "m" },
            hourly: hourly,
            hourly_units: { visibility: "m" },
            daily: {
                time: ["2026-10-24", "2026-10-25", "2026-10-26", "2026-10-27", "2026-10-28",
                    "2026-10-29", "2026-10-30"],
                weather_code: week([61, 3, 0]),
                temperature_2m_max: week([14.2, 9, 11]),
                temperature_2m_min: week([6, 3.4, 2]),
                apparent_temperature_max: week([12, 6, 9]),
                apparent_temperature_min: week([2, 0, -1]),
                precipitation_sum: week([4.86, 0, 0]),
                precipitation_hours: week([3, 0, 0]),
                precipitation_probability_max: week([90, 10, 0]),
                wind_speed_10m_max: week([24, 15, 10]),
                wind_gusts_10m_max: week([48, 30, 20]),
                wind_direction_10m_dominant: week([200, 290, 10]),
                uv_index_max: week([2.6, 3, 4]),
                sunrise: week(["2026-10-24T07:39", "2026-10-25T07:40", "2026-10-26T07:41"]),
                sunset: week(["2026-10-24T18:14", "2026-10-25T18:12", "2026-10-26T18:11"]),
                daylight_duration: week([37500, 37380, 37260])
            }
        };
    }

    // Keeps what a URL would say to a provider, the first unless another
    // is named, as if it had just answered.
    function kept(url, value, provider) {
        (provider ?? weather).asked[url] = { pending: false, value: value, failed: false,
            at: Date.now() };
    }

    function forecastUrl(latitude, longitude) {
        return "https://api.open-meteo.com/v1/forecast"
            + `?latitude=${latitude}&longitude=${longitude}`
            + `&current=${instant}&hourly=${instant},precipitation_probability`
            + "&daily=weather_code,temperature_2m_max,temperature_2m_min,"
            + "apparent_temperature_max,apparent_temperature_min,precipitation_sum,"
            + "precipitation_hours,precipitation_probability_max,wind_speed_10m_max,"
            + "wind_gusts_10m_max,wind_direction_10m_dominant,uv_index_max,sunrise,sunset,"
            + "daylight_duration"
            + "&forecast_days=7&timezone=auto";
    }

    // Every figure of a moment, as now and each hour are asked for.
    readonly property string instant: "temperature_2m,apparent_temperature,"
        + "relative_humidity_2m,dew_point_2m,precipitation,weather_code,is_day,cloud_cover,"
        + "wind_speed_10m,wind_direction_10m,wind_gusts_10m,uv_index,visibility,pressure_msl"

    function where(name) {
        return `https://geocoding-api.open-meteo.com/v1/search?count=1&name=${name}`;
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    Launcher.Weather {
        id: weather

        settings: ({ name: "weather", units: "metric", zone: "America/Toronto", lookups: true })
    }

    // A laptop Location has found, ahead of a place set.
    Launcher.Weather {
        id: travelling

        laptop: ({ v: 2, latitude: 10.123, longitude: -20.456, accuracy: 3000,
            when: 1790000000, place: null, zone: null })
        settings: ({ name: "weather", units: "metric", zone: "America/Toronto",
            place: "Shoalmouth", lookups: true })
    }

    // A place set, with no laptop found, ahead of the time zone.
    Launcher.Weather {
        id: settled

        laptop: null
        settings: ({ name: "weather", units: "metric", zone: "America/Toronto",
            place: "Shoalmouth", lookups: true })
    }

    Launcher.Weather {
        id: offline

        settings: ({ name: "weather", units: "metric", zone: "America/Toronto", lookups: false })
    }

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
        // Here, it rains from three; Gullhaven stays dry; Brinemoor is
        // raining now; Driftholm only might. Each is invented, as its
        // country and its place in open sea are, so no case reads as
        // where someone is; here comes from the time zone, so it's the
        // zone's own town.
        kept(where("Toronto"), place("Toronto", "Canada", 43.7, -79.42));
        kept(forecastUrl(43.7, -79.42), forecast("2026-10-24T13:45",
            { 13: 10, 14: 20, 15: 70, 16: 90, 17: 40 }, { 15: 1.2, 16: 3.4, 17: 0.25 }));
        kept(where("gullhaven"), place("Gullhaven", "Velmora", 30.41, 171.23));
        kept(forecastUrl(30.41, 171.23), forecast("2026-10-24T22:45", {}, {}));
        kept(where("brinemoor"), place("Brinemoor", "Ostrandia", -38.12, -18.45));
        kept(forecastUrl(-38.12, -18.45), forecast("2026-10-24T09:15", { 9: 80 }, { 9: 2 }));
        kept(where("driftholm"), place("Driftholm", "Norvessa", 47.33, -38.66));
        kept(forecastUrl(47.33, -38.66), forecast("2026-10-24T09:15", { 12: 40 }, {}));
        const asked = Object.keys(weather.asked).length;

        const here = weather.answer("weather");
        expect("what it's like now", here.headline === "12\u00b0 \u00b7 mostly clear",
            here.headline);
        expect("the parts, flat, in order",
            names(here.rows) === "Conditions, Rain, Today, Tomorrow, Monday, Tuesday, Wednesday, "
                + "Thursday, Friday", names(here.rows));
        expect("where it is, and how that was found",
            here.note === "Toronto, from your time zone \u00b7 Open-Meteo", here.note);
        const [now, rain, today, tomorrow] = here.rows;
        // Conditions is a thing, every figure of this moment a step in, each
        // copied, and its drawer saying them all.
        const sides = rows => rows.map(row => `${row.name} ${row.side}`).join(", ");
        expect("now steps into what it's like",
            now.side === "12\u00b0 \u00b7 mostly clear" && now.verb === "more" && now.thing,
            `${now.side} ${now.verb}`);
        expect("every figure of now", sides(now.verbs()) === "Temperature 12\u00b0, "
            + "Feels Like 10\u00b0, Humidity 71%, Dew Point 7\u00b0, Precipitation 0 mm, "
            + "Wind 18 km/h from the WSW, Gusts 33 km/h, Cloud Cover 22%, "
            + "UV Index 3.4, moderate, Visibility 24.1 km, Pressure 1013 hPa",
            sides(now.verbs()));
        expect("a figure of now copies itself", now.verbs()[0].verb === "copy",
            now.verbs()[0].verb);
        expect("now's drawer says every figure", now.drawer().facts.length === 11,
            JSON.stringify(now.drawer()));
        // The weather itself is a thing whose drawer says it in brief, as
        // Now's other rows' do: now and the rain to come, where, the next
        // twelve hours, the rain, then each of the seven days.
        const brief = here.thing === true ? here.drawer() : null;
        const briefFacts = brief ? brief.facts.map(fact => `${fact.label}: ${fact.value}`) : [];
        expect("the weather's drawer says now and the rain", brief?.body
            === "Mostly clear at 12\u00b0, feeling like 10\u00b0; rain likely from 15:00.",
            JSON.stringify(brief));
        expect("the weather's drawer says where, the hours, the rain and the days",
            briefFacts.slice(0, 4).join(", ") === "Forecast For: Toronto, from your time zone, "
                + "Next 12 Hours: 0\u00b0 to 12\u00b0, Rain: from 15:00 \u00b7 70%, "
                + "Today: 6\u00b0 to 14\u00b0 \u00b7 light rain" && briefFacts.length === 10,
            briefFacts.join(", "));
        expect("today", today.side === "6\u00b0 to 14\u00b0 \u00b7 light rain"
            && today.note === "90% chance", `${today.side} ${today.note}`);
        expect("tomorrow", tomorrow.side === "3\u00b0 to 9\u00b0 \u00b7 overcast"
            && tomorrow.note === "", `${tomorrow.side} ${tomorrow.note}`);
        // A day steps into its own figures, then its hours: today's from
        // this one, tomorrow's every one. None of a day's figures is one
        // its hours say.
        const ownOf = day => day.verbs().filter(row => !/^\d\d:\d\d$/.test(row.name));
        const hoursOf = day => day.verbs().filter(row => /^\d\d:\d\d$/.test(row.name));
        expect("a day's own figures", sides(ownOf(today)) === "High 14\u00b0, Low 6\u00b0, "
            + "Feels Like Range 2\u00b0 to 12\u00b0, Highest Chance 90%, "
            + "Total Precipitation 4.9 mm, Strongest Wind 24 km/h from the SSW, "
            + "Strongest Gust 48 km/h, Highest UV Index 2.6, low, Sunrise 07:39, "
            + "Sunset 18:14, Daylight 10 h 25 min", sides(ownOf(today)));
        expect("a day's figures lead its hours",
            today.verbs().findIndex(row => /^\d\d:\d\d$/.test(row.name))
                === ownOf(today).length, names(today.verbs()));
        expect("today's hours, from this one", hoursOf(today).length === 11
            && hoursOf(today)[0].name === "13:00" && hoursOf(today)[10].name === "23:00",
            names(hoursOf(today)));
        expect("tomorrow's hours, every one", hoursOf(tomorrow).length === 24
            && hoursOf(tomorrow)[0].name === "00:00", names(hoursOf(tomorrow)));
        const hourNames = hoursOf(today)[0].verbs().map(row => row.name);
        expect("a day says none of its hours' figures",
            !ownOf(today).some(row => hourNames.includes(row.name)),
            ownOf(today).filter(row => hourNames.includes(row.name)).map(row => row.name)
                .join(", "));
        // An hour is a thing, its sky beside it and its rain under it,
        // every figure a step in and in its drawer.
        const three = hoursOf(today)[2];
        expect("an hour's sky and rain", three.side === "8\u00b0 \u00b7 light rain"
            && three.note === "70% \u00b7 1.2 mm", `${three.side} ${three.note}`);
        expect("an hour steps into its figures", three.verb === "more" && three.thing
            && sides(three.verbs()) === "Temperature 8\u00b0, Feels Like 5\u00b0, "
            + "Humidity 80%, Dew Point 3\u00b0, Chance of Precipitation 70%, "
            + "Precipitation 1.2 mm, Wind 12 km/h from the S, Gusts 25 km/h, "
            + "Cloud Cover 60%, UV Index 1.5, low, Visibility 16 km, Pressure 1009 hPa",
            sides(three.verbs()));
        expect("an hour's drawer says when",
            three.drawer().eyebrow === "Saturday 24 October \u00b7 15:00"
                && three.drawer().facts.length === 12, JSON.stringify(three.drawer()));
        expect("the next rain", rain.side === "from 15:00 \u00b7 70%", rain.side);
        expect("a spark of the next twelve hours",
            rain.graph.shape === "spark" && rain.graph.values.length === 12
                && rain.graph.values[0] === 0.1 && rain.graph.values[3] === 0.9,
            JSON.stringify(rain.graph));
        // Return copies when it next rains, as asked alone, and the right
        // arrow steps into the hours, the same at the top or a step down
        // (Joel, 2026-10-26: a row showing a value copies it, everywhere).
        expect("rain copies", rain.verb === "copy" && rain.figure === true, rain.verb);
        const hours = rain.verbs();
        expect("a step down, the same hours", names(rain.targets()) === names(hours),
            names(rain.targets()));
        expect("the next twelve hours, from this one",
            hours.length === 12 && hours[0].name === "13:00" && hours[11].name === "00:00",
            names(hours));
        expect("an hour's chance and amount", hours[2].note === "70% \u00b7 1.2 mm",
            hours[2].note);
        expect("a dry hour's", hours[0].note === "10% \u00b7 0 mm", hours[0].note);
        expect("an hour's bar",
            hours[3].graph.shape === "bar" && hours[3].graph.values.length === 1
                && hours[3].graph.values[0] === 0.9, JSON.stringify(hours[3].graph));
        expect("an hour in the rain is the hour in its day",
            sides(hours[2].verbs()) === sides(three.verbs()) && hours[2].side === three.side,
            sides(hours[2].verbs()));

        // The rain asked for itself, by its one word.
        for (const typed of ["rain", "Rain", "rain?"]) {
            const said = weather.answer(typed);
            expect(`"${typed}" answers the next rain`,
                said?.headline === "rain from 15:00 \u00b7 70%" && said.rows.length === 12
                    && said.graph?.shape === "spark", said?.headline);
        }
        const asRain = weather.answer("rain");
        expect("the rain's hours, a step in", names(asRain.rows) === names(hours), names(asRain.rows));
        expect("where the rain is", asRain.note === here.note, asRain.note);

        // Elsewhere, with its own clock and its own rain.
        const there = weather.answer("weather in gullhaven");
        expect("conditions there, as here", there.rows[0].name === "Conditions",
            there.rows[0].name);
        expect("the time there, a fact of its own", there.rows[1].name === "Local Time"
            && there.rows[1].side === "22:45 Saturday" && there.rows[1].verb === "copy",
            `${there.rows[1].name} ${there.rows[1].side} ${there.rows[1].verb}`);
        // No rain due there, so Rain gives up its place rather than say
        // there's none (VKCF3Z, 2026-11-04).
        expect("no rain, no Rain", !there.rows.some(row => row.name === "Rain"),
            names(there.rows));
        expect("where it is", there.note === "Gullhaven, Velmora \u00b7 Open-Meteo", there.note);
        expect("its hours run into tomorrow",
            hoursOf(there.rows[2]).pop().name === "23:00"
                && hoursOf(there.rows[3])[0].name === "00:00",
            `${names(hoursOf(there.rows[2]))} | ${names(hoursOf(there.rows[3]))}`);
        const dry = weather.answer("rain in gullhaven");
        expect("no rain there", dry.headline === "no rain in the next 12 hours", dry.headline);
        const wet = weather.answer("rain in brinemoor?");
        expect("raining now", wet.headline === "rain now \u00b7 80%", wet.headline);
        const maybe = weather.answer("rain for driftholm");
        expect("might", maybe.headline === "rain unlikely, at most 40%", maybe.headline);
        // Rain that only might come isn't due, so it has no row either.
        expect("rain that might, no Rain",
            !weather.answer("weather for driftholm").rows.some(row => row.name === "Rain"),
            names(weather.answer("weather for driftholm").rows));

        // The weather's one word asks it, here or anywhere.
        for (const typed of ["weather", "Weather"]) {
            const said = weather.answer(typed);
            expect(`"${typed}" answers the weather`,
                said?.rows.length === 9 && said.rows[1].name === "Rain", said?.headline);
        }
        for (const typed of ["weather gullhaven", "weather for gullhaven", "gullhaven weather"]) {
            const said = weather.answer(typed);
            expect(`"${typed}" answers the weather`,
                said?.rows.length === 9 && said.rows[1].name === "Local Time"
                    && said.rows[2].name === "Today", said?.headline);
        }
        // And nothing else is read as rain, nor a place looked up by it.
        for (const typed of ["rainbow", "rain tomorrow", "train", "brain", "umbrella", "forecast"])
            expect(`"${typed}" isn't asked`, weather.answer(typed) === null, typed);
        expect("one request a place, and no more", Object.keys(weather.asked).length === asked,
            Object.keys(weather.asked).join(" "));
        expect("the rain's words start a question",
            weather.questions.includes("rain") && weather.questions.includes("weather"),
            weather.questions.join(", "));
        expect("with lookups off, nothing", offline.answer("rain") === null
            && offline.answer("weather") === null, "it answered");

        // Where the laptop is comes first, asked by its position to a
        // hundredth of a degree, with no geocoder, and said by it.
        const far = forecastUrl(10.12, -20.46);
        kept(far, forecast("2026-10-24T13:45", {}, {}), travelling);
        const away = travelling.answer("weather");
        const position = "10.12\u00b0 N 20.46\u00b0 W";
        expect("the laptop's here",
            away.note === `${position}, from where the laptop is \u00b7 Open-Meteo`, away.note);
        expect("its forecast alone is asked", Object.keys(travelling.asked).join(" ") === far,
            Object.keys(travelling.asked).join(" "));
        expect("the rain where the laptop is", travelling.answer("rain").note === away.note,
            travelling.answer("rain").note);
        // With no laptop found, the place set, then the time zone.
        kept(where("Shoalmouth"), place("Shoalmouth", "Quellara", -14.32, -101.87), settled);
        kept(forecastUrl(-14.32, -101.87), forecast("2026-10-24T13:45", {}, {}), settled);
        const set = settled.answer("weather");
        expect("the place set",
            set.note === "Shoalmouth, from the place you set \u00b7 Open-Meteo",
            set.note);
        // Once its town is known, the laptop's here is said by it.
        travelling.laptop = Object.assign({}, travelling.laptop, { place: "Kelpwater, Westmark" });
        expect("the laptop's town",
            travelling.answer("weather").note
                === "Kelpwater, Westmark, from where the laptop is \u00b7 Open-Meteo",
            travelling.answer("weather").note);

        // Now asks the weather at every opening, so asked again with the
        // forecast it holds, it makes nothing anew: Joel's openings each
        // made the weather's 8 rows again, though nothing had changed
        // (2026-11-12). A forecast heard anew answers anew.
        const held = weather.answer("weather");
        const made = weather.tally.rows;
        const again = weather.answer("weather");
        expect("asked again with what it holds, it makes no row",
            again === held && weather.tally.rows === made,
            `${weather.tally.rows - made} rows made again`);
        expect("the rain is its own answer", weather.answer("rain") !== held,
            "the rain answered the weather");
        kept(forecastUrl(43.7, -79.42), forecast("2026-10-24T14:45", {}, {}));
        const heard = weather.answer("weather");
        expect("a forecast heard anew answers anew",
            heard !== held && weather.tally.rows > made, "it said what it held");
    }
}
