// A colour in its other forms, as the launcher answers it: "#ff5345",
// "rgb(255 83 69)", "hsl(4.5 100% 63.5%)", "oklch(68.4% 0.21 29.3)" or
// "colour accent" for one of the theme's own. It's the one file outside the
// theme that names colours, because colours are what it's asked about.
// Plain JavaScript, so tests/answers.qml runs it in Qt's own engine.
.pragma library

// Rounded to `places` without trailing zeros, and never "-0".
function figure(value, places) {
    const rounded = Number(value.toFixed(places));
    return String(rounded === 0 ? 0 : rounded);
}

// An angle, where 359.96 rounds round to 0.
function angle(degrees) {
    return figure(Number(degrees.toFixed(1)) % 360, 1);
}

function clamp(value) {
    return Math.min(1, Math.max(0, value));
}

function linear(channel) {
    return channel <= 0.04045 ? channel / 12.92 : Math.pow((channel + 0.055) / 1.055, 2.4);
}

function gamma(channel) {
    return channel <= 0.0031308 ? 12.92 * channel
        : 1.055 * Math.pow(channel, 1 / 2.4) - 0.055;
}

// Björn Ottosson's Oklab, from sRGB and back, each channel from 0 to 1.
function oklab([r, g, b]) {
    const [lr, lg, lb] = [linear(r), linear(g), linear(b)];
    const l = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
    const m = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
    const s = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);
    return [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s
    ];
}

function fromOklab([lightness, a, b]) {
    const l = Math.pow(lightness + 0.3963377774 * a + 0.2158037573 * b, 3);
    const m = Math.pow(lightness - 0.1055613458 * a - 0.0638541728 * b, 3);
    const s = Math.pow(lightness - 0.0894841775 * a - 1.2914855480 * b, 3);
    return [
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
    ].map(channel => clamp(gamma(channel)));
}

function hsl([r, g, b]) {
    const high = Math.max(r, g, b);
    const low = Math.min(r, g, b);
    const lightness = (high + low) / 2;
    const span = high - low;
    if (span === 0)
        return [0, 0, lightness];
    const saturation = span / (1 - Math.abs(2 * lightness - 1));
    const hue = high === r ? ((g - b) / span + 6) % 6 : high === g ? (b - r) / span + 2
        : (r - g) / span + 4;
    return [hue * 60, saturation, lightness];
}

function fromHsl([hue, saturation, lightness]) {
    const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
    const sector = ((hue % 360) + 360) % 360 / 60;
    const x = chroma * (1 - Math.abs(sector % 2 - 1));
    const base = lightness - chroma / 2;
    const [r, g, b] = sector < 1 ? [chroma, x, 0] : sector < 2 ? [x, chroma, 0]
        : sector < 3 ? [0, chroma, x] : sector < 4 ? [0, x, chroma]
        : sector < 5 ? [x, 0, chroma] : [chroma, 0, x];
    return [r + base, g + base, b + base].map(clamp);
}

function hex(colour) {
    const digits = colour.rgb.map(channel => Math.round(channel * 255));
    if (colour.alpha < 1)
        digits.push(Math.round(colour.alpha * 255));
    return `#${digits.map(digit => digit.toString(16).padStart(2, "0")).join("")}`;
}

function alpha(colour) {
    return colour.alpha < 1 ? ` / ${figure(colour.alpha, 3)}` : "";
}

const forms = {
    hex: hex,
    rgb: colour => `rgb(${colour.rgb.map(c => Math.round(c * 255)).join(" ")}${alpha(colour)})`,
    hsl: colour => {
        const [h, s, l] = hsl(colour.rgb);
        return `hsl(${angle(h)} ${figure(s * 100, 1)}% ${figure(l * 100, 1)}%${alpha(colour)})`;
    },
    oklch: colour => {
        const [l, a, b] = oklab(colour.rgb);
        const chroma = Math.hypot(a, b);
        // A grey has no hue, which CSS writes as none.
        const hue = chroma < 0.0005 ? "none"
            : angle((Math.atan2(b, a) * 180 / Math.PI + 360) % 360);
        return `oklch(${figure(l * 100, 1)}% ${figure(chroma, 3)} ${hue}${alpha(colour)})`;
    }
};

// A number as CSS writes it: plain, or a percentage of `whole`.
function amount(text, whole) {
    const found = /^(-?\d*\.?\d+)(%|deg)?$/.exec(text);
    if (!found)
        return NaN;
    const value = parseFloat(found[1]);
    return found[2] === "%" ? value / 100 * whole : value;
}

function opacity(text) {
    if (text === undefined)
        return 1;
    const value = amount(text, 1);
    return value >= 0 && value <= 1 ? value : NaN;
}

