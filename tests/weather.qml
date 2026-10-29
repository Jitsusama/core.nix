// Drives the launcher's weather (modules/quickshell/launcher/Weather.qml)
// through Quickshell itself, with what Open-Meteo would say already kept,
// as if just asked, so nothing leaves the sandbox: "weather" answers what
// it's like now, its parts Now, Rain, Wind, Today, Tomorrow and the day
// after, all from the one request, which asks the hourly rain too; Rain
// says when it next rains, with a spark of the next twelve hours, and Return
// steps into them, an hour a row, each its chance, how much and a bar; the
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

    // What the forecast says, three days from midnight on Saturday 24
    // October 2026, its clock at `now`, with the hourly chances and amounts
    // given by hour of the first day and none after.
    function forecast(now, chances, amounts) {
        const times = [];
        const chance = [];
        const falls = [];
        for (let hour = 0; hour < 72; hour++) {
            const day = 24 + Math.floor(hour / 24);
            times.push(`2026-10-${day}T${String(hour % 24).padStart(2, "0")}:00`);
            chance.push(chances[hour] ?? 0);
            falls.push(amounts[hour] ?? 0);
        }
        return {
            current: { time: now, temperature_2m: 12.4, weather_code: 1, is_day: 1, wind_speed_10m: 17.6 },
            hourly: { time: times, precipitation_probability: chance, precipitation: falls },
            daily: {
                time: ["2026-10-24", "2026-10-25", "2026-10-26"],
                weather_code: [61, 3, 0],
                temperature_2m_max: [14.2, 9, 11],
                temperature_2m_min: [6, 3.4, 2]
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
            + "&current=temperature_2m,weather_code,is_day,wind_speed_10m"
            + "&hourly=precipitation_probability,precipitation"
            + "&daily=weather_code,temperature_2m_max,temperature_2m_min"
            + "&forecast_days=3&timezone=auto";
    }

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
        settings: ({ name: "weather", units: "metric", zone: "America/Toronto", place: "Lima",
            lookups: true })
    }

    // A place set, with no laptop found, ahead of the time zone.
    Launcher.Weather {
        id: settled

        laptop: null
        settings: ({ name: "weather", units: "metric", zone: "America/Toronto", place: "Lima",
            lookups: true })
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
        // Here, it rains from three; Tokyo stays dry; Paris is raining now;
        // Oslo only might.
        kept(where("Toronto"), place("Toronto", "Canada", 43.7, -79.42));
        kept(forecastUrl(43.7, -79.42), forecast("2026-10-24T13:45",
            { 13: 10, 14: 20, 15: 70, 16: 90, 17: 40 }, { 15: 1.2, 16: 3.4, 17: 0.25 }));
        kept(where("tokyo"), place("Tokyo", "Japan", 35.69, 139.69));
        kept(forecastUrl(35.69, 139.69), forecast("2026-10-24T22:45", {}, {}));
        kept(where("paris"), place("Paris", "France", 48.85, 2.35));
        kept(forecastUrl(48.85, 2.35), forecast("2026-10-24T09:15", { 9: 80 }, { 9: 2 }));
        kept(where("oslo"), place("Oslo", "Norway", 59.91, 10.75));
        kept(forecastUrl(59.91, 10.75), forecast("2026-10-24T09:15", { 12: 40 }, {}));
        const asked = Object.keys(weather.asked).length;

        const here = weather.answer("weather");
        expect("what it's like now", here.headline === "12\u00b0 \u00b7 mostly clear",
            here.headline);
        expect("the parts, flat, in order",
            names(here.rows) === "Now, Rain, Wind, Today, Tomorrow, Monday", names(here.rows));
        expect("where it is, and how that was found",
            here.note === "Toronto, from your time zone \u00b7 Open-Meteo", here.note);
        const [now, rain, wind, today, tomorrow] = here.rows;
        expect("now copies itself",
            now.side === "12\u00b0 \u00b7 mostly clear" && now.verb === "copy",
            `${now.side} ${now.verb}`);
        expect("the wind", wind.side === "18 km/h", wind.side);
        expect("today", today.side === "6\u00b0 to 14\u00b0 \u00b7 light rain", today.side);
        expect("tomorrow", tomorrow.side === "3\u00b0 to 9\u00b0 \u00b7 overcast", tomorrow.side);
        expect("the next rain", rain.side === "from 15:00, 70%", rain.side);
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
        expect("an hour's chance and amount", hours[2].side === "70%, 1.2 mm", hours[2].side);
        expect("a dry hour's", hours[0].side === "10%, 0 mm", hours[0].side);
        expect("an hour's bar",
            hours[3].graph.shape === "bar" && hours[3].graph.values.length === 1
                && hours[3].graph.values[0] === 0.9, JSON.stringify(hours[3].graph));
        expect("an hour is a figure, copying what it shows", hours[2].verb === "copy",
            hours[2].verb);

        // The rain asked for itself, by its one word.
        for (const typed of ["rain", "Rain", "rain?"]) {
            const said = weather.answer(typed);
            expect(`"${typed}" answers the next rain`,
                said?.headline === "rain from 15:00, 70%" && said.rows.length === 12
                    && said.graph?.shape === "spark", said?.headline);
        }
        const asRain = weather.answer("rain");
        expect("the rain's hours, a step in", names(asRain.rows) === names(hours), names(asRain.rows));
        expect("where the rain is", asRain.note === here.note, asRain.note);

        // Elsewhere, with its own clock and its own rain.
        const tokyo = weather.answer("weather in tokyo");
        expect("the time there", tokyo.rows[0].name === "Now There, 22:45 Saturday", tokyo.rows[0].name);
        expect("its own rain", tokyo.rows[1].side === "none in the next 12 hours", tokyo.rows[1].side);
        expect("where it is", tokyo.note === "Tokyo, Japan \u00b7 Open-Meteo", tokyo.note);
        expect("its hours run into tomorrow",
            tokyo.rows[1].verbs()[1].name === "23:00" && tokyo.rows[1].verbs()[2].name === "00:00",
            names(tokyo.rows[1].verbs()));
        const dry = weather.answer("rain in tokyo");
        expect("no rain there", dry.headline === "no rain in the next 12 hours", dry.headline);
        const wet = weather.answer("rain in paris?");
        expect("raining now", wet.headline === "rain now, 80%", wet.headline);
        const maybe = weather.answer("rain for oslo");
        expect("might", maybe.headline === "rain unlikely, at most 40%", maybe.headline);

        // The weather's one word asks it, here or anywhere.
        for (const typed of ["weather", "Weather", "weather tokyo", "weather for tokyo",
                "tokyo weather"]) {
            const said = weather.answer(typed);
            expect(`"${typed}" answers the weather`,
                said?.rows.length === 6 && said.rows[1].name === "Rain", said?.headline);
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
        kept(where("Lima"), place("Lima", "Peru", -12.05, -77.04), settled);
        kept(forecastUrl(-12.05, -77.04), forecast("2026-10-24T13:45", {}, {}), settled);
        const set = settled.answer("weather");
        expect("the place set", set.note === "Lima, from the place you set \u00b7 Open-Meteo",
            set.note);
        // Once its town is known, the laptop's here is said by it.
        travelling.laptop = Object.assign({}, travelling.laptop, { place: "Lighthouse, Ontario" });
        expect("the laptop's town",
            travelling.answer("weather").note
                === "Lighthouse, Ontario, from where the laptop is \u00b7 Open-Meteo",
            travelling.answer("weather").note);
    }
}
