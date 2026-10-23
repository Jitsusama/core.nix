// Arithmetic and number bases for the launcher's answers, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does.
//
// An answer is given only when what's typed is plainly a sum or a base:
// "12*7", "2^10", "(3+4)/2", "0x1f", "255 in hex", "0o17 to dec". A lone
// number is never answered, since "2" is as likely the start of "2fa".
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
const prefixed = { x: 16, o: 8, b: 2 };
const names = { 16: "hex", 8: "octal", 2: "binary", 10: "decimal" };

const functions = {
    sqrt: Math.sqrt, abs: Math.abs, round: Math.round, floor: Math.floor, ceil: Math.ceil,
    ln: Math.log, log: Math.log10, log2: Math.log2, exp: Math.exp,
    sin: Math.sin, cos: Math.cos, tan: Math.tan, asin: Math.asin, acos: Math.acos, atan: Math.atan
};
const constants = { pi: Math.PI, e: Math.E, tau: 2 * Math.PI };
// The bases a whole number's answer is shown in, decimal first.
const shownBases = [bases.dec, bases.hex, bases.oct, bases.bin];

// A piece of what's typed, read where the last left off: a number in a
// base, a decimal one, a name, or an operator.
const SPELLED = new RegExp([
    "\\s*(?:(0x[0-9a-f_]+|0o[0-7_]+|0b[01_]+)",
    "(\\d[\\d_,]*(?:\\.\\d+)?(?:e[+-]?\\d+)?|\\.\\d+)",
    "([a-z]+)",
    // biome-ignore lint/security/noSecrets: it's the operators a sum is written with.
    "(\\*\\*|[-+*/%^()\u00d7\u00f7]))"
].join("|"), "iy");
// What groups a number's digits as it's typed, and is left out to read it.
const DIGIT_GROUPING = /_/g;
const DECIMAL_GROUPING = /[_,]/g;
// The zeros a fixed number ends with after its point, and a point left bare.
const TRAILING_ZEROS = /\.?0+$/;
// Where a whole number's groups of three start, counting from its end.
const THOUSANDS = /\B(?=(\d{3})+(?!\d))/g;
// A sum asked for in a base: "255 in hex".
const IN_BASE = /^(.*\S)\s+(?:in|to|as)\s+([a-z]+)$/i;
// The places a number has, unless the caller asks for others.
const PLACES = 10;
// What a place is divided by for its last two digits.
const HUNDREDS = 100;
// The teens, 11th to 13th, which end in "th" whatever their last digit.
const FIRST_TEEN = 11;
const LAST_TEEN = 13;

// The pieces of what's typed: numbers, with the base they were written in,
// names, and operators. Null when anything else is in it.
function tokens(text) {
    const out = [];
    let at = 0;
    while (at < text.length) {
        SPELLED.lastIndex = at;
        const m = SPELLED.exec(text);
        if (!m) {
            if (text.slice(at).trim() === "") {
                break;
            }
            return null;
        }
        at = SPELLED.lastIndex;
        if (m[1]) {
            const base = prefixed[m[1][1].toLowerCase()];
            const digits = m[1].slice(2).replace(DIGIT_GROUPING, "");
            out.push({ number: Number.parseInt(digits, base), base });
        } else if (m[2]) {
            const plain = m[2].replace(DECIMAL_GROUPING, "");
            out.push({ number: Number.parseFloat(plain), base: 10 });
        } else if (m[3]) {
            out.push({ name: m[3].toLowerCase() });
        } else {
            const op = { "\u00d7": "*", "\u00f7": "/", "**": "^" }[m[4]] ?? m[4];
            out.push({ op });
        }
    }
    return out;
}

// The functions below read a sum from a reader, { list, at }: the tokens,
// and how many have been read. This one says whether the next is `op`.
function isOp(reader, op) {
    const t = reader.list[reader.at];
    return t && t.op === op;
}

// The token to be read next, read.
function next(reader) {
    const t = reader.list[reader.at];
    reader.at += 1;
    return t;
}

// Reads the bracket closing a sum, which has to be there.
function closing(reader) {
    if (!isOp(reader, ")")) {
        throw new Error("unclosed");
    }
    reader.at += 1;
}

