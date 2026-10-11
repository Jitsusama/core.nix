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
const names = { 16: "hex", 8: "octal", 2: "binary", 10: "decimal" };

const functions = {
    sqrt: Math.sqrt, abs: Math.abs, round: Math.round, floor: Math.floor, ceil: Math.ceil,
    ln: Math.log, log: Math.log10, log2: Math.log2, exp: Math.exp,
    sin: Math.sin, cos: Math.cos, tan: Math.tan, asin: Math.asin, acos: Math.acos, atan: Math.atan
};
const constants = { pi: Math.PI, e: Math.E, tau: 2 * Math.PI };

// The pieces of what's typed: numbers, with the base they were written in,
// names, and operators. Null when anything else is in it.
function tokens(text) {
    const out = [];
    const spelled = new RegExp([
        "\\s*(?:(0x[0-9a-f_]+|0o[0-7_]+|0b[01_]+)",
        "(\\d[\\d_,]*(?:\\.\\d+)?(?:e[+-]?\\d+)?|\\.\\d+)",
        "([a-z]+)",
        "(\\*\\*|[-+*/%^()\u00d7\u00f7]))"
    ].join("|"), "iy");
    let at = 0;
    while (at < text.length) {
        spelled.lastIndex = at;
        const m = spelled.exec(text);
        if (!m) {
            if (text.slice(at).trim() === "")
                break;
            return null;
        }
        at = spelled.lastIndex;
        if (m[1]) {
            const base = { x: 16, o: 8, b: 2 }[m[1][1].toLowerCase()];
            out.push({ number: parseInt(m[1].slice(2).replace(/_/g, ""), base), base: base });
        } else if (m[2]) {
            const plain = m[2].replace(/[_,]/g, "");
            out.push({ number: parseFloat(plain), base: 10 });
        } else if (m[3]) {
            out.push({ name: m[3].toLowerCase() });
        } else {
            const op = { "\u00d7": "*", "\u00f7": "/", "**": "^" }[m[4]] ?? m[4];
            out.push({ op: op });
        }
    }
    return out;
}

// Reads a sum by precedence: + and - below * / %, below ^, which binds to
// the right, below a sign and a function's argument.
function parse(list) {
    let at = 0;
    const peek = () => list[at];
    const isOp = op => peek() && peek().op === op;

    function primary() {
        const t = list[at++];
        if (!t)
            throw "short";
        if (t.number !== undefined)
            return t.number;
        if (t.op === "(") {
            const inner = sum();
            if (!isOp(")"))
                throw "unclosed";
            at++;
            return inner;
        }
        if (t.name !== undefined) {
            if (constants[t.name] !== undefined)
                return constants[t.name];
            if (functions[t.name] && isOp("(")) {
                at++;
                const inner = sum();
                if (!isOp(")"))
                    throw "unclosed";
                at++;
                return functions[t.name](inner);
            }
        }
        throw "unknown";
    }
    function unary() {
        if (isOp("-")) {
            at++;
            return -unary();
        }
        if (isOp("+")) {
            at++;
            return unary();
        }
        return power();
    }
    function power() {
        const base = primary();
        if (isOp("^")) {
            at++;
            return Math.pow(base, unary());
        }
        return base;
    }
    function product() {
        let value = unary();
        while (isOp("*") || isOp("/") || isOp("%")) {
            const op = list[at++].op;
            const right = unary();
            value = op === "*" ? value * right : op === "/" ? value / right : value % right;
        }
        return value;
    }
    function sum() {
        let value = product();
        while (isOp("+") || isOp("-")) {
            const op = list[at++].op;
            const right = product();
            value = op === "+" ? value + right : value - right;
        }
        return value;
    }
    const value = sum();
    if (at !== list.length)
        throw "left over";
    return value;
}

// A number grouped in threes, with at most `decimals` places, trailing
// zeros dropped: 1234567.891 as "1,234,567.891".
function format(value, decimals, group, point) {
    if (!isFinite(value))
        return isNaN(value) ? "not a number" : value > 0 ? "\u221e" : "-\u221e";
    const places = decimals ?? 10;
    const sign = value < 0 ? "-" : "";
    const exact = Math.abs(value).toFixed(places);
    const fixed = exact.includes(".") ? exact.replace(/\.?0+$/, "") : exact;
    const [whole, fraction] = fixed.split(".");
    const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, group ?? ",");
    return sign + grouped + (fraction ? (point ?? ".") + fraction : "");
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
    const show = value => format(value, settings.decimals, settings.group, settings.point);
    const typed = text.trim();
    if (typed === "")
        return null;
    let asked = typed;
    let into = null;
    const wanted = /^(.*\S)\s+(?:in|to|as)\s+([a-z]+)$/i.exec(typed);
    if (wanted && bases[wanted[2].toLowerCase()]) {
        asked = wanted[1];
        into = bases[wanted[2].toLowerCase()];
    }
    const list = tokens(asked);
    if (!list || list.length === 0)
        return null;
    const operators = list.filter(t => t.op !== undefined && t.op !== "(" && t.op !== ")");
    const based = list.some(t => t.base !== undefined && t.base !== 10);
    const named = list.some(t => t.name !== undefined);
    // A lone number is an answer only when it's written in a base or asked
    // for in one; a lone name, such as "pi", only with an operator.
    if (operators.length === 0 && !based && into === null && !(named && list.length > 1))
        return null;
    // "-5" alone is a negative number, not a sum.
    const negative = operators.length === 1 && list.length === 2 && list[0].op === "-";
    if (negative && !based && into === null)
        return null;
    let value;
    try {
        value = parse(list);
    } catch (e) {
        return null;
    }
    if (typeof value !== "number" || isNaN(value))
        return null;
    const whole = Number.isInteger(value) && Math.abs(value) <= Number.MAX_SAFE_INTEGER;
    if (into !== null && !whole)
        return null;
    const bare = format(value, settings.decimals, "", settings.point);
    const headline = into !== null && into !== 10 ? inBase(value, into) : show(value);
    const rows = [];
    if (whole && (based || into !== null))
        for (const base of [10, 16, 8, 2])
            if (base !== (into ?? 10))
                rows.push(base === 10
                    ? { text: show(value), side: names[base], copy: bare }
                    : { text: inBase(value, base), side: names[base], copy: inBase(value, base) });
    return {
        value: value,
        headline: headline,
        copy: into !== null && into !== 10 ? headline : bare,
        rows: rows
    };
}
