.pragma library
.import "Numbers.js" as Numbers
// What a screen's drawer says of it from what niri says (`niri msg -j
// outputs`), in plain JavaScript so a check runs it in Qt's own engine
// (tests/plate.qml), as the shell does: whether the mode it runs is the
// one it prefers or one of niri's own making, its refresh exactly as niri
// has it where its row rounds it, whether it can and does refresh
// variably, the logical size and place it takes on the desktop, how
// it's turned, the modes it offers and the one it prefers, how fine its
// pixels are and its serial. Its size, whole refresh and scale are its
// row's to say, so they aren't said again (VKCF3Z, 2026-11-07). It also
// says how each screen stands and the ways that turn it off or on, apart
// from drawing (tests/screens.qml), where `each` says whether the running
// niri powers one output off by name (modules/niri/
// powers-off-each-output.patch), the only niri that says `powered_off`.
// Displays.qml reads niri and lays the plate, the light and what the
// EDID says around these.

// Two items of one value, as a figure's runs are set apart.
const APART = " \u00b7 ";
const TIMES = " \u00d7 ";
const MILLIMETRES_AN_INCH = 25.4;
const MILLIHERTZ = 1000;
const TRAILING_ZEROS = /0+$/;

// A rate as niri gives it, in millihertz, said in hertz with every digit
// it has, since it names a mode rather than measuring one: 120001 is
// "120.001 Hz", 60000 "60 Hz", 59940 "59.94 Hz".
function hertz(millihertz) {
    const whole = Math.floor(millihertz / MILLIHERTZ);
    const rest = millihertz % MILLIHERTZ;
    const decimals = String(rest).padStart(3, "0").replace(TRAILING_ZEROS, "");
    return rest === 0 ? `${whole} Hz` : `${whole}.${decimals} Hz`;
}

function size(width, height) {
    return `${width}${TIMES}${height}`;
}

// A scale as the theme says one, a multiple: 2 is "2\u00d7", 1.25
// "1.25\u00d7".
function scale(multiple) {
    return `${Number(multiple.toFixed(2))}\u00d7`;
}

// How fine its pixels are, in pixels an inch across its width, or "" where
// niri knows no size for it, as a projector's EDID says none. It's worked
// out from the screen's measured size, so it's a measure, to three
// significant digits with its zeros kept, grouped and pointed as the
// launcher's settings say: "244 ppi", "96.0 ppi" (VKCF3Z, 2026-11-08).
function density(pixels, millimetres, group, point) {
    return pixels > 0 && millimetres > 0
        ? `${Numbers.significant(pixels / (millimetres / MILLIMETRES_AN_INCH), group, point)} ppi`
        : "";
}

function modeOf(output) {
    return output.current_mode === null || output.current_mode === undefined ? null
        : output.modes[output.current_mode] ?? null;
}

// The mode it prefers, the first niri marks so, or null.
function preferredOf(output) {
    return (output.modes ?? []).find(mode => mode.is_preferred) ?? null;
}

// Its facts, each { label, value }, in the drawer's order (VKCF3Z,
// 2026-11-07): what it runs first, then where it sits, then what it
// offers and is. `words` names each (`named(id)`, displays:<id>), says the
// vocabulary's words (`said(id, key)`) and how it's turned
// (`turned(transform)`); `settings` hold the launcher's `group` and
// `point`, which its measures take. A screen that's off runs nothing and
// sits nowhere, so it says only what it offers and is.
function facts(output, words, settings) {
    const out = [];
    const add = (id, value) => {
        if (value !== "") {
            out.push({ label: words.named(id), value });
        }
    };
    const now = modeOf(output);
    const logical = output.logical ?? null;
    if (now !== null && logical !== null) {
        add("mode", [now.is_preferred ? words.said("modes", "preferred") : "",
            output.is_custom_mode ? words.said("modes", "custom") : ""]
            .filter(part => part !== "").join(APART));
        add("refresh", now.refresh_rate % MILLIHERTZ === 0 ? "" : hertz(now.refresh_rate));
        add("variable", output.vrr_supported
            ? [words.said("variable", "supported"),
                words.said("variable", output.vrr_enabled ? "on" : "off")].join(APART)
            : words.said("variable", "unsupported"));
        add("logical", size(logical.width, logical.height));
        add("position", `${logical.x}, ${logical.y}`);
        add("rotation", words.turned(logical.transform));
    }
    const preferred = preferredOf(output);
    const offered = (output.modes ?? []).length;
    add("modes", offered === 0 ? "" : [String(offered), preferred === null ? ""
        : words.said("modes", "preferred-mode").replace("{mode}",
            words.said("modes", "at").replace("{size}", size(preferred.width, preferred.height))
                .replace("{rate}", hertz(preferred.refresh_rate)))]
        .filter(part => part !== "").join(APART));
    const [wide] = output.physical_size ?? [0, 0];
    const across = now ?? preferred;
    add("density", across === null ? ""
        : density(across.width, wide, settings?.group, settings?.point));
    add("serial", String(output.serial ?? ""));
    return out;
}

