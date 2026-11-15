.pragma library
// How a screen stands from what niri says (`niri msg -j outputs`), which
// of the acts the daemon serves the : line's :setl power and unplugged
// ask, what a share typed for the Brightness asks and where the
// Brightness served stands, in plain JavaScript so a check runs it in
// Qt's own engine apart from drawing (tests/screens.qml), as the shell
// does. Its
// drawer, its settings, what its EDID says, the ways it turns off or on
// and what the : line's :setl types for them are the windows daemon's to
// say (modules/windows: screens.rs and the files beside it). niri says
// `powered_off` only where it darkens one screen by name
// (modules/niri/powers-off-each-output.patch).

// "off" when niri has taken it out of its outputs, as unplugging it
// would; "dark" when it's powered off by name, still docked with its hub
// powered; else "lit".
function standing(output) {
    if (output.logical === null) {
        return "off";
    }
    return output.powered_off === true ? "dark" : "lit";
}

// The act the : line does to a screen, as :setl nopower or :setl
// unplugged asks, of the acts its row serves (`acts`, each { key, ask }):
// power on is Turn It On and off Turn It Off; unplugged is the act that
// takes it out of niri's outputs, Unplug It where niri darkens each
// screen by name and an older niri's Turn It Off where it doesn't, and
// back is Turn It On only for one unplugged, never a dark one. Null where
// the screen stands so already or the act isn't served, as unplugging
// the last lit screen, so nothing else is done in its place. `asked` is
// the option and the state asked of it, { option, on }.
function wayTo(output, acts, asked) {
    const { option, on } = asked;
    if (option === "unplugged" && !on && standing(output) !== "off") {
        return null;
    }
    const lights = option === "power" ? on : !on;
    const found = option === "unplugged" && on
        ? one => one.ask?.do === "unplug" || one.ask?.do === "off"
        : one => one.key === `verb:display-${lights ? "on" : "off"}`;
    return acts.find(found) ?? null;
}

// What the : line says for a screen where it does nothing, by its words'
// key (words.json's displays said), or "" where a way is offered: what's
// true where it's already as asked, as Vim's :set says it, and a refused
// act's words where the way isn't offered, which is only ever the last
// lit screen (VKCF3Z, 2026-11-08). `asked` is as wayTo's.
function toldOf(output, acts, asked) {
    const { option, on } = asked;
    if (optionNow(output, option) === on) {
        if (option === "unplugged" && !on) {
            return "isPluggedIn";
        }
        return { lit: "isOn", dark: "isDark", off: "isUnplugged" }[standing(output)] ?? "";
    }
    if (wayTo(output, acts, asked) !== null) {
        return "";
    }
    // A screen that isn't lit always offers to come on, so only a way off
    // is ever refused.
    return option === "unplugged" ? "not-unplugged" : "not-off";
}

// What a screen's option says it is now: powered while lit, unplugged
// while out of niri's outputs.
function optionNow(output, option) {
    return option === "power" ? standing(output) === "lit" : standing(output) === "off";
}

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

// The Brightness served among `rows` (light/brightness.rs): the row's
// key of the screen whose place holds it, the share it read, or null
// while it's being read, so no old level is said as now, and what its
// side says: the share, nothing in its first second, then that it isn't
// read yet; or null while none is served.
const BRIGHTNESS = /^displays:[^:]+:brightness$/;
function brightnessOf(rows) {
    for (const row of rows) {
        if (row?.kind === "setting" && BRIGHTNESS.test(row.thing ?? "")) {
            const share = row.held?.share;
            return {
                screen: row.parent,
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
