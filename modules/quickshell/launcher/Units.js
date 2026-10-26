// Converting a measure from one unit to another, as the launcher answers it:
// "5 km in mi", "100f to c", "1 gib as mb", "3 cups in ml", and a measure
// alone in its kind's base unit, "25ms" in seconds. Each kind of measure is
// a table of its units, each with how many of the kind's base it is and the
// names it's typed as, so a new unit is one entry. Temperature is the one
// kind that isn't a plain factor, so its scales carry an offset too. What
// the table can't answer, a unit it doesn't hold or measures worked out
// together ("5 ft + 3 in in cm", "1 GiB / (10 Mbit/s) in min"), is handed
// to the shell's maths (the Maths module, libqalculate), with each name the
// table knows put as the maths names that unit. Plain JavaScript, the maths
// given to it, so tests/answers.qml runs it in Qt's own engine.
.pragma library
.import "Numbers.js" as Numbers

// Each unit as { symbol, size, names }: how many of the base it is and the
// names it's typed as; the symbol is typed as well, lowercased. `maths` is
// the maths' name for it where its symbol means something else there, as
// pt is a printer's point and qt a quectotonne. Each kind's
// glyph is the name of the theme's mark for what it measures, which
// Units.qml looks up, since a library can't reach the theme.
const kinds = {
    length: {
        glyph: "distance",
        units: [
            { symbol: "nm", size: 1e-9,
                names: ["nanometre", "nanometres", "nanometer", "nanometers"] },
            { symbol: "\u00b5m", size: 1e-6,
                names: ["um", "micron", "microns", "micrometre", "micrometres"] },
            { symbol: "mm", size: 1e-3,
                names: ["millimetre", "millimetres", "millimeter", "millimeters"] },
            { symbol: "cm", size: 1e-2,
                names: ["centimetre", "centimetres", "centimeter", "centimeters"] },
            { symbol: "m", size: 1, names: ["metre", "metres", "meter", "meters"] },
            { symbol: "km", size: 1e3,
                names: ["kilometre", "kilometres", "kilometer", "kilometers", "kms"] },
            { symbol: "in", size: 0.0254, names: ["inch", "inches", "\""] },
            { symbol: "ft", size: 0.3048, names: ["foot", "feet", "'"] },
            { symbol: "yd", size: 0.9144, names: ["yard", "yards"] },
            { symbol: "mi", size: 1609.344, names: ["mile", "miles"] },
            { symbol: "nmi", size: 1852, names: ["nautical mile", "nautical miles"] }
        ]
    },
    mass: {
        glyph: "mass",
        units: [
            { symbol: "mg", size: 1e-6, names: ["milligram", "milligrams"] },
            { symbol: "g", size: 1e-3, names: ["gram", "grams"] },
            { symbol: "kg", size: 1, names: ["kilogram", "kilograms", "kilo", "kilos", "kgs"] },
            { symbol: "t", size: 1e3, names: ["tonne", "tonnes", "metric ton", "metric tons"] },
            { symbol: "oz", size: 0.028_349_523_125, names: ["ounce", "ounces"] },
            { symbol: "lb", size: 0.453_592_37, names: ["lbs", "pound", "pounds"] },
            { symbol: "st", size: 6.350_293_18, names: ["stone", "stones"], maths: "stone" }
        ]
    },
    // In litres; cups, pints, quarts and gallons are American.
    volume: {
        glyph: "capacity",
        units: [
            { symbol: "mL", size: 1e-3,
                names: ["ml", "millilitre", "millilitres", "milliliter", "milliliters"] },
            { symbol: "cL", size: 1e-2, names: ["cl", "centilitre", "centilitres"] },
            { symbol: "dL", size: 1e-1, names: ["dl", "decilitre", "decilitres"] },
            { symbol: "L", size: 1, names: ["l", "litre", "litres", "liter", "liters"] },
            { symbol: "m\u00b3", size: 1e3, names: ["m3", "cubic metre", "cubic metres"] },
            { symbol: "tsp", size: 0.004_928_921_593_75, names: ["teaspoon", "teaspoons"],
                maths: "tsp_US" },
            { symbol: "tbsp", size: 0.014_786_764_781_25,
                names: ["tablespoon", "tablespoons"], maths: "tbsp_US" },
            { symbol: "fl oz", size: 0.029_573_529_562_5,
                names: ["floz", "fluid ounce", "fluid ounces"], maths: "fl_oz" },
            { symbol: "cup", size: 0.236_588_236_5, names: ["cups"], maths: "cup_US" },
            { symbol: "pt", size: 0.473_176_473, names: ["pint", "pints"], maths: "liq_pt" },
            { symbol: "qt", size: 0.946_352_946, names: ["quart", "quarts"], maths: "liq_qt" },
            { symbol: "gal", size: 3.785_411_784, names: ["gallon", "gallons"] }
        ]
    },
    area: {
        glyph: "area",
        units: [
            { symbol: "m\u00b2", size: 1, names: ["m2", "sqm", "square metre", "square metres"] },
            { symbol: "km\u00b2", size: 1e6,
                names: ["km2", "square kilometre", "square kilometres"] },
            { symbol: "ha", size: 1e4, names: ["hectare", "hectares"] },
            { symbol: "ft\u00b2", size: 0.092_903_04,
                names: ["ft2", "sqft", "square foot", "square feet"] },
            { symbol: "ac", size: 4046.856_422_4, names: ["acre", "acres"], maths: "acre" },
            { symbol: "mi\u00b2", size: 2_589_988.110_336,
                names: ["mi2", "square mile", "square miles"] }
        ]
    },
    // A kilometre an hour is 1 / 3.6 metres a second, and a knot a nautical
    // mile, 1852 metres, an hour.
    speed: {
        glyph: "speed",
        units: [
            { symbol: "m/s", size: 1, names: ["mps", "metres per second"] },
            { symbol: "km/h", size: 0.277_777_777_777_777_8,
                names: ["kmh", "kph", "kilometres per hour"] },
            { symbol: "mph", size: 0.447_04, names: ["miles per hour"] },
            { symbol: "kn", size: 0.514_444_444_444_444_5, names: ["kt", "knot", "knots"],
                maths: "knot" }
        ]
    },
    // In bytes: a thousand to a kilobyte, 1,024 to a kibibyte and each power
    // of it to the next, and eight bits to a byte.
    data: {
        glyph: "data",
        units: [
            { symbol: "bit", size: 0.125, names: ["bits"] },
            { symbol: "B", size: 1, names: ["b", "byte", "bytes"] },
            { symbol: "kB", size: 1e3, names: ["kb", "kilobyte", "kilobytes"] },
            { symbol: "MB", size: 1e6, names: ["mb", "megabyte", "megabytes"] },
            { symbol: "GB", size: 1e9, names: ["gb", "gigabyte", "gigabytes"] },
            { symbol: "TB", size: 1e12, names: ["tb", "terabyte", "terabytes"] },
            { symbol: "PB", size: 1e15, names: ["pb", "petabyte", "petabytes"] },
            { symbol: "KiB", size: 1024, names: ["kib", "kibibyte", "kibibytes"] },
            { symbol: "MiB", size: 1_048_576, names: ["mib", "mebibyte", "mebibytes"] },
            { symbol: "GiB", size: 1_073_741_824, names: ["gib", "gibibyte", "gibibytes"] },
            { symbol: "TiB", size: 1_099_511_627_776,
                names: ["tib", "tebibyte", "tebibytes"] },
            { symbol: "PiB", size: 1_125_899_906_842_624,
                names: ["pib", "pebibyte", "pebibytes"] },
            { symbol: "kbit", size: 125, names: ["kilobit", "kilobits"] },
            { symbol: "Mbit", size: 125_000, names: ["megabit", "megabits"] },
            { symbol: "Gbit", size: 125_000_000, names: ["gigabit", "gigabits"] }
        ]
    },
    // In seconds; a year is a Julian one, 365.25 days.
    time: {
        glyph: "duration",
        units: [
            { symbol: "ns", size: 1e-9, names: ["nanosecond", "nanoseconds"] },
            { symbol: "\u00b5s", size: 1e-6, names: ["us", "microsecond", "microseconds"] },
            { symbol: "ms", size: 1e-3, names: ["millisecond", "milliseconds"] },
            { symbol: "s", size: 1, names: ["sec", "secs", "second", "seconds"] },
            { symbol: "min", size: 60, names: ["mins", "minute", "minutes"] },
            { symbol: "h", size: 3600, names: ["hr", "hrs", "hour", "hours"] },
            { symbol: "d", size: 86_400, names: ["day", "days"] },
            { symbol: "wk", size: 604_800, names: ["week", "weeks"], maths: "week" },
            { symbol: "yr", size: 31_557_600, names: ["year", "years"] }
        ]
    },
    energy: {
        glyph: "energy",
        units: [
            { symbol: "J", size: 1, names: ["j", "joule", "joules"] },
            { symbol: "kJ", size: 1e3, names: ["kj", "kilojoule", "kilojoules"] },
            { symbol: "cal", size: 4.184, names: ["calorie", "calories"] },
            { symbol: "kcal", size: 4184, names: ["kilocalorie", "kilocalories", "food calories"] },
            { symbol: "Wh", size: 3600, names: ["wh", "watt hour", "watt hours"] },
            { symbol: "kWh", size: 3.6e6, names: ["kwh", "kilowatt hour", "kilowatt hours"] }
        ]
    },
    pressure: {
        glyph: "pressure",
        units: [
            { symbol: "Pa", size: 1, names: ["pa", "pascal", "pascals"] },
            { symbol: "kPa", size: 1e3, names: ["kpa", "kilopascal", "kilopascals"] },
            { symbol: "hPa", size: 100,
                names: ["hpa", "hectopascal", "hectopascals", "mbar", "millibar"] },
            { symbol: "bar", size: 1e5, names: ["bars"] },
            { symbol: "psi", size: 6894.757_293_168, names: ["pounds per square inch"] },
            { symbol: "atm", size: 101_325, names: ["atmosphere", "atmospheres"] }
        ]
    },
    // In degrees; a radian is 180 / pi of them.
    angle: {
        glyph: "angle",
        units: [
            { symbol: "\u00b0", size: 1, names: ["deg", "degree", "degrees"] },
            { symbol: "rad", size: 57.295_779_513_082_32, names: ["radian", "radians"] },
            { symbol: "grad", size: 0.9, names: ["gon", "gradian", "gradians"], maths: "gon" },
            { symbol: "turn", size: 360, names: ["turns", "rev", "revolution", "revolutions"] }
        ]
    },
    // In kelvin: each scale's size is the kelvin a degree of it is, 5 / 9 of
    // one for Fahrenheit's, and its zero where it starts.
    temperature: {
        glyph: "temperature",
        units: [
            { symbol: "\u00b0C", size: 1, zero: 273.15, called: "Celsius",
                names: ["c", "\u00b0c", "celsius", "centigrade"] },
            { symbol: "\u00b0F", size: 0.555_555_555_555_555_6, called: "Fahrenheit",
                zero: 255.372_222_222_222_2, names: ["f", "\u00b0f", "fahrenheit"] },
            { symbol: "K", size: 1, zero: 0, called: "Kelvin", names: ["k", "kelvin", "kelvins"] }
        ]
    }
};

