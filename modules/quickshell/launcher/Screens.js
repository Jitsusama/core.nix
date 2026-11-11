.pragma library
.import "Numbers.js" as Numbers
// What a screen's acts and colours say of it from what niri says (`niri
// msg -j outputs`), in plain JavaScript so a check runs it in Qt's own
// engine (tests/plate.qml), as the shell does: the white point and
// primaries from the figures desk-facts gives. Its drawer, its settings
// and what the : line's :setl types for them are the windows daemon's
// to say (modules/windows: screens.rs, screens/settings.rs and
// screens/line.rs). It also says how each screen stands and which of
// the acts the daemon serves the : line's :setl power and unplugged
// ask, apart from drawing (tests/screens.qml); the ways a screen turns
// off or on are the daemon's (screens/acts.rs). niri says `powered_off`
// only where it darkens one screen by name
// (modules/niri/powers-off-each-output.patch).

// Two items of one value, as a figure's runs are set apart.
const APART = " \u00b7 ";
// A colour temperature's unit.
const KELVIN = " K";
// The places a chromaticity's coordinate is said to: ten bits a coordinate
// hold one to about a thousandth, so "0.060" says all an EDID knows.
const COORDINATE_PLACES = 3;
// The primaries desk-facts gives figures for, in its order.
const PRIMARIES = ["red", "green", "blue"];
// What sets desk-facts' figures apart.
const BETWEEN_FIGURES = /\s+/;

// The figures desk-facts writes for the launcher to say, as numbers, or
// null where there aren't `count` of them.
function figuresOf(given, count) {
    const all = String(given ?? "").trim().split(BETWEEN_FIGURES).map(Number);
    return all.length === count && all.every(Number.isFinite) ? all : null;
}

// A point on the CIE 1931 chart in a part's words, each coordinate named
// so nobody has to remember the order (VKCF3Z, 2026-11-09), to three
// places, pointed as the launcher's settings say: "x 0.313 y 0.329".
function xy(said, x, y, point) {
    const placed = value => value.toFixed(COORDINATE_PLACES).replace(".", point ?? ".");
    return said.replace("{x}", placed(x)).replace("{y}", placed(y));
}

// A screen's white from its EDID's figures, "6462 0.3135 0.3291" from
// desk-facts: its colour temperature by the launcher's one rule for a
// measure, grouped and pointed as its settings say, then its point,
// "6,460 K \u00b7 x 0.313 y 0.329"; "" where the figures aren't whole.
function whitePoint(given, words, settings) {
    const figures = figuresOf(given, 3);
    if (figures === null) {
        return "";
    }
    const [kelvin, x, y] = figures;
    return Numbers.significant(kelvin, settings?.group, settings?.point) + KELVIN + APART
        + xy(words.said("white", "point"), x, y, settings?.point);
}

// A screen's primaries from its EDID's figures, each point's x and y in
// turn, each named: "red x 0.640 y 0.330", then green and blue, a dot
// apart; "" where the figures aren't whole.
function primaries(given, words, settings) {
    const figures = figuresOf(given, PRIMARIES.length * 2);
    if (figures === null) {
        return "";
    }
    return PRIMARIES.map((colour, at) => xy(words.said("primaries", colour),
        figures[at * 2], figures[at * 2 + 1], settings?.point)).join(APART);
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