// What's typed, read as a colour and the form it was written in, or null.
function read(text) {
    const typed = text.trim().toLowerCase();
    const digits = /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/.exec(typed);
    if (digits) {
        const short = digits[1].length <= 4;
        const pairs = short ? digits[1].split("").map(d => d + d)
            : digits[1].match(/../g);
        const values = pairs.map(pair => parseInt(pair, 16) / 255);
        return { form: "hex", rgb: values.slice(0, 3), alpha: values[3] ?? 1 };
    }
    const called = /^(rgba?|hsla?|oklch)\s*\(?\s*([^()]*?)\s*\)?$/.exec(typed);
    if (!called)
        return null;
    const parts = called[2].split(/[\s,/]+/).filter(part => part !== "");
    if (parts.length < 3 || parts.length > 4)
        return null;
    const kind = called[1].replace(/a$/, "");
    const shade = opacity(parts[3]);
    let rgb = null;
    if (kind === "rgb") {
        rgb = parts.slice(0, 3).map(part => amount(part, 255));
        if (rgb.some(value => !(value >= 0 && value <= 255)))
            return null;
        rgb = rgb.map(value => value / 255);
    } else if (kind === "hsl") {
        const [h, s, l] = [amount(parts[0], 360), amount(parts[1], 100),
            amount(parts[2], 100)];
        if ([h, s, l].some(isNaN) || s < 0 || s > 100 || l < 0 || l > 100)
            return null;
        rgb = fromHsl([h, s / 100, l / 100]);
    } else {
        const [l, c, h] = [amount(parts[0], 1), amount(parts[1], 0.4),
            parts[2] === "none" ? 0 : amount(parts[2], 360)];
        if ([l, c, h].some(isNaN) || l < 0 || l > 1 || c < 0)
            return null;
        const angle = h * Math.PI / 180;
        rgb = fromOklab([l, c * Math.cos(angle), c * Math.sin(angle)]);
    }
    return isNaN(shade) ? null : { form: kind, rgb: rgb, alpha: shade };
}

// The theme's pigments, from `palette` (name to "#rrggbb") and `roles`
// (role to the same), each with the roles that wear it.
function pigments(palette, roles) {
    const found = [];
    for (const name of Object.keys(palette ?? {})) {
        const colour = read(palette[name]);
        if (!colour)
            continue;
        const worn = Object.keys(roles ?? {})
            .filter(role => String(roles[role]).toLowerCase() === palette[name].toLowerCase());
        found.push({ name: name, colour: colour, lab: oklab(colour.rgb), roles: worn });
    }
    return found;
}

// Past this, no pigment is near enough to be worth naming.
const near = 10;

// The pigment nearest a colour, and how far (ΔE in Oklab, times 100), or
// null when none is near.
function nearest(colour, known) {
    const lab = oklab(colour.rgb);
    let best = null;
    for (const pigment of known) {
        const apart = 100 * Math.hypot(lab[0] - pigment.lab[0], lab[1] - pigment.lab[1],
            lab[2] - pigment.lab[2]);
        if (!best || apart < best.apart)
            best = { pigment: pigment, apart: apart };
    }
    return best && best.apart <= near ? best : null;
}

// The answer to what's typed, given the theme's `palette` and `roles`, or
// null when it isn't a colour. A row that shows its colour carries it as
// `swatch`, in Qt's own "#aarrggbb", which puts the alpha first.
function answer(text, palette, roles) {
    const known = pigments(palette, roles);
    let colour = read(text);
    const asked = /^colou?r\s+(.+)$/i.exec(text.trim());
    if (!colour && asked) {
        const name = asked[1].toLowerCase().replace(/\s+/g, "_");
        const pigment = known.find(p => p.name === name || p.roles.indexOf(name) >= 0);
        if (!pigment)
            return null;
        colour = { form: "name", rgb: pigment.colour.rgb, alpha: 1 };
    }
    if (!colour)
        return null;
    const leading = colour.form === "hex" ? "rgb" : "hex";
    const headline = forms[leading](colour);
    const swatch = c => `#${[c.alpha].concat(c.rgb)
        .map(channel => Math.round(channel * 255).toString(16).padStart(2, "0")).join("")}`;
    const rows = Object.keys(forms)
        .filter(form => form !== colour.form && form !== leading)
        .map((form, i) => ({ text: forms[form](colour), side: "", copy: forms[form](colour),
            swatch: i === 0 ? swatch(colour) : null }));
    const close = nearest(colour, known);
    if (close) {
        const apart = close.apart < 0.05 ? [] : [`\u0394E ${figure(close.apart, 1)}`];
        rows.push({
            text: close.pigment.name.replace(/_/g, " "),
            side: close.pigment.roles.concat(apart).join(" \u00b7 "),
            copy: hex(close.pigment.colour),
            swatch: swatch(close.pigment.colour)
        });
    }
    return { headline: headline, copy: headline, rows: rows };
}
