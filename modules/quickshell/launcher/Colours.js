// A colour in its other forms, as the launcher answers it: "#ff6347",
// "rgb(255 99 71)", "hsl(9.1 100% 63.9%)", "oklch(69.6% 0.196 32.3)" or
// "colour accent" for one of the theme's own. It's the one file outside the
// theme that names colours, because colours are what it's asked about.
// Plain JavaScript, so tests/answers.qml runs it in Qt's own engine.
.pragma library

// Degrees in a turn and in half of one; hue's six sectors of sixty degrees,
// green's and blue's starting at the second and the fourth; a channel's top
// value; a percentage's whole; the channels before an alpha, and the places
// it's written to; and the hex base.
const TURN = 360;
const HALF_TURN = 180;
const SECTORS = 6;
const SECTOR = 60;
const GREEN_SECTOR = 2;
const BLUE_SECTOR = 4;
const CHANNEL_TOP = 255;
const PERCENT = 100;
const CHANNELS = 3;
const ALPHA_PLACES = 3;
const HEX = 16;
// Oklab's cone responses are cube roots, cubed back; 100% of an oklch()
// chroma, by CSS; less chroma than the least is a grey, whose hue is none;
// and a pigment less than the least ΔE away is the colour itself.
const CUBE = 3;
const FULL_CHROMA = 0.4;
const LEAST_CHROMA = 0.0005;
const LEAST_DELTA_E = 0.05;
// Past this, no pigment is near enough to be worth naming.
const near = 10;
// sRGB's transfer function (IEC 61966-2-1): a line below its knee, a power above.
const srgb = {
    knee: 0.040_45,
    linearKnee: 0.003_130_8,
    slope: 12.92,
    offset: 0.055,
    scale: 1.055,
    power: 2.4,
};
// Björn Ottosson's Oklab matrices, by row, each three weights summed: from
// linear sRGB to cone responses (LMS), their cube roots to Oklab, and back.
const RGB_TO_L = { x: 0.412_221_470_8, y: 0.536_332_536_3, z: 0.051_445_992_9 };
const RGB_TO_M = { x: 0.211_903_498_2, y: 0.680_699_545_1, z: 0.107_396_956_6 };
const RGB_TO_S = { x: 0.088_302_461_9, y: 0.281_718_837_6, z: 0.629_978_700_5 };
const LMS_TO_L = { x: 0.210_454_255_3, y: 0.793_617_785_0, z: -0.004_072_046_8 };
const LMS_TO_A = { x: 1.977_998_495_1, y: -2.428_592_205_0, z: 0.450_593_709_9 };
const LMS_TO_B = { x: 0.025_904_037_1, y: 0.782_771_766_2, z: -0.808_675_766_0 };
const LAB_TO_L = { x: 1, y: 0.396_337_777_4, z: 0.215_803_757_3 };
const LAB_TO_M = { x: 1, y: -0.105_561_345_8, z: -0.063_854_172_8 };
const LAB_TO_S = { x: 1, y: -0.089_484_177_5, z: -1.291_485_548_0 };
const LMS_TO_RED = { x: 4.076_741_662_1, y: -3.307_711_591_3, z: 0.230_969_929_2 };
const LMS_TO_GREEN = { x: -1.268_438_004_6, y: 2.609_757_401_1, z: -0.341_319_396_5 };
const LMS_TO_BLUE = { x: -0.004_196_086_3, y: -0.703_418_614_7, z: 1.707_614_701_0 };
// A hex colour and its digits in pairs; a colour function, its parts, what's between
// them, and the "a" of rgba() and hsla(), which mean the same; a number as CSS writes
// it; and a pigment asked for by name, the gaps in its name, and its underscores.
const HEX_COLOUR = /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/;
const DIGIT_PAIRS = /../g;
const CALLED = /^(rgba?|hsla?|oklch)\s*\(?\s*([^()]*?)\s*\)?$/;
const BETWEEN_PARTS = /[\s,/]+/;
const TRAILING_A = /a$/;
const CSS_NUMBER = /^(-?\d*\.?\d+)(%|deg)?$/;
const ASKED = /^colou?r\s+(.+)$/i;
const GAPS = /\s+/g;
const UNDERSCORES = /_/g;

// A row of a matrix's weights summed over three values.
function weighed(row, x, y, z) {
    return row.x * x + row.y * y + row.z * z;
}

// Rounded to `places` without trailing zeros, and never "-0".
function figure(value, places) {
    const rounded = Number(value.toFixed(places));
    return String(rounded === 0 ? 0 : rounded);
}

// An angle, where 359.96 rounds round to 0.
function angle(degrees) {
    return figure(Number(degrees.toFixed(1)) % TURN, 1);
}

function clamp(value) {
    return Math.min(1, Math.max(0, value));
}

function linear(channel) {
    return channel <= srgb.knee ? channel / srgb.slope
        : ((channel + srgb.offset) / srgb.scale) ** srgb.power;
}

