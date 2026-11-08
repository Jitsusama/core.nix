// Where the laptop is, as Now's Location says it (launcher/Now.qml), from
// what whereabouts writes (Location.qml): its position to no more places
// than it knows, how near it is, when it was found, and its plate's facts
// (VKCF3Z's ruling, 2026-11-08). Apart from the view, so tests/answers.qml
// holds it in Qt's own engine. Every word comes from the vocabulary, passed
// in as `said`; how long ago and which day, as the launcher says them
// everywhere (Ago.js, When.js), come in as `told.ago` and `told.day`, a
// time by the clock as `told.clock`, and a number in the settings' marks
// as `told.number(value, decimals)`.
.pragma library

const KILOMETRE = 1000;
const SECOND = 1000;
const DAY = 86_400_000;
// A degree's decimal places each a tenth of the last, from about 111 km
// for none: a position claims the nearest order of magnitude to how near
// it is, switching where its accuracy is a third of the way between
// them, at about 35 m, 350 m, 3.5 km and 35 km.
const BANDS = [35, 350, 3500, 35_000];
const MOST = BANDS.length;
// The significant figures a distance is said to, rounded up.
const FIGURES = 3;
const TEN = 10;

// The decimals a position may claim for its accuracy, in metres: four
// under 35 m, none from 35 km.
function places(accuracy) {
    const passed = BANDS.filter(band => accuracy >= band).length;
    return MOST - passed;
}

// A position as degrees and hemispheres, one figure: "12.3° N · 98.8° W",
// the hemispheres' letters `said`'s. A degree that rounds to none has no
// hemisphere to say.
function coordinates(here, said) {
    const decimals = places(here.accuracy);
    const degrees = (value, positive, negative) => {
        const shown = Math.abs(value).toFixed(decimals);
        const side = Number(shown) === 0 ? "" : value < 0 ? ` ${negative}` : ` ${positive}`;
        return `${shown}\u00b0${side}`;
    };
    return `${degrees(here.latitude, said.north, said.south)} \u00b7 `
        + `${degrees(here.longitude, said.east, said.west)}`;
}

// How near the laptop is to where it's said to be, to three significant
// figures, rounded up, so it never claims nearer than it knows, in
// whichever of metres and kilometres keeps it under a thousand: "within
// 80 m", "within 1.01 km", "within 26 km", "within 1,240 km".
function within(metres, said, told) {
    const figured = (value) => {
        const whole = Math.max(1, Math.floor(Math.log10(Math.max(value, 1))) + 1);
        // Past three whole digits, the rest round up to tens and hundreds.
        const decimals = FIGURES - whole;
        // Kept whole either side of the point, so a float never says
        // 1,239.9999 for 1,240.
        const up = decimals >= 0
            ? Math.ceil(value * TEN ** decimals - 1e-9) / TEN ** decimals
            : Math.ceil(value / TEN ** -decimals - 1e-9) * TEN ** -decimals;
        // Trailing zeros say nothing more: 26.0 is 26.
        const kept = Math.max(0, Math.min(decimals, (String(up).split(".")[1] ?? "").length));
        return { value: up, text: told.number(up, kept) };
    };
    const near = figured(metres);
    if (near.value < KILOMETRE) {
        return said.metres.replace("{n}", near.text);
    }
    return said.kilometres.replace("{n}", figured(metres / KILOMETRE).text);
}


// When it was found, `when` in seconds since 1970: how long ago within a
// day, else its day and time, as the launcher says a day ("Saturday 06:46",
// "28 Oct 09:30").
function located(when, now, told) {
    const at = when * SECOND;
    if (now - at < DAY) {
        return told.ago(at, now);
    }
    return [told.day(at, now), told.clock(at)].filter(part => part !== "").join(" ");
}

// Its plate's facts, from what moves most: when it was found, how near,
// where on a map, its country, then its zone, the identifier it is, last.
// The town is the row's side, so the plate never says it again.
function facts(here, now, said, told) {
    return [
        { id: "located", value: located(here.when, now, told) },
        { id: "accuracy", value: here.hand === true ? "" : within(here.accuracy, said, told) },
        { id: "coordinates", value: coordinates(here, said) },
        { id: "country", value: here.country ?? "" },
        { id: "zone", value: here.zone ?? "", identifies: true }
    ].filter(fact => fact.value !== "");
}
