// Where the laptop is, as Now's Location says it (launcher/Now.qml), from
// what whereabouts writes (Location.qml): its position to no more places
// than it knows, how near it is, when it was found, and its plate's facts
// (VKCF3Z's ruling, 2026-11-08). Apart from the view, so tests/answers.qml
// holds it in Qt's own engine. Every word comes from the vocabulary, passed
// in as `said`; how long ago and which day, as the launcher says them
// everywhere (Ago.js, When.js), come in as `told.ago` and `told.day`, and a
// time by the clock as `told.clock`.
.pragma library

const KILOMETRE = 1000;
const SECOND = 1000;
const DAY = 86_400_000;
// Decimals a position claims for how near it is: four, about 11 m, under
// 100 m; three under a kilometre; two under ten; else one, about 11 km.
const METRES = 100;
const KILOMETRES = 10 * KILOMETRE;
const FOUR = 4;
const THREE = 3;
const TWO = 2;
const ONE = 1;

// The decimals a position may claim for its accuracy, in metres.
function places(accuracy) {
    if (accuracy < METRES) {
        return FOUR;
    }
    if (accuracy < KILOMETRE) {
        return THREE;
    }
    return accuracy < KILOMETRES ? TWO : ONE;
}

// A position as degrees and hemispheres, one figure: "43.7° N · 79.4° W".
function coordinates(here) {
    const decimals = places(here.accuracy);
    const degrees = (value, positive, negative) =>
        `${Math.abs(value).toFixed(decimals)}\u00b0 ${value < 0 ? negative : positive}`;
    return `${degrees(here.latitude, "N", "S")} \u00b7 ${degrees(here.longitude, "E", "W")}`;
}

// How near the laptop is to where it's said to be: "within 80 m", "within
// 26 km".
function within(accuracy, said) {
    return accuracy < KILOMETRE
        ? said.metres.replace("{n}", String(Math.ceil(accuracy)))
        : said.kilometres.replace("{n}", String(Math.ceil(accuracy / KILOMETRE)));
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
        { id: "accuracy", value: here.hand === true ? "" : within(here.accuracy, said) },
        { id: "coordinates", value: coordinates(here) },
        { id: "country", value: here.country ?? "" },
        { id: "zone", value: here.zone ?? "", identifier: true }
    ].filter(fact => fact.value !== "");
}