function gamma(channel) {
    return channel <= srgb.linearKnee ? srgb.slope * channel
        : srgb.scale * channel ** (1 / srgb.power) - srgb.offset;
}

// Björn Ottosson's Oklab, from sRGB and back, each channel from 0 to 1.
function oklab([r, g, b]) {
    const [lr, lg, lb] = [linear(r), linear(g), linear(b)];
    const l = Math.cbrt(weighed(RGB_TO_L, lr, lg, lb));
    const m = Math.cbrt(weighed(RGB_TO_M, lr, lg, lb));
    const s = Math.cbrt(weighed(RGB_TO_S, lr, lg, lb));
    return [weighed(LMS_TO_L, l, m, s), weighed(LMS_TO_A, l, m, s), weighed(LMS_TO_B, l, m, s)];
}

function fromOklab([lightness, a, b]) {
    const l = weighed(LAB_TO_L, lightness, a, b) ** CUBE;
    const m = weighed(LAB_TO_M, lightness, a, b) ** CUBE;
    const s = weighed(LAB_TO_S, lightness, a, b) ** CUBE;
    return [weighed(LMS_TO_RED, l, m, s), weighed(LMS_TO_GREEN, l, m, s),
        weighed(LMS_TO_BLUE, l, m, s)].map(channel => clamp(gamma(channel)));
}

function hsl([r, g, b]) {
    const high = Math.max(r, g, b);
    const low = Math.min(r, g, b);
    const lightness = (high + low) / 2;
    const span = high - low;
    if (span === 0) {
        return [0, 0, lightness];
    }
    const saturation = span / (1 - Math.abs(2 * lightness - 1));
    // Where in hue's six sectors it is, by which channel is highest.
    let hue = (r - g) / span + BLUE_SECTOR;
    if (high === r) {
        hue = ((g - b) / span + SECTORS) % SECTORS;
    } else if (high === g) {
        hue = (b - r) / span + GREEN_SECTOR;
    }
    return [hue * SECTOR, saturation, lightness];
}

function fromHsl([hue, saturation, lightness]) {
    const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
    const sector = ((hue % TURN) + TURN) % TURN / SECTOR;
    const x = chroma * (1 - Math.abs(sector % 2 - 1));
    const base = lightness - chroma / 2;
    // Each sector's channels, before the lightness is added to them.
    const sectors = [[chroma, x, 0], [x, chroma, 0], [0, chroma, x], [0, x, chroma],
        [x, 0, chroma], [chroma, 0, x]];
    const [r, g, b] = sectors[Math.floor(sector)] ?? sectors[SECTORS - 1];
    return [r + base, g + base, b + base].map(clamp);
}

function hex(colour) {
    const values = colour.alpha < 1 ? colour.rgb.concat([colour.alpha]) : colour.rgb;
    return `#${values.map(value => Math.round(value * CHANNEL_TOP).toString(HEX)
        .padStart(2, "0")).join("")}`;
}

function alpha(colour) {
    return colour.alpha < 1 ? ` / ${figure(colour.alpha, ALPHA_PLACES)}` : "";
}

const forms = {
    hex,
    rgb: colour => `rgb(${colour.rgb.map(c => Math.round(c * CHANNEL_TOP)).join(" ")}${
        alpha(colour)})`,
    hsl: colour => {
        const [h, s, l] = hsl(colour.rgb);
        return `hsl(${angle(h)} ${figure(s * PERCENT, 1)}% ${figure(l * PERCENT, 1)}%${
            alpha(colour)})`;
    },
    oklch: colour => {
        const [l, a, b] = oklab(colour.rgb);
        const chroma = Math.hypot(a, b);
        const hue = chroma < LEAST_CHROMA ? "none"
            : angle((Math.atan2(b, a) * HALF_TURN / Math.PI + TURN) % TURN);
        return `oklch(${figure(l * PERCENT, 1)}% ${figure(chroma, ALPHA_PLACES)} ${hue}${
            alpha(colour)})`;
    }
};

// A number as CSS writes it: plain, or a percentage of `whole`.
function amount(text, whole) {
    const found = CSS_NUMBER.exec(text);
    if (!found) {
        return Number.NaN;
    }
    const value = Number.parseFloat(found[1]);
    return found[2] === "%" ? value / PERCENT * whole : value;
}

function opacity(text) {
    const value = text === undefined ? 1 : amount(text, 1);
    return value >= 0 && value <= 1 ? value : Number.NaN;
}

