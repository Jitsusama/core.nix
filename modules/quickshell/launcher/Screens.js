.pragma library
.import "Numbers.js" as Numbers
// What a screen's settings and acts say of it from what niri says (`niri
// msg -j outputs`), in plain JavaScript so a check runs it in Qt's own
// engine (tests/plate.qml), as the shell does: a rate with every digit
// niri has, a scale as a multiple, and the white point and primaries from
// the figures desk-facts gives. Its drawer is the windows daemon's to say
// (modules/windows, screens.rs). It also says how each screen stands and
// the ways that turn it off or on, apart from drawing (tests/screens.qml),
// where `each` says whether the running niri powers one output off by
// name (modules/niri/powers-off-each-output.patch), the only niri that
// says `powered_off`. Its settings' choices of mode and turn
// are said here too, each by its value, the mode it prefers noted so,
// and what each takes typed on the : line's :setl, as scale=1.5.
// What niri answers a change is said in ScreenAnswers.js.

// Two items of one value, as a figure's runs are set apart.
const APART = " \u00b7 ";
const TIMES = " \u00d7 ";
const MILLIHERTZ = 1000;
const TRAILING_ZEROS = /0+$/;
// A colour temperature's unit.
const KELVIN = " K";
// The places a chromaticity's coordinate is said to: ten bits a coordinate
// hold one to about a thousandth, so "0.060" says all an EDID knows.
const COORDINATE_PLACES = 3;
// The primaries desk-facts gives figures for, in its order.
const PRIMARIES = ["red", "green", "blue"];
// What sets desk-facts' figures apart.
const BETWEEN_FIGURES = /\s+/;

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

function modeOf(output) {
    return output.current_mode === null || output.current_mode === undefined ? null
        : output.modes[output.current_mode] ?? null;
}

// niri's eight transforms, as Rotation offers them: the four turns, then
// the four mirrored (VKCF3Z, 2026-11-09).
const TRANSFORMS = ["Normal", "90", "180", "270",
    "Flipped", "Flipped90", "Flipped180", "Flipped270"];

function transforms() {
    return TRANSFORMS.slice();
}

// What a mode's choice notes after its value: "preferred" where the EDID
// prefers it, "custom" where it's a custom mode niri runs, in
// displays:modes' words (`words.said(id, key)`).
function noted(preferred, custom, words) {
    return [preferred ? words.said("modes", "preferred") : "",
        custom ? words.said("modes", "custom") : ""].filter(part => part !== "").join(APART);
}

// Refresh's choices: each rate the mode it runs is offered at, at its
// size, each { mode, value, note, chosen }, said by its value alone with
// every digit niri gives, so 59.94 Hz and 60 Hz are two (design.md: a
// choice is its value, its setting named in the path above).
function refreshes(output, words) {
    const now = modeOf(output);
    if (now === null) {
        return [];
    }
    return output.modes.filter(mode => mode.width === now.width && mode.height === now.height)
        .map(mode => ({
            mode,
            value: hertz(mode.refresh_rate),
            note: noted(mode.is_preferred === true,
                mode === now && output.is_custom_mode === true, words),
            chosen: mode === now
        }));
}

// Resolution's choices: each size once, in niri's order, as Refresh's
// are, preferred where any of its modes is.
function sizes(output, words) {
    const now = modeOf(output);
    const found = [];
    for (const mode of output.modes ?? []) {
        const at = found.find(each => each.mode.width === mode.width
            && each.mode.height === mode.height);
        if (at === undefined) {
            found.push({ mode, preferred: mode.is_preferred === true });
        } else if (mode.is_preferred === true) {
            at.preferred = true;
        }
    }
    return found.map(({ mode, preferred }) => {
        const chosen = now !== null && mode.width === now.width && mode.height === now.height;
        return {
            mode,
            value: size(mode.width, mode.height),
            note: noted(preferred, chosen && output.is_custom_mode === true, words),
            chosen
        };
    });
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
    return option === "unplugged" ? "not-unplugged" : "not-off";
}

// What a screen's option says it is now: powered while lit, unplugged
// while out of niri's outputs.
function optionNow(output, option) {
    return option === "power" ? standing(output) === "lit" : standing(output) === "off";
}

// The scales Scale offers, as multiples.
const SCALES = [1, 1.25, 1.5, 1.75, 2, 2.5, 3];

function scales() {
    return SCALES.slice();
}

// A rate's figure as it's typed, its hertz with every digit and no unit:
// 59951 is "59.951".
function hertzTyped(millihertz) {
    return hertz(millihertz).slice(0, -" Hz".length);
}

// What's typed after a figure's own unit, a multiple's "\u00d7" or a
// rate's "Hz", so either way is taken.
const SCALE_UNIT = /\s*[x\u00d7]$/i;
const HERTZ_UNIT = /\s*hz$/i;
const SIZE_TYPED = /^([0-9]+)\s*[x\u00d7]\s*([0-9]+)$/i;
const WHOLE = /^[0-9]+$/;

// The settings a lit screen takes on the : line's :setl, by its row's
// id (VKCF3Z, 2026-11-09): its mode's where niri says which it runs.
function lineSettings(output) {
    return modeOf(output) === null ? ["scale", "rotation"]
        : ["scale", "refresh", "resolution", "rotation"];
}

