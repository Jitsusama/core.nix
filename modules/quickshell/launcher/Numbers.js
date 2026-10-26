// Sums and number bases for the launcher's answers: what counts as one,
// what's handed to the shell's maths (libqalculate, through the Maths
// module, modules/quickshell/maths), and how its answer is written. Plain
// JavaScript, the maths given to it, so a check can run it in Qt's own
// engine with the real module beside it.
//
// An answer is given only when what's typed is plainly a sum or a base:
// "12*7", "2^10", "(3+4)/2", "0x1f", "255 in hex", "0b1011 << 3", "~5",
// "5 xor 3", "20% of 50", "3 squared". A lone number is never answered,
// since "2" is as likely the start of "2fa", nor a measure, which Units
// answers, nor money, a date or a name the maths doesn't know.
// Numbers are written with our own digit grouping, since Qt's JavaScript
// ignores the locale's.
.pragma library

const bases = {
    hex: 16, hexadecimal: 16,
    dec: 10, decimal: 10,
    oct: 8, octal: 8,
    bin: 2, binary: 2
};
const prefixes = { 16: "0x", 8: "0o", 2: "0b", 10: "" };
const names = { 16: "hex", 8: "octal", 2: "binary", 10: "decimal" };
// The digits the maths gives a whole number in, by base.
const digitsIn = { 16: "hex", 8: "octal", 2: "binary", 10: "digits" };
// The bases a whole number's answer is shown in, decimal first.
const shownBases = [bases.dec, bases.hex, bases.oct, bases.bin];