// The channels, from 0 to 1, of rgb(), hsl() and oklch() as their parts
// say, or null when they aren't a colour.
const channels = {
    rgb: parts => {
        const rgb = parts.slice(0, CHANNELS).map(part => amount(part, CHANNEL_TOP));
        return rgb.some(v => !(v >= 0 && v <= CHANNEL_TOP)) ? null : rgb.map(v => v / CHANNEL_TOP);
    },
    hsl: parts => {
        const [h, s, l] = [amount(parts[0], TURN), amount(parts[1], PERCENT),
            amount(parts[2], PERCENT)];
        if ([h, s, l].some(Number.isNaN) || s < 0 || s > PERCENT || l < 0 || l > PERCENT) {
            return null;
        }
        return fromHsl([h, s / PERCENT, l / PERCENT]);
    },
    oklch: parts => {
        const [l, c, h] = [amount(parts[0], 1), amount(parts[1], FULL_CHROMA),
            parts[2] === "none" ? 0 : amount(parts[2], TURN)];
        if ([l, c, h].some(Number.isNaN) || l < 0 || l > 1 || c < 0) {
            return null;
        }
        const hue = h * Math.PI / HALF_TURN;
        return fromOklab([l, c * Math.cos(hue), c * Math.sin(hue)]);
    }
};

// What's typed, read as a colour and the form it was written in, or null.
function read(text) {
    const typed = text.trim().toLowerCase();
    const digits = HEX_COLOUR.exec(typed);
    if (digits) {
        // A short colour has a digit for each channel and the alpha, if any.
        const short = digits[1].length <= CHANNELS + 1;
        const pairs = short ? digits[1].split("").map(d => d + d) : digits[1].match(DIGIT_PAIRS);
        const values = pairs.map(pair => Number.parseInt(pair, HEX) / CHANNEL_TOP);
        return { form: "hex", rgb: values.slice(0, CHANNELS), alpha: values[CHANNELS] ?? 1 };
    }
    const called = CALLED.exec(typed);
    if (!called) {
        return null;
    }
    const parts = called[2].split(BETWEEN_PARTS).filter(part => part !== "");
    if (parts.length < CHANNELS || parts.length > CHANNELS + 1) {
        return null;
    }
    const kind = called[1].replace(TRAILING_A, "");
    const shade = opacity(parts[CHANNELS]);
    const rgb = channels[kind](parts);
    return rgb === null || Number.isNaN(shade) ? null : { form: kind, rgb, alpha: shade };
}

// The theme's pigments, from `palette` (name to "#rrggbb") and `roles`
// (role to the same), each with the roles that wear it.
function pigments(palette, roles) {
    const found = [];
    for (const name of Object.keys(palette ?? {})) {
        const colour = read(palette[name]);
        if (colour) {
            const worn = Object.keys(roles ?? {})
                .filter(role => String(roles[role]).toLowerCase() === palette[name].toLowerCase());
            found.push({ name, colour, lab: oklab(colour.rgb), roles: worn });
        }
    }
    return found;
}

// The pigment nearest a colour, and how far (ΔE in Oklab, times 100), or
// null when none is near.
function nearest(colour, known) {
    const lab = oklab(colour.rgb);
    let best = null;
    for (const pigment of known) {
        const apart = PERCENT * Math.hypot(lab[0] - pigment.lab[0], lab[1] - pigment.lab[1],
            lab[2] - pigment.lab[2]);
        if (!best || apart < best.apart) {
            best = { pigment, apart };
        }
    }
    return best && best.apart <= near ? best : null;
}

// The answer to what's typed, given the theme's `palette` and `roles`, or
// null when it isn't a colour. A row that shows its colour carries it as
// `swatch`, in Qt's own "#aarrggbb", which puts the alpha first.
function answer(text, palette, roles) {
    const known = pigments(palette, roles);
    let colour = read(text);
    const asked = ASKED.exec(text.trim());
    if (!colour && asked) {
        const name = asked[1].toLowerCase().replace(GAPS, "_");
        const pigment = known.find(p => p.name === name || p.roles.indexOf(name) >= 0);
        colour = pigment ? { form: "name", rgb: pigment.colour.rgb, alpha: 1 } : null;
    }
    if (!colour) {
        return null;
    }
    const leading = colour.form === "hex" ? "rgb" : "hex";
    const headline = forms[leading](colour);
    const swatch = c => `#${[c.alpha].concat(c.rgb)
        .map(channel => Math.round(channel * CHANNEL_TOP).toString(HEX).padStart(2, "0"))
        .join("")}`;
    const rows = Object.keys(forms)
        .filter(form => form !== colour.form && form !== leading)
        .map((form, i) => ({ text: forms[form](colour), side: "", copy: forms[form](colour),
            swatch: i === 0 ? swatch(colour) : null }));
    const close = nearest(colour, known);
    if (close) {
        // Named first, capitalised as every row is, the roles that wear it
        // beside it, and how far it is from what was asked in the note.
        const apart = close.apart < LEAST_DELTA_E ? "" : `\u0394E ${figure(close.apart, 1)}`;
        const named = close.pigment.name.replace(UNDERSCORES, " ");
        rows.push({
            text: named.charAt(0).toUpperCase() + named.slice(1),
            side: close.pigment.roles.join(" \u00b7 "),
            note: apart,
            copy: hex(close.pigment.colour),
            swatch: swatch(close.pigment.colour)
        });
    }
    return { headline, copy: headline, rows };
}