// A setting's values as :setl takes them, each { value, note, asks }:
// scale=1.5, refresh=59.951 with every digit, resolution=2560x1440 and
// rotation= in the vocabulary's typed words (displays:rotation's
// typed-Normal is "upright"), never niri's names, since nothing shows
// those. Each notes its choice's words, so completing says what it is,
// and asks niri what its choice's row does.
// Two modes of one size and rate are one value, said once.
function lineValues(output, setting, words) {
    const dotted = (value, note) => [value, note].filter(part => part !== "").join(APART);
    const once = (each, at, all) => all.findIndex(other => other.value === each.value) === at;
    if (setting === "scale") {
        return SCALES.map(multiple => ({
            value: String(Number(multiple.toFixed(2))),
            note: scale(multiple),
            asks: { Scale: { scale: { Specific: multiple } } }
        }));
    }
    if (setting === "refresh") {
        return refreshes(output, words).map(each => ({
            value: hertzTyped(each.mode.refresh_rate),
            note: dotted(each.value, each.note),
            asks: { Mode: { mode: { Specific: { width: each.mode.width,
                height: each.mode.height, refresh: each.mode.refresh_rate / MILLIHERTZ } } } }
        })).filter(once);
    }
    if (setting === "resolution") {
        return sizes(output, words).map(each => ({
            value: `${each.mode.width}x${each.mode.height}`,
            note: dotted(each.value, each.note),
            asks: { Mode: { mode: { Specific: { width: each.mode.width,
                height: each.mode.height, refresh: null } } } }
        })).filter(once);
    }
    return TRANSFORMS.map(transform => ({
        value: words.said("rotation", `typed-${transform}`),
        note: words.turned(transform),
        asks: { Transform: { transform } }
    }));
}

// Where a setting stands, as :setl scale? says it, in its typed words.
function lineNow(output, setting, words) {
    const now = modeOf(output);
    if (setting === "scale") {
        return String(Number(output.logical.scale.toFixed(2)));
    }
    if (setting === "refresh") {
        return now === null ? null : hertzTyped(now.refresh_rate);
    }
    if (setting === "resolution") {
        return now === null ? null : `${now.width}x${now.height}`;
    }
    return words.said("rotation", `typed-${output.logical.transform}`);
}

// The value a setting's typed words name, or null: a multiple or a rate
// with its unit or without, a size by x or \u00d7, a turn by its typed
// word in any case, and a whole rate the one mode at its size that rounds
// to it, so refresh=60 sets 59.951 where none runs at 60 (VKCF3Z,
// 2026-11-09).
function lineFound(values, setting, typed) {
    const given = typed.trim();
    if (setting === "scale") {
        const multiple = Number(given.replace(SCALE_UNIT, ""));
        return values.find(each => Math.abs(Number(each.value) - multiple) < 0.001) ?? null;
    }
    if (setting === "refresh") {
        const rate = given.replace(HERTZ_UNIT, "");
        const exact = values.find(each => each.value === rate);
        if (exact !== undefined || !WHOLE.test(rate)) {
            return exact ?? null;
        }
        const near = values.filter(each => Math.round(Number(each.value)) === Number(rate));
        return near.length === 1 ? near[0] : null;
    }
    if (setting === "resolution") {
        const found = given.match(SIZE_TYPED);
        return found === null ? null
            : values.find(each => each.value === `${found[1]}x${found[2]}`) ?? null;
    }
    return values.find(each => each.value === given.toLowerCase()) ?? null;
}

// A value typed as its setting says one, for the words refusing it: 1.3
// as "1.3\u00d7", 75 as "75 Hz", a size as "2560 \u00d7 1440".
function typedSaid(setting, typed) {
    const given = typed.trim();
    const multiple = Number(given.replace(SCALE_UNIT, ""));
    const sized = given.match(SIZE_TYPED);
    if (setting === "scale" && given !== "" && Number.isFinite(multiple)) {
        return scale(multiple);
    }
    if (setting === "refresh" && given !== "") {
        return `${given.replace(HERTZ_UNIT, "")} Hz`;
    }
    return setting === "resolution" && sized !== null ? size(sized[1], sized[2]) : given;
}

// What a setting's value typed on the line asks: { asks }, niri's action,
// or { refused }, the words saying it wasn't set and what the screen,
// named as its row is, offers instead, in the foot's form: "1.3\u00d7 not
// set \u00b7 HP offers 1\u00d7 to 3\u00d7" (VKCF3Z, 2026-11-09). A
// setting of figures says its least to its most, whatever order niri
// lists its modes in; Rotation, whose eight have no order between, says
// each; and nothing after = says none was given. `asked` is what the line
// gave, as { setting, typed }.
function lineSet(output, asked, words, named) {
    const { setting, typed } = asked;
    const values = lineValues(output, setting, words);
    const found = lineFound(values, setting, typed);
    if (found !== null) {
        return { asks: found.asks };
    }
    // A figure's size: a multiple's or a rate's own, a resolution's area.
    const sized = each => each.value.split("x")
        .reduce((product, side) => product * Number(side), 1);
    const said = (setting === "rotation" ? values
        : values.slice().sort((one, other) => sized(one) - sized(other)))
        .map(each => setting === "rotation" ? each.value : each.note.split(APART)[0]);
    const offers = setting === "rotation" || said.length < 2 ? said.join(", ")
        : words.said("", "offers-range").replace("{first}", said[0])
            .replace("{last}", said[said.length - 1]);
    const refusal = typed.trim() === "" ? words.said("", "none-given")
        : words.said("", "not-offered").replace("{value}", typedSaid(setting, typed));
    return {
        refused: refusal.replace("{setting}", setting).replace("{screen}", named)
            .replace("{offers}", offers)
    };
}
