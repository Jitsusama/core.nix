.pragma library
// What a screen's drawer says of it from what niri says (`niri msg -j
// outputs`), in plain JavaScript so a check runs it in Qt's own engine
// (tests/plate.qml), as the shell does: whether the mode it runs is the
// one it prefers or one of niri's own making, its refresh exactly as niri
// has it where its row rounds it, whether it can and does refresh
// variably, the logical size and place it takes on the desktop, how
// it's turned, the modes it offers and the one it prefers, how fine its
// pixels are and its serial. Its size, whole refresh and scale are its
// row's to say, so they aren't said again (VKCF3Z, 2026-11-07).
// Displays.qml reads niri and lays the plate, the light and what the
// EDID says around these.

// Two items of one value, as a figure's runs are set apart.
const APART = " \u00b7 ";
const TIMES = " \u00d7 ";
const MILLIMETRES_AN_INCH = 25.4;
const MILLIHERTZ = 1000;
// Measured figures keep three significant digits (VKCF3Z, 2026-11-07).
const SIGNIFICANT = 3;
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

// A measured figure to three significant digits, as a whole number past
// a thousand: 243.84 is "244", 96 "96", 12.345 "12.3".
function significant(value) {
    return String(Number(value.toPrecision(SIGNIFICANT)));
}

// How fine its pixels are, in pixels an inch across its width, or "" where
// niri knows no size for it, as a projector's EDID says none.
function density(pixels, millimetres) {
    return pixels > 0 && millimetres > 0
        ? `${significant(pixels / (millimetres / MILLIMETRES_AN_INCH))} ppi` : "";
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
// (`turned(transform)`). A screen that's off runs nothing and sits
// nowhere, so it says only what it offers and is.
function facts(output, words) {
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
    add("density", across === null ? "" : density(across.width, wide));
    add("serial", String(output.serial ?? ""));
    return out;
}

// Its size as niri has it from the EDID, where desk-facts doesn't say one:
// "300 \u00d7 190 mm", or "".
function physical(output) {
    const [wide, high] = output.physical_size ?? [0, 0];
    return wide > 0 && high > 0 ? `${size(wide, high)} mm` : "";
}
