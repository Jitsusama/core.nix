// Converting a measure from one unit to another, as the launcher answers it:
// "5 km in mi", "100f to c", "1 gib as mb", "3 cups in ml". Each kind of
// measure is a table of its units, each with how many of the kind's base it
// is and the names it's typed as, so a new unit is one line. Temperature is
// the one kind that isn't a plain factor, so its scales carry an offset too.
// Plain JavaScript, so tests/answers.qml runs it in Qt's own engine.
.pragma library
.import "Numbers.js" as Numbers

// [symbol, how many of the base, names it's typed as]; the symbol is
// typed as well, lowercased.
const kinds = {
    length: {
        glyph: 0xf046d,
        units: [
            ["nm", 1e-9, ["nanometre", "nanometres", "nanometer", "nanometers"]],
            ["\u00b5m", 1e-6, ["um", "micron", "microns", "micrometre", "micrometres"]],
            ["mm", 1e-3, ["millimetre", "millimetres", "millimeter", "millimeters"]],
            ["cm", 1e-2, ["centimetre", "centimetres", "centimeter", "centimeters"]],
            ["m", 1, ["metre", "metres", "meter", "meters"]],
            ["km", 1e3, ["kilometre", "kilometres", "kilometer", "kilometers", "kms"]],
            ["in", 0.0254, ["inch", "inches", "\""]],
            ["ft", 0.3048, ["foot", "feet", "'"]],
            ["yd", 0.9144, ["yard", "yards"]],
            ["mi", 1609.344, ["mile", "miles"]],
            ["nmi", 1852, ["nautical mile", "nautical miles"]]
        ]
    },
    mass: {
        glyph: 0xf05d1,
        units: [
            ["mg", 1e-6, ["milligram", "milligrams"]],
            ["g", 1e-3, ["gram", "grams"]],
            ["kg", 1, ["kilogram", "kilograms", "kilo", "kilos", "kgs"]],
            ["t", 1e3, ["tonne", "tonnes", "metric ton", "metric tons"]],
            ["oz", 0.028349523125, ["ounce", "ounces"]],
            ["lb", 0.45359237, ["lbs", "pound", "pounds"]],
            ["st", 6.35029318, ["stone", "stones"]]
        ]
    },
    // In litres; cups, pints, quarts and gallons are American.
    volume: {
        glyph: 0xf01ab,
        units: [
            ["mL", 1e-3, ["ml", "millilitre", "millilitres", "milliliter", "milliliters"]],
            ["cL", 1e-2, ["cl", "centilitre", "centilitres"]],
            ["dL", 1e-1, ["dl", "decilitre", "decilitres"]],
            ["L", 1, ["l", "litre", "litres", "liter", "liters"]],
            ["m\u00b3", 1e3, ["m3", "cubic metre", "cubic metres"]],
            ["tsp", 0.00492892159375, ["teaspoon", "teaspoons"]],
            ["tbsp", 0.01478676478125, ["tablespoon", "tablespoons"]],
            ["fl oz", 0.0295735295625, ["floz", "fluid ounce", "fluid ounces"]],
            ["cup", 0.2365882365, ["cups"]],
            ["pt", 0.473176473, ["pint", "pints"]],
            ["qt", 0.946352946, ["quart", "quarts"]],
            ["gal", 3.785411784, ["gallon", "gallons"]]
        ]
    },
    area: {
        glyph: 0xf0fe6,
        units: [
            ["m\u00b2", 1, ["m2", "sqm", "square metre", "square metres"]],
            ["km\u00b2", 1e6, ["km2", "square kilometre", "square kilometres"]],
            ["ha", 1e4, ["hectare", "hectares"]],
            ["ft\u00b2", 0.09290304, ["ft2", "sqft", "square foot", "square feet"]],
            ["ac", 4046.8564224, ["acre", "acres"]],
            ["mi\u00b2", 2589988.110336, ["mi2", "square mile", "square miles"]]
        ]
    },
    speed: {
        glyph: 0xf04c5,
        units: [
            ["m/s", 1, ["mps", "metres per second"]],
            ["km/h", 1 / 3.6, ["kmh", "kph", "kilometres per hour"]],
            ["mph", 0.44704, ["miles per hour"]],
            ["kn", 1852 / 3600, ["kt", "knot", "knots"]]
        ]
    },
    // In bytes: a thousand to a kilobyte, 1,024 to a kibibyte.
    data: {
        glyph: 0xf02ca,
        units: [
            ["bit", 1 / 8, ["bits"]],
            ["B", 1, ["b", "byte", "bytes"]],
            ["kB", 1e3, ["kb", "kilobyte", "kilobytes"]],
            ["MB", 1e6, ["mb", "megabyte", "megabytes"]],
            ["GB", 1e9, ["gb", "gigabyte", "gigabytes"]],
            ["TB", 1e12, ["tb", "terabyte", "terabytes"]],
            ["PB", 1e15, ["pb", "petabyte", "petabytes"]],
            ["KiB", 1024, ["kib", "kibibyte", "kibibytes"]],
            ["MiB", 1024 ** 2, ["mib", "mebibyte", "mebibytes"]],
            ["GiB", 1024 ** 3, ["gib", "gibibyte", "gibibytes"]],
            ["TiB", 1024 ** 4, ["tib", "tebibyte", "tebibytes"]],
            ["PiB", 1024 ** 5, ["pib", "pebibyte", "pebibytes"]],
            ["kbit", 1e3 / 8, ["kilobit", "kilobits"]],
            ["Mbit", 1e6 / 8, ["megabit", "megabits"]],
            ["Gbit", 1e9 / 8, ["gigabit", "gigabits"]]
        ]
    },
    // In seconds; a year is a Julian one, 365.25 days.
    time: {
        glyph: 0xf051b,
        units: [
            ["ns", 1e-9, ["nanosecond", "nanoseconds"]],
            ["\u00b5s", 1e-6, ["us", "microsecond", "microseconds"]],
            ["ms", 1e-3, ["millisecond", "milliseconds"]],
            ["s", 1, ["sec", "secs", "second", "seconds"]],
            ["min", 60, ["mins", "minute", "minutes"]],
            ["h", 3600, ["hr", "hrs", "hour", "hours"]],
            ["d", 86400, ["day", "days"]],
            ["wk", 604800, ["week", "weeks"]],
            ["yr", 31557600, ["year", "years"]]
        ]
    },
    energy: {
        glyph: 0xf140b,
        units: [
            ["J", 1, ["j", "joule", "joules"]],
            ["kJ", 1e3, ["kj", "kilojoule", "kilojoules"]],
            ["cal", 4.184, ["calorie", "calories"]],
            ["kcal", 4184, ["kilocalorie", "kilocalories", "food calories"]],
            ["Wh", 3600, ["wh", "watt hour", "watt hours"]],
            ["kWh", 3.6e6, ["kwh", "kilowatt hour", "kilowatt hours"]]
        ]
    },
    pressure: {
        glyph: 0xf029a,
        units: [
            ["Pa", 1, ["pa", "pascal", "pascals"]],
            ["kPa", 1e3, ["kpa", "kilopascal", "kilopascals"]],
            ["hPa", 100, ["hpa", "hectopascal", "hectopascals", "mbar", "millibar"]],
            ["bar", 1e5, ["bars"]],
            ["psi", 6894.757293168, ["pounds per square inch"]],
            ["atm", 101325, ["atmosphere", "atmospheres"]]
        ]
    },
    angle: {
        glyph: 0xf0937,
        units: [
            ["\u00b0", 1, ["deg", "degree", "degrees"]],
            ["rad", 180 / Math.PI, ["radian", "radians"]],
            ["grad", 0.9, ["gon", "gradian", "gradians"]],
            ["turn", 360, ["turns", "rev", "revolution", "revolutions"]]
        ]
    },
    // Each scale as [symbol, degrees per kelvin's step, its zero in
    // kelvin, names].
    temperature: {
        glyph: 0xf050f,
        units: [
            ["\u00b0C", 1, 273.15, ["c", "\u00b0c", "celsius", "centigrade"]],
            ["\u00b0F", 5 / 9, 255.3722222222222, ["f", "\u00b0f", "fahrenheit"]],
            ["K", 1, 0, ["k", "kelvin", "kelvins"]]
        ]
    }
};