// Six figures are enough to read and to use, and no more are claimed.
const FIGURES = 6;
// The most places a number is written with before it takes an exponent.
const MOST_PLACES = 12;
// The gaps between a unit's words, however they're typed.
const GAPS = /\s+/g;
// The zeros before an exponent, and a point left bare by them.
const ZEROS_BEFORE_EXPONENT = /\.?0+e/;
// What groups an amount's digits as it's typed, and is left out to read it.
const GROUPING = /[,_]/g;

// Every name a unit is typed as, lowercased, to its kind and its row.
const named = (() => {
    const all = {};
    for (const [kind, { units }] of Object.entries(kinds)) {
        for (const unit of units) {
            for (const name of [unit.symbol.toLowerCase()].concat(unit.names)) {
                all[name] = { kind, unit };
            }
        }
    }
    return all;
})();

function lookup(name) {
    return named[name.trim().toLowerCase().replace(GAPS, " ")] ?? null;
}

// A unit's value in the kind's base, and back.
function toBase(kind, unit, value) {
    return kind === "temperature" ? value * unit.size + unit.zero : value * unit.size;
}

function fromBase(kind, unit, value) {
    return kind === "temperature" ? (value - unit.zero) / unit.size : value / unit.size;
}

function figures(value, group, point) {
    if (value === 0 || !Number.isFinite(value)) {
        return Numbers.format(value, 0, group, point);
    }
    const places = Math.max(0, FIGURES - 1 - Math.floor(Math.log10(Math.abs(value))));
    if (places > MOST_PLACES) {
        return value.toExponential(FIGURES - 1).replace(ZEROS_BEFORE_EXPONENT, "e");
    }
    return Numbers.format(value, places, group, point);
}

