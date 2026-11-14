// The built-in screen's warmth as Screens holds it (launcher/Displays.qml),
// from what the backlight program says of it, as the machine daemon
// serves it (`warmth:built-in`, modules/machine's steered.rs): the
// warmth now, whether it follows the room and a sensor sees colour, the
// room's white, and the night light, on or off, its white and the night's
// sunset and sunrise. The : line's :set nightlight and :set warmth read
// it; the Warmth's rows are the windows daemon's (modules/windows,
// light/warmth.rs). Apart from the view, so tests/answers.qml holds it in
// Qt's own engine.
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