// Every name a unit is typed as, lowercased, to its kind and its row.
const named = (() => {
    const all = {};
    for (const kind in kinds)
        for (const unit of kinds[kind].units) {
            const names = unit[unit.length - 1];
            for (const name of [unit[0].toLowerCase()].concat(names))
                all[name] = { kind: kind, unit: unit };
        }
    return all;
})();

function unit(name) {
    return named[name.trim().toLowerCase().replace(/\s+/g, " ")] ?? null;
}

// A unit's value in the kind's base, and back.
function toBase(kind, unit, value) {
    return kind === "temperature" ? value * unit[1] + unit[2] : value * unit[1];
}

function fromBase(kind, unit, value) {
    return kind === "temperature" ? (value - unit[2]) / unit[1] : value / unit[1];
}

// Six figures are enough to read and to use, and no more are claimed.
function figures(value, group, point) {
    if (value === 0 || !isFinite(value))
        return Numbers.format(value, 0, group, point);
    const places = Math.max(0, 5 - Math.floor(Math.log10(Math.abs(value))));
    if (places > 12)
        return value.toExponential(5).replace(/\.?0+e/, "e");
    return Numbers.format(value, places, group, point);
}

// An amount, its unit, a word for "into", and the unit wanted.
const number = "-?(?:\\d[\\d,_]*(?:\\.\\d+)?|\\.\\d+)(?:e[+-]?\\d+)?";
const asking = new RegExp(`^(${number})\\s*(.+?)\\s+(?:in|to|as|into)\\s+(.+)$`, "i");

