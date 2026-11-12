.pragma library
// How a screen stands from what niri says (`niri msg -j outputs`), and
// which of the acts the daemon serves the : line's :setl power and
// unplugged ask, in plain JavaScript so a check runs it in Qt's own
// engine apart from drawing (tests/screens.qml), as the shell does. Its
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
