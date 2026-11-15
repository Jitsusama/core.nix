.pragma library
// What a share typed for the Brightness asks, where the Brightness
// served stands and how a served switch the : line takes stands, in
// plain JavaScript so a check runs it in Qt's own engine apart from
// drawing (tests/screens.qml), as the shell does. A screen's drawer, its
// settings, what its EDID says, the ways it turns off or on and what the
// : line's :setl types for them, power and unplugged included, are the
// windows daemon's to say (modules/windows: screens.rs and the files
// beside it).

// What the : line's :set brightness= and a typed "brightness 40" ask of
// the windows daemon: what a Brightness level it serves asks
// (light/brightness.rs), at the share typed, the first step kept lit,
// since none at all leaves the screen black; or null with no level
// served, so nothing is asked. `rows` are the rows served.
function brightening(rows, share) {
    for (const row of rows) {
        const level = (row?.acts ?? []).find(one => one.key === "displays:brightness:level");
        if (level?.ask) {
            return Object.assign({}, level.ask, { share: Math.max(1, share) });
        }
    }
    return null;
}

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

// The switch served among `rows` whose act is `act`, as the : line's
// :set nightlight and :set warmth take the Warmth's Night Light and
// Follow the Room (light/warmth.rs): how it stands, and what turning it
// `to` asks of the windows daemon, which is its Return's ask, since that
// always asks the other way, or null where it stands so already; or null
// while none is served, so the option isn't offered.
function switchOf(rows, act) {
    for (const row of rows) {
        const served = (row?.acts ?? []).find(one => one.key === act);
        if (served?.ask) {
            const on = row.held?.on === true;
            return { on, asked: to => to === on ? null : served.ask };
        }
    }
    return null;
}