// The answer to what's typed, or null when it isn't a conversion.
function answer(text, options) {
    const asked = asking.exec(text.trim());
    if (!asked)
        return null;
    const from = unit(asked[2]);
    const to = unit(asked[3]);
    if (!from || !to || from.kind !== to.kind || from.unit === to.unit)
        return null;
    const group = options ? options.group : undefined;
    const point = options ? options.point : undefined;
    const kind = from.kind;
    const amount = parseFloat(asked[1].replace(/[,_]/g, ""));
    const convert = (value, a, b) => fromBase(kind, b.unit, toBase(kind, a.unit, value));
    const value = convert(amount, from, to);
    const said = (number, of) => `${figures(number, group, point)}${of.unit[0].startsWith("\u00b0")
        && of.unit[0].length === 1 ? "" : " "}${of.unit[0]}`;
    const glyph = kinds[kind].glyph;
    let rows;
    if (kind === "temperature")
        // The third scale, since a temperature has no rate to give.
        rows = kinds.temperature.units.filter(scale => scale !== from.unit && scale !== to.unit)
            .map(scale => {
                const other = { unit: scale };
                const there = convert(amount, from, other);
                return { text: said(there, other), side: "",
                    copy: String(Number(there.toPrecision(6))) };
            });
    else {
        const each = convert(1, from, to);
        rows = [
            { text: `1 ${from.unit[0]} = ${said(each, to)}`, side: "rate",
                copy: String(Number(each.toPrecision(6))) },
            { text: `1 ${to.unit[0]} = ${said(1 / each, from)}`, side: "inverse",
                copy: String(Number((1 / each).toPrecision(6))) }
        ];
    }
    return {
        headline: said(value, to),
        copy: String(Number(value.toPrecision(6))),
        glyph: glyph,
        kind: kind,
        rows: rows
    };
}
