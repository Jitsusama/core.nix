.pragma library
// Where the Brightness served stands, in plain JavaScript so a check runs
// it in Qt's own engine apart from drawing (tests/screens.qml), as the
// shell does. A screen's drawer, its settings, what its EDID says, the
// ways it turns off or on, what the : line's :setl types for them, power
// and unplugged included, and the place's own :set line, brightness and
// the Warmth's switches among it, are the windows daemon's to say
// (modules/windows: screens.rs and the files beside it).

// The Brightness served among `rows` (light/brightness.rs): the share
// it read, or null while it's being read, so no old level is said as
// now, and what its side says: the share, nothing in its first second,
// then that it isn't read yet; or null while none is served.
const BRIGHTNESS = /^displays:[^:]+:brightness$/;
function brightnessOf(rows) {
    for (const row of rows) {
        if (row?.kind === "setting" && BRIGHTNESS.test(row.thing ?? "")) {
            const share = row.held?.share;
            return {
                share: typeof share === "number" ? share : null,
                said: (row.side ?? []).map(each => each.said).join(" ")
            };
        }
    }
    return null;
}