// Its size as niri has it from the EDID, where desk-facts doesn't say one:
// "300 \u00d7 190 mm", or "".
function physical(output) {
    const [wide, high] = output.physical_size ?? [0, 0];
    return wide > 0 && high > 0 ? `${size(wide, high)} mm` : "";
}

// "off" when niri has taken it out of its outputs, as unplugging it
// would; "dark" when it's powered off by name, still docked with its hub
// powered; else "lit".
function standing(output) {
    if (output.logical === null) {
        return "off";
    }
    return output.powered_off === true ? "dark" : "lit";
}

// Whether no other screen is lit, so this one is all that shows the
// launcher, and once it's dark a key lights every screen again.
function lastLit(output, outputs) {
    return !outputs.some(other => other.name !== output.name && standing(other) === "lit");
}

function outputAsked(output, action) {
    return { Output: { output: output.name, action } };
}

function actAsked(action, output) {
    return { Action: { [action]: { output: output.name } } };
}

// What a screen's step-in offers to turn it off or on, in order, each as
// { word, asks, note, waits }: the word it's named by, what niri is
// asked, the note it says beside (a key "wakes" the last one lit, and
// unplugging "moves" its windows), and whether it waits for the key that
// asked to come up, since niri lights every screen at any key once all
// are off by name. Darkening comes before unplugging, the bigger step,
// and the last lit screen is never unplugged, or nothing would show the
// launcher; an older niri only unplugs, as its Turn It Off.
function ways(output, outputs, each) {
    const stands = standing(output);
    if (stands === "off") {
        return [{ word: "on", asks: outputAsked(output, "On"), note: "", waits: false }];
    }
    if (stands === "dark") {
        return [{ word: "on", asks: actAsked("PowerOnMonitor", output), note: "", waits: false }];
    }
    const last = lastLit(output, outputs);
    const offered = [];
    if (each) {
        offered.push({
            word: "off",
            asks: actAsked("PowerOffMonitor", output),
            note: last ? "wakes" : "",
            waits: last
        });
    }
    if (!last) {
        offered.push({
            word: each ? "unplug" : "off",
            asks: outputAsked(output, "Off"),
            note: each ? "moves" : "",
            waits: false
        });
    }
    return offered;
}

// The way the : line turns a screen, as :setl nopower or :setl unplugged
// asks, of those `ways` offers: power on is "on" and off "off";
// unplugged is "unplug" where niri darkens each screen by name, an older
// niri's "off" where it doesn't, and back "on" only for one unplugged,
// never a dark one. Null where the screen stands so already or the way
// isn't offered, as unplugging the last lit screen, so nothing else is
// done in its place. `asked` is the option and the state asked of it,
// { option, on }.
function wayTo(output, outputs, each, asked) {
    const { option, on } = asked;
    if (option === "unplugged" && !on && standing(output) !== "off") {
        return null;
    }
    const unplugging = each ? "unplug" : "off";
    const word = option === "power" ? (on ? "on" : "off") : on ? unplugging : "on";
    return ways(output, outputs, each).find(way => way.word === word) ?? null;
}

// What the : line says for a screen where it does nothing, by its words'
// key (words.json's displays said), or "" where a way is offered: what's
// true where it's already as asked, as Vim's :set says it, and a refused
// act's words where the way isn't offered, which is only ever the last
// lit screen (VKCF3Z, 2026-11-08). `asked` is as wayTo's.
function toldOf(output, outputs, each, asked) {
    const { option, on } = asked;
    if (optionNow(output, option) === on) {
        if (option === "unplugged" && !on) {
            return "isPluggedIn";
        }
        return { lit: "isOn", dark: "isDark", off: "isUnplugged" }[standing(output)] ?? "";
    }
    if (wayTo(output, outputs, each, asked) !== null) {
        return "";
    }
    // A screen that isn't lit always offers to come on, so only a way off
    // is ever refused.
    return option === "unplugged" ? "notUnplugged" : "notOff";
}

// What a screen's option says it is now: powered while lit, unplugged
// while out of niri's outputs.
function optionNow(output, option) {
    return option === "power" ? standing(output) === "lit" : standing(output) === "off";
}