// A value as it's copied, in as many figures, without the zeros after them.
function copied(value) {
    return String(Number(value.toPrecision(FIGURES)));
}

// A measure as it's said, its figures then its unit's symbol, which a lone
// degree sign is written against.
function saying(figured, unit) {
    const gap = unit.symbol.startsWith("\u00b0") && unit.symbol.length === 1 ? "" : " ";
    return `${figured}${gap}${unit.symbol}`;
}

// An amount, its unit, a word for "into", and the unit wanted.
const number = "-?(?:\\d[\\d,_]*(?:\\.\\d+)?|\\.\\d+)(?:e[+-]?\\d+)?";
const asking = new RegExp(`^(${number})\\s*(.+?)\\s+(?:in|to|as|into)\\s+(.+)$`, "i");
// An amount and its unit alone: "25ms".
const alone = new RegExp(`^(${number})\\s*(\\S.*)$`, "i");
// The names a measure alone isn't answered by: a letter, which a word
// such as "4k" or "3d" ends in, and "st", which ends "1st".
const SHORTEST_ALONE = 2;
const NOT_ALONE = ["st"];
// The scale a temperature alone is said in: Fahrenheit for Celsius, and
// Celsius for the others.
const WARMTH = { "\u00b0C": "\u00b0F", "\u00b0F": "\u00b0C", K: "\u00b0C" };
// What a measure handed to the maths starts with: a number or a bracket.
const MEASURE_START = /^[-+]?(?:\d|\.\d|\()/;
// The unit asked for at the end, after the last word for "into", so the
// inches in "3 in in cm" stay with the measure.
const INTO_END = /^(.*\S)\s+(?:in|to|as|into)\s+(\S.*)$/i;
// An operator between measures, after a sign the first might carry.
const BETWEEN = /[+\-*/^\u00d7\u00f7]/;
// A name in a measure, which the table may know.
const WORD = /[a-z\u00b5\u00b0]+/gi;

// The answer to what's typed, or null when it isn't a conversion or a
// measure. `maths` (the Maths module) answers what the table can't.
function answer(text, options, maths) {
    const typed = text.trim();
    const asked = asking.exec(typed);
    if (asked) {
        const from = lookup(asked[2]);
        const to = lookup(asked[3]);
        // Both units known, the table's answer stands, even when it's none,
        // since a kilometre is no weight and in kilometres is no change.
        if (from && to) {
            return from.kind === to.kind ? converted(asked[1], from, to.unit, options) : null;
        }
    }
    return measureAlone(typed, options) ?? worked(typed, options, maths);
}

// A measure alone, in its kind's base unit, or a temperature in the scale
// it's most often wanted in: "25ms" as 0.025 s, "5 km" as 5,000 m.
function measureAlone(typed, options) {
    const asked = alone.exec(typed);
    const from = asked ? lookup(asked[2]) : null;
    if (!from) {
        return null;
    }
    const name = asked[2].trim().toLowerCase();
    const sign = name === "'" || name === "\"";
    if (!sign && (name.length < SHORTEST_ALONE || NOT_ALONE.includes(name))) {
        return null;
    }
    const { units } = kinds[from.kind];
    const to = from.kind === "temperature"
        ? units.find(unit => unit.symbol === WARMTH[from.unit.symbol])
        : units.find(unit => unit.size === 1);
    return converted(asked[1], from, to, options);
}

// An amount of one unit in another of its kind, with the rate each way as
// rows, or for a temperature the third scale; null when they're the same.
function converted(written, from, to, options) {
    if (from.unit === to) {
        return null;
    }
    const group = options ? options.group : undefined;
    const point = options ? options.point : undefined;
    const { kind } = from;
    const amount = Number.parseFloat(written.replace(GROUPING, ""));
    const convert = (quantity, a, b) => fromBase(kind, b, toBase(kind, a, quantity));
    const said = (quantity, unit) => saying(figures(quantity, group, point), unit);
    const value = convert(amount, from.unit, to);
    const { glyph } = kinds[kind];
    // A temperature has no rate to give, so its rows are in the third
    // scale, each named first as a part is: "Kelvin 373.15 K".
    const rows = kind === "temperature"
        ? kinds.temperature.units.filter(scale => scale !== from.unit && scale !== to)
            .map(scale => {
                const there = convert(amount, from.unit, scale);
                return { text: scale.called, side: said(there, scale), copy: copied(there) };
            })
        : rates(convert(1, from.unit, to), from.unit, to, said);
    return { headline: said(value, to), copy: copied(value), glyph, kind, rows };
}

// Each name in a measure the table knows, put as the maths names its unit.
function forMaths(measure) {
    return measure.replace(WORD, word => {
        const known = lookup(word);
        return known ? known.unit.maths ?? known.unit.symbol : word;
    });
}

// What the maths makes of a measure the table can't answer: a unit it
// doesn't hold, "5 parsecs in km", or measures worked out together, "5 ft
// + 3 in in cm". Only one starting with a number, asking for a unit or
// working something out, and coming to a measure in the unit asked for,
// is answered, said in the field with no rows, since there's no one rate.
function worked(typed, options, maths) {
    if (!(maths && MEASURE_START.test(typed))) {
        return null;
    }
    const into = INTO_END.exec(typed);
    if (!(into || BETWEEN.test(typed.slice(1)))) {
        return null;
    }
    const said = maths.calculate(into ? `${forMaths(into[1])} to ${forMaths(into[2])}`
        : forMaths(typed));
    if (!said.answered || !said.units || said.currency || said.unknown || said.time
        || !said.unit || (into && !said.fits)) {
        return null;
    }
    const group = options ? options.group : undefined;
    const point = options ? options.point : undefined;
    const gap = said.unit === "\u00b0" ? "" : " ";
    return {
        headline: `${figures(said.amount, group, point)}${gap}${said.unit}`,
        copy: copied(said.amount),
        glyph: "",
        kind: "",
        rows: []
    };
}

// The rows saying what one of each unit is in the other, `each` being one
// of `from` in `to`: named by the one, "1 km", then saying it in the
// other, "0.621371 mi".
function rates(each, from, to, said) {
    return [
        { text: `1 ${from.symbol}`, side: said(each, to), copy: copied(each) },
        { text: `1 ${to.symbol}`, side: said(1 / each, from), copy: copied(1 / each) }
    ];
}