// A number, a bracketed sum, a constant, or a function of the bracketed sum
// after its name.
function primary(reader) {
    const t = next(reader);
    if (!t) {
        throw new Error("short");
    }
    if (t.number !== undefined) {
        return t.number;
    }
    if (t.op === "(") {
        const inner = sum(reader);
        closing(reader);
        return inner;
    }
    // An operator has no name, which neither table holds.
    if (constants[t.name] !== undefined) {
        return constants[t.name];
    }
    if (functions[t.name] && isOp(reader, "(")) {
        reader.at += 1;
        const inner = sum(reader);
        closing(reader);
        return functions[t.name](inner);
    }
    throw new Error("unknown");
}

// A power, with any signs before it.
function unary(reader) {
    if (isOp(reader, "-") || isOp(reader, "+")) {
        const { op } = next(reader);
        return op === "-" ? -unary(reader) : unary(reader);
    }
    return power(reader);
}

// A primary raised to what follows ^, which binds to the right.
function power(reader) {
    const base = primary(reader);
    if (isOp(reader, "^")) {
        reader.at += 1;
        return base ** unary(reader);
    }
    return base;
}

// Signed powers multiplied, divided and taken remainders of.
function product(reader) {
    let value = unary(reader);
    while (isOp(reader, "*") || isOp(reader, "/") || isOp(reader, "%")) {
        const { op } = next(reader);
        const right = unary(reader);
        if (op === "*") {
            value *= right;
        } else {
            value = op === "/" ? value / right : value % right;
        }
    }
    return value;
}

// Products added and taken away.
function sum(reader) {
    let value = product(reader);
    while (isOp(reader, "+") || isOp(reader, "-")) {
        const { op } = next(reader);
        const right = product(reader);
        value = op === "+" ? value + right : value - right;
    }
    return value;
}

// Reads a sum by precedence: + and - below * / %, below ^, which binds to
// the right, below a sign and a function's argument.
function parse(list) {
    const reader = { list, at: 0 };
    const value = sum(reader);
    if (reader.at !== list.length) {
        throw new Error("left over");
    }
    return value;
}

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

// A whole number in a base, with its prefix: 255 in 16 as "0xff".
function inBase(value, base) {
    const sign = value < 0 ? "-" : "";
    return sign + prefixes[base] + Math.abs(value).toString(base);
}

// What's typed, answered: { value, headline, copy, rows: [{ text, side,
// copy }] }, or null when it isn't plainly a sum or a base. `options` gives
// decimals and the grouping and decimal marks. What's copied is never
// grouped, so it pastes as a number.
function answer(text, options) {
    const settings = options ?? {};
    const show = number => format(number, settings.decimals, settings.group, settings.point);
    const { asked, into } = wanting(text.trim());
    const list = tokens(asked);
    if (!list || list.length === 0) {
        return null;
    }
    const based = list.some(t => t.base !== undefined && t.base !== 10);
    if (!plainly(list, based, into)) {
        return null;
    }
    const value = valued(list);
    if (value === null) {
        return null;
    }
    const whole = Number.isInteger(value) && Math.abs(value) <= Number.MAX_SAFE_INTEGER;
    if (into !== null && !whole) {
        return null;
    }
    const bare = format(value, settings.decimals, "", settings.point);
    const headline = into !== null && into !== 10 ? inBase(value, into) : show(value);
    const rows = whole && (based || into !== null) ? inBases(value, into, show, bare) : [];
    return { value, headline, copy: into !== null && into !== 10 ? headline : bare, rows };
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

// Whether the tokens are plainly a sum or a base, `based` saying whether a
// number in them is written in one and `into` the base asked for, if any.
function plainly(list, based, into) {
    const operators = list.filter(t => t.op !== undefined && t.op !== "(" && t.op !== ")");
    const named = list.some(t => t.name !== undefined);
    // A lone number is an answer only when it's written in a base or asked
    // for in one; a lone name, such as "pi", only with an operator.
    if (operators.length === 0 && !based && into === null && !(named && list.length > 1)) {
        return false;
    }
    // "-5" alone is a negative number, not a sum.
    const negative = operators.length === 1 && list.length === 2 && list[0].op === "-";
    return !(negative && !based && into === null);
}

// The sum the tokens make, or null when they don't make a number.
function valued(list) {
    try {
        const value = parse(list);
        return typeof value === "number" && !Number.isNaN(value) ? value : null;
    } catch (_error) {
        return null;
    }
}

// The rows of a whole number in each base but the one it's answered in,
// `show` writing it in decimal and `bare` as it's copied.
function inBases(value, into, show, bare) {
    return shownBases.filter(base => base !== (into ?? 10)).map(base => (base === 10
        ? { text: show(value), side: names[base], copy: bare }
        : { text: inBase(value, base), side: names[base], copy: inBase(value, base) }));
}
