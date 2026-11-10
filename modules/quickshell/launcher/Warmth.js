// The built-in screen's Warmth as Screens says it (launcher/Displays.qml),
// from what the backlight program says of it, as the machine daemon
// serves it (`warmth:built-in`, modules/machine's steered.rs): the
// warmth now, whether it follows the room and a sensor sees colour, the
// room's white, and the night light, on or off, its white and the night's
// sunset and sunrise. Its note says what holds the warmth, its figures are
// what it steps into and its plate shows (VKCF3Z's ruling, 2026-11-08).
// Apart from the view, so tests/answers.qml holds it in Qt's own engine.
// Every word comes from the vocabulary, passed in as `said`; a time as the
// clock says it, and a kelvin with its thousands grouped, come in as
// `told.clock` and `told.kelvin`.
.pragma library

// What the machine daemon's records say of the warmth, or null where
// nothing warms the screen, as when the backlight program has stopped.
function of(records) {
    const said = records.find(record => record?.kind === "warmth");
    if (typeof said?.kelvin !== "number" || typeof said.follows_the_room !== "boolean") {
        return null;
    }
    const known = value => (typeof value === "number" ? value : null);
    return {
        kelvin: said.kelvin,
        followsTheRoom: said.follows_the_room,
        seesColour: said.sees_colour === true,
        roomWhite: known(said.room_white_kelvin),
        nightLight: said.night_light === true,
        nightWhite: known(said.night_white_kelvin),
        sunset: known(said.sunset_ms),
        sunrise: known(said.sunrise_ms)
    };
}

// Whether the night under way holds the moment.
function dark(warmth, now) {
    return warmth.sunset !== null && warmth.sunrise !== null && warmth.sunset <= now
        && now < warmth.sunrise;
}

// What holds the warmth now, in a few words: the night light until
// sunrise, or from sunset, or waiting for where Joel is; else the room, or
// the screen's own white.
function note(warmth, now, said, told) {
    if (warmth.nightLight && warmth.sunset === null) {
        return said.waits;
    }
    if (warmth.nightLight && dark(warmth, now)) {
        return said.until.replace("{time}", told.clock(warmth.sunrise));
    }
    if (warmth.nightLight) {
        return said.from.replace("{time}", told.clock(warmth.sunset));
    }
    return warmth.followsTheRoom && warmth.seesColour ? said.following : said.own;
}

// A time and, when it isn't today's, which day it's on.
function day(at, now, said, told) {
    const midnight = moment => new Date(moment).setHours(0, 0, 0, 0);
    const days = Math.round((midnight(at) - midnight(now)) / 86_400_000);
    const which = days === 1 ? said.tomorrow : days === -1 ? said.yesterday : "";
    return [told.clock(at), which].filter(part => part !== "").join(" \u00b7 ");
}

// Its figures, from what moves most: the room's white where a sensor sees
// it, the night's sunset and sunrise, whichever comes next first, so by
// night the sunrise leads (VKCF3Z, 2026-11-08), then the night white; each
// only where it's known.
function figures(warmth, now, said, told) {
    const sun = [
        { id: "sunset", side: warmth.sunset === null ? "" : day(warmth.sunset, now, said, told) },
        { id: "sunrise", side: warmth.sunrise === null ? ""
            : day(warmth.sunrise, now, said, told) }
    ];
    return [
        { id: "room", side: warmth.seesColour && warmth.roomWhite !== null
            ? told.kelvin(warmth.roomWhite) : "" }
    ].concat(dark(warmth, now) ? sun.reverse() : sun, [
        { id: "night", side: warmth.nightWhite === null ? "" : told.kelvin(warmth.nightWhite) }
    ]).filter(figure => figure.side !== "");
}