// What a sum is written with: a digit, an operator or a bracket, at least
// one of which it has, so words alone, such as "battery life", are never
// handed to the maths.
const SUM_MARKS = /[\d+\-*/^%()&|~!<>\u00d7\u00f7]/;
// What the launcher's other answers own: a colon, as an address, a hardware
// address or a time has, a colour's #, and a number with two points, as
// 8.8.8.8, which the maths would read as one with a stray point.
const OTHERS = /[:#]|\d*\.\d*\./;
// A number written in a base, by its prefix.
const BASED = /\b0[xob][\da-f]/i;
// The operators that work on bits, after which every base is shown.
const BITWISE = /<<|>>|[&|~]|\b(?:xor|nand|nor)\b/i;
// A sum asked for in a base: "255 in hex".
const IN_BASE = /^(.*\S)\s+(?:in|to|as)\s+([a-z]+)$/i;
// A conversion into anything else, which is Units' to answer.
const CONVERTED = /\s(?:in|to|as|into)\s+\S/i;
// The marks grouping a number's digits as it's typed, which the maths
// would read as a list: a comma or an underscore before three digits, or an
// underscore between a based number's digits.
const GROUPED = /(\d)[,_](?=\d{3}(?!\d))/g;
const BASED_GROUPS = /([\da-f])_(?=[\da-f])/gi;
// Words for operators, as they're said, in what the maths reads instead:
// "log" is the common logarithm here, as on a calculator, where the maths
// reads it as the natural one, and "of" after a percentage multiplies.
const SAID = [
    [/\blog\s*\(/gi, "log10("],
    [/\s*\bsquared\b/gi, "^2"],
    [/\s*\bcubed\b/gi, "^3"],
    [/%\s*of\b/gi, "% *"]
];
// The size past which a number is written with an exponent rather than its
// places, and below which a fraction is: toFixed writes an exponent from
// 1e21, and ten places can't show less than 1e-10.
const LARGEST_FIXED = 1e21;
const SMALLEST_FIXED = 1e-10;
// Where a whole number's groups of three start, counting from its end.
const THOUSANDS = /\B(?=(\d{3})+(?!\d))/g;
// The zeros a fixed number ends with after its point, and a point left bare.
const TRAILING_ZEROS = /\.?0+$/;
// The places a number has, unless the caller asks for others.
const PLACES = 10;
// What a place is divided by for its last two digits.
const HUNDREDS = 100;
// The teens, 11th to 13th, which end in "th" whatever their last digit.
const FIRST_TEEN = 11;
const LAST_TEEN = 13;

// A number grouped in threes, with at most `decimals` places, trailing
// zeros dropped: 1234567.891 as "1,234,567.891".
function format(value, decimals, group, point) {
    // The value is read as a number, as it always has been, given as text.
    if (!Number.isFinite(Number(value))) {
        if (Number.isNaN(Number(value))) {
            return "not a number";
        }
        return value > 0 ? "\u221e" : "-\u221e";
    }
    const places = decimals ?? PLACES;
    const sign = value < 0 ? "-" : "";
    const exact = Math.abs(value).toFixed(places);
    const fixed = exact.includes(".") ? exact.replace(TRAILING_ZEROS, "") : exact;
    const [whole, fraction] = fixed.split(".");
    const grouped = whole.replace(THOUSANDS, group ?? ",");
    return sign + grouped + (fraction ? (point ?? ".") + fraction : "");
}

// A place in a line as it's said: 1 as "1st", 12 as "12th", 22 as "22nd".
function ordinal(place) {
    const tens = place % HUNDREDS;
    const suffix = tens >= FIRST_TEEN && tens <= LAST_TEEN ? "th"
        : ["th", "st", "nd", "rd"][place % 10] ?? "th";
    return `${place}${suffix}`;
}

// A whole number in a base, with its prefix, from the maths' signed
// digits: -1f in 16 as "-0x1f".
function inBase(said, base) {
    const digits = said[digitsIn[base]];
    const sign = digits.startsWith("-") ? "-" : "";
    return sign + prefixes[base] + digits.slice(sign.length);
}

// What's typed, apart from the base it's asked for in: { asked, into },
// `into` being null when it names none.
function wanting(typed) {
    const wanted = IN_BASE.exec(typed);
    if (wanted && bases[wanted[2].toLowerCase()]) {
        return { asked: wanted[1], into: bases[wanted[2].toLowerCase()] };
    }
    return { asked: typed, into: null };
}

// Whether what's typed could be a sum, before the maths is asked.
function sumLike(asked) {
    return SUM_MARKS.test(asked) && !OTHERS.test(asked) && !CONVERTED.test(asked);
}

// A sum as the maths reads it: digits ungrouped, and words said for
// operators put as the maths writes them.
function written(asked) {
    const ungrouped = asked.replace(GROUPED, "$1").replace(BASED_GROUPS, "$1");
    return SAID.reduce((text, [said, meant]) => text.replace(said, meant), ungrouped);
}

// Whether the maths' answer is plainly a sum's: worked out, no measure,
// money, date or unknown name in it, and more than a lone number, unless
// that's written in a base or asked for in one.
function plainly(said, based, into) {
    if (!said.answered || said.units || said.currency || said.unknown || said.time) {
        return false;
    }
    if (said.plain && !based && into === null) {
        return false;
    }
    return said.real || said.complex;
}

// A real number as it's shown, `mark` grouping it, and as it's copied.
function figures(said, settings, mark) {
    if (said.digits !== undefined) {
        return said.digits.replace(THOUSANDS, mark);
    }
    const size = Math.abs(said.number);
    if (!Number.isFinite(said.number) || size >= LARGEST_FIXED
        || (size > 0 && size < SMALLEST_FIXED)) {
        return said.text;
    }
    return format(said.number, settings.decimals, mark, settings.point);
}

// What's typed, answered by `maths` (the Maths module): { headline, copy,
// rows: [{ text, side, copy }] }, or null when it isn't plainly a sum or a
// base. `options` gives decimals and the grouping and decimal marks. What's
// copied is never grouped, so it pastes as a number.
function answer(text, options, maths) {
    const settings = options ?? {};
    const { asked, into } = wanting(text.trim());
    if (!sumLike(asked)) {
        return null;
    }
    const said = maths.calculate(written(asked));
    const based = BASED.test(asked);
    if (!plainly(said, based, into)) {
        return null;
    }
    const whole = said.digits !== undefined;
    if (into !== null && !whole) {
        return null;
    }
    if (said.complex) {
        return { headline: said.text, copy: said.text, rows: [] };
    }
    const shown = figures(said, settings, settings.group ?? ",");
    const bare = figures(said, settings, "");
    const inOne = into !== null && into !== bases.dec;
    const headline = inOne ? inBase(said, into) : shown;
    const rows = whole && (based || into !== null || BITWISE.test(asked))
        ? inBases(said, into, shown, bare) : [];
    return { headline, copy: inOne ? headline : bare, rows };
}

// The rows of a whole number in each base but the one it's answered in,
// `shown` writing it in decimal and `bare` as it's copied.
function inBases(said, into, shown, bare) {
    return shownBases.filter(base => base !== (into ?? bases.dec)).map(base => (base === bases.dec
        ? { text: shown, side: names[base], copy: bare }
        : { text: inBase(said, base), side: names[base], copy: inBase(said, base) }));
}
