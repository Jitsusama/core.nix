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
// The significant figures a distance or a speed is said to.
const FIGURES = 3;
// How near a fix is rounds up, so it never claims nearer than it knows,
// and drops its zeros (Y2YBTS's review of #2785); a speed is a measure,
// which keeps them, so its width holds as it changes, as design.md's Set,
// Not Styled has "0.500 A" (VKCF3Z, 2026-11-10).
const WITHIN = { up: true, zeros: false };
const MEASURED = { up: false, zeros: true };
const TEN = 10;
// The widest a fix may be for its zone to move the clock, where its zone
// holds round its circle and where nothing says, as clock-follows decides
// (clock-follows/src/main.rs's TRUSTED and NEAR).
const TRUSTED = 50_000;
const NEAR = 10_000;

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
    const near = figured(metres, WITHIN, told);
    if (near.value < KILOMETRE) {
        return said.metres.replace("{n}", near.text);
    }
    return said.kilometres.replace("{n}", figured(metres / KILOMETRE, WITHIN, told).text);
}

// A figure to three significant figures, rounding up where `how.up`
// says it mustn't claim less than it is, else to the nearest, and with
// its trailing zeros where `how.zeros` keeps them: "1.01", "26" or
// "26.0", "1,240". Past three whole digits the rest round to tens and
// hundreds, and the rounding is kept whole either side of the point, so
// a float never says 1,239.9999 for 1,240. Its value and its text.
function figured(value, how, told) {
    const whole = Math.max(1, Math.floor(Math.log10(Math.max(value, 1))) + 1);
    const decimals = FIGURES - whole;
    const round = how.up ? (scaled) => Math.ceil(scaled - 1e-9) : Math.round;
    const rounded = decimals >= 0
        ? round(value * TEN ** decimals) / TEN ** decimals
        : round(value / TEN ** -decimals) * TEN ** -decimals;
    const written = (String(rounded).split(".")[1] ?? "").length;
    // A zero has no significant digits, so keeps no zeros either: "0 km/h",
    // never a precision nothing gave (VKCF3Z, 2026-11-10).
    const kept = rounded === 0 ? 0
        : Math.max(0, how.zeros ? decimals : Math.min(decimals, written));
    return { value: rounded, text: told.number(rounded, kept) };
}


// Why the clock isn't following a place in another zone, the last item of
// Now's note, which gives way first: near a time zone's edge where its
// circle keeps another time, the fix's own width where it's too wide, and
// nothing where the clock would follow it, as while Put It Back holds the
// zone, which the drawer leads with (VKCF3Z, 2026-11-09).
function unfollowed(here, said, told) {
    if (here.zoneHolds === false) {
        return said.nearEdge;
    }
    const widest = here.zoneHolds === true ? TRUSTED : NEAR;
    if (!Number.isFinite(here.accuracy) || here.accuracy <= widest) {
        return "";
    }
    return said.foundWithin.replace("{within}", within(here.accuracy, said, told));
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

// What found it, as a plate's eyebrow says where then what, as WARP's
// "via WARP \u00b7 tunnel" does (VKCF3Z, 2026-11-09): "via Wi-Fi \u00b7
// location", "found \u00b7 location" where GeoClue named no source
// whereabouts knows, and "set by hand \u00b7 location" for Joel's own.
function eyebrow(here, said) {
    if (here.hand === true) {
        return said.viaHand;
    }
    return (here.source ? said[`via-${here.source}`] : null) ?? said.via;
}

// The figures only some sources give, as GPS does, and only while held:
// altitude to the metre and heading to the degree east of north, in
// GeoClue's own units, and speed in kilometres an hour, as a road says it,
// to three significant figures, from GeoClue's metres a second, so 4.26
// m/s reads "15.3 km/h" (VKCF3Z, 2026-11-10).
const MEASURES = [
    { id: "altitude", decimals: 0 },
    { id: "speed", scale: 3.6 },
    { id: "heading", decimals: 0 }
];

// A measure's figure: to its places where it keeps a fixed number, else
// to three significant figures, to the nearest, keeping its zeros, so
// 10 m/s reads "36.0 km/h", and none reads "0 km/h".
function figure(value, decimals, told) {
    if (decimals !== undefined) {
        return told.number(value, decimals);
    }
    return figured(value, MEASURED, told).text;
}

function held(value) {
    return Number.isFinite(value);
}

function measures(here, said, told) {
    return MEASURES.map(({ id, decimals, scale }) => ({
        id,
        value: held(here[id])
            ? said[id].replace("{n}", figure(here[id] * (scale ?? 1), decimals, told)) : ""
    }));
}

// What its source couldn't give of the three, in one sentence the plate's
// body says, naming the source as its eyebrow does: "Wi-Fi gives no
// altitude, speed or heading.", "GPS gave no speed or heading." Nothing
// when all three are held, nor for a place set by hand, which no source
// found (VKCF3Z, 2026-11-09). `said` and `tells` are the vocabulary's,
// each a sentence of its own for a source, as its words read whole.
function leftOut(here, said, tells) {
    if (here.hand === true) {
        return "";
    }
    const missing = MEASURES.filter(({ id }) => !held(here[id])).map(({ id }) => said[`no-${id}`]);
    if (missing.length === 0) {
        return "";
    }
    const last = missing.pop();
    const listed = missing.length === 0 ? last : `${missing.join(", ")} ${said.or} ${last}`;
    const sentence = (here.source ? tells[`leftOut-${here.source}`] : null) ?? tells.leftOut;
    return sentence.replace("{missing}", listed);
}

// Its plate's facts, from what moves most: when it was found, how near,
// where on a map, and how high, fast and which way where a source gives
// them, its country, then its zone, the identifier it is, last. The town
// is the row's side, so the plate never says it again. Whether the zone
// holds round the fix is left out: the note's "near a time zone's edge"
// says it where it matters (VKCF3Z, 2026-11-09).
function facts(here, now, said, told) {
    return [
        { id: "located", value: located(here.when, now, told) },
        { id: "accuracy", value: here.hand === true ? "" : within(here.accuracy, said, told) },
        { id: "coordinates", value: coordinates(here, said) },
        ...measures(here, said, told),
        { id: "country", value: here.country ?? "" },
        { id: "zone", value: here.zone ?? "", identifies: true }
    ].filter(fact => fact.value !== "");
}

// The facts the row's note doesn't already say, since its drawer, shown
// beside it, never says a thing twice (VKCF3Z, 2026-11-09): Accuracy goes
// from the plate while the note says how near it is, and stays while the
// note says the zone instead. Known by its id rather than its words, so a
// note that ever said a fact's value for another reason takes nothing.
function besideNote(plate, noteSaysHowNear) {
    return noteSaysHowNear ? plate.filter(fact => fact.id !== "accuracy") : plate;
}
