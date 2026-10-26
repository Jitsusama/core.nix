// Characters and their codes for the launcher's answers, in plain
// JavaScript so a check can run it in Qt's own engine, as the shell does.
//
// What's asked is read whole or not at all: "char 65" or "ascii 65" is A,
// "char A" or "ord A" is 65, "0x41 as text" and "U+1F600" are the
// characters they number, and "utf8 é" is é's bytes. Each says the rest of
// a character's forms as parts, { id, side, copy }, named by their ids in
// the vocabulary (words.json's characters:*): its number in decimal, hex
// and binary, its code point, its name and its UTF-8. Several characters
// are a row each, { code, name, side, note, parts }, holding the parts.
// Names are Unicode's, as names.py writes them, read by prepare(); without
// them a character is told by its numbers alone.
.pragma library

// The highest code point, the highest of the first plane, which UTF-16
// writes in one unit, and the surrogates, which only UTF-16 uses.
const LAST = 0x10_ff_ff;
const FIRST_PLANE = 0xff_ff;
const SURROGATES = 0xd8_00;
const SURROGATES_END = 0xdf_ff;
// How many characters one question asks after, so a long one stays a list.
const MOST = 64;
// The bases a character's number is written in.
const HEX = 16;
const BINARY = 2;
const OCTAL = 8;
const DECIMAL = 10;
// The bases of NUMBER's groups, in order.
const WRITTEN = [HEX, OCTAL, BINARY, HEX, DECIMAL];
// The digits a byte is written with in hex.
const BYTE_DIGITS = 2;
// The digits a code point is written with at least, and at most.
const POINT_DIGITS = 4;
const MOST_DIGITS = 6;
// How near a search of the names comes to a character's line before it
// reads along, and how long a line can be, in the text's units.
const NEAR = 256;
const LONGEST_LINE = 160;
// The controls and the space, drawn as their pictures from U+2400 on, and
// delete, which has its own.
const PICTURED = 0x21;
const PICTURES = 0x24_00;
const DELETE = 0x7f;
const DELETE_PICTURE = 0x24_21;
// What a combining mark is drawn on, so it shows.
const DOTTED_CIRCLE = "\u25cc";
// The categories of a mark drawn on what comes before it.
const MARKS = ["Mn", "Me"];
// The first code point UTF-8 writes in two bytes, in three and in four,
// and the lead byte of each length; and what each byte after the lead
// starts with and carries.
const TWO_BYTES = 0x80;
const THREE_BYTES = 0x8_00;
const FOUR_BYTES = 0x1_00_00;
const UTF8_LIMITS = [TWO_BYTES, THREE_BYTES, FOUR_BYTES];
const LEAD_TWO = 0xc0;
const LEAD_THREE = 0xe0;
const LEAD_FOUR = 0xf0;
const UTF8_LEADS = [0, LEAD_TWO, LEAD_THREE, LEAD_FOUR];
const CONTINUATION = 0x80;
const CARRIED = 64;
// The bits a byte is written in.
const BYTE_BITS = 8;
// Hangul's syllables, named by their parts: the first and last, and the
// leading consonants, vowels and trailing consonants, in Unicode's order.
const HANGUL_FIRST = 0xac_00;
const HANGUL_LAST = 0xd7_a3;
const LEADS = ["g", "gg", "n", "d", "dd", "r", "m", "b", "bb", "s", "ss", "", "j", "jj", "c",
    "k", "t", "p", "h"];
const VOWELS = ["a", "ae", "ya", "yae", "eo", "e", "yeo", "ye", "o", "wa", "wae", "oe", "yo",
    "u", "weo", "we", "wi", "yu", "eu", "yi", "i"];
const TAILS = ["", "g", "gg", "gs", "n", "nj", "nh", "d", "l", "lg", "lm", "lb", "ls", "lt",
    "lp", "lh", "m", "b", "bs", "s", "ss", "ng", "j", "c", "k", "t", "p", "h"];

// A question led by its word: "char 65", "ord A", "utf8 é". One space
// parts the word from what it asks, so "ord " and a space asks the space.
const LED = /^\s*(ascii|char|chr|ord|utf-?8)\s([\s\S]+)$/i;
// The words that ask for bytes.
const BYTES = /^utf-?8$/i;
// Numbers asked for as the characters they are: "0x41 as text".
const AS_TEXT = /^(.+?)\s+(?:as|in|to)\s+(?:text|chars?|characters?|ascii|unicode)$/i;
// Code points alone, written as Unicode writes them: "U+1F600".
const POINTS = /^(?:u\+[0-9a-f]{4,6}[\s,]*)+$/i;
// One number, in a base or as a code point.
const NUMBER = /^(?:0x([0-9a-f]+)|0o([0-7]+)|0b([01]+)|u\+([0-9a-f]{4,6})|(\d+))$/i;
// What parts numbers written together.
const BETWEEN = /[\s,]+/;

// The names names.py wrote, ready to be looked up: { text, version, runs },
// the runs being the characters named by their code point. A character's
// line is found by its code point after a line break, so the text is kept
// whole rather than split into thousands of entries.
function prepare(text) {
    const names = { text: text ?? "", version: "", runs: [], start: 0 };
    let at = 0;
    while (at < names.text.length) {
        const end = names.text.indexOf("\n", at);
        const fields = names.text.slice(at, end < 0 ? names.text.length : end).split("\t");
        if (fields[0] === "unicode") {
            names.version = fields[1];
        } else if (fields[0] === "range") {
            names.runs.push({ first: Number.parseInt(fields[1], HEX),
                last: Number.parseInt(fields[2], HEX), shared: fields[3] });
        } else {
            break;
        }
        at = end < 0 ? names.text.length : end + 1;
    }
    names.start = at;
    return names;
}

// A code point as Unicode writes it, in four digits or more: "0041".
function digits(code) {
    const hex = code.toString(HEX);
    return hex.length < POINT_DIGITS ? `000${hex}`.slice(-POINT_DIGITS) : hex;
}

function hangul(code) {
    const index = code - HANGUL_FIRST;
    const perLead = VOWELS.length * TAILS.length;
    const lead = LEADS[Math.floor(index / perLead)];
    const vowel = VOWELS[Math.floor((index % perLead) / TAILS.length)];
    return `hangul syllable ${lead}${vowel}${TAILS[index % TAILS.length]}`;
}

// The stretch of the names holding a character's line, if it has one,
// with the line break before it: the lines are in order, so the search
// halves the text until it's near, rather than reading a megabyte.
function nearLine(code, names) {
    const { text } = names;
    let low = names.start;
    let high = text.length;
    while (high - low > NEAR) {
        const middle = Math.floor((low + high) / 2);
        const start = text.indexOf("\n", middle) + 1;
        if (start === 0 || start >= high) {
            high = middle;
        } else if (Number.parseInt(text.slice(start, start + MOST_DIGITS + 1), HEX) <= code) {
            low = start;
        } else {
            high = start;
        }
    }
    return text.slice(Math.max(low - 1, 0), high + LONGEST_LINE);
}

// A character's name and, for one not drawn by itself, its category:
// { name, category }, or null when it has none or no names were read.
function lookup(code, names) {
    if (names.text === "") {
        return null;
    }
    if (code >= HANGUL_FIRST && code <= HANGUL_LAST) {
        return { name: hangul(code), category: "" };
    }
    const run = names.runs.find(each => code >= each.first && code <= each.last);
    if (run) {
        return { name: `${run.shared}${digits(code)}`, category: "" };
    }
    const near = nearLine(code, names);
    const key = `\n${code.toString(HEX)}\t`;
    const at = near.indexOf(key);
    if (at < 0) {
        return null;
    }
    const end = near.indexOf("\n", at + 1);
    const fields = near.slice(at + key.length, end < 0 ? near.length : end).split("\t");
    return { name: fields[0], category: fields[1] ?? "" };
}

// How a character is shown: a control or the space as its picture, a mark
// on a dotted circle, one that isn't drawn by its name, the rest as itself.
function shownAs(code, named) {
    if (code < PICTURED) {
        return String.fromCodePoint(PICTURES + code);
    }
    if (code === DELETE) {
        return String.fromCodePoint(DELETE_PICTURE);
    }
    const category = named ? named.category : "";
    if (MARKS.includes(category)) {
        return DOTTED_CIRCLE + String.fromCodePoint(code);
    }
    return category === "" ? String.fromCodePoint(code) : named.name;
}

// A character's UTF-8, as its bytes: the lead, then six bits a byte.
function utf8(code) {
    const index = UTF8_LIMITS.findIndex(limit => code < limit);
    const after = index < 0 ? UTF8_LIMITS.length : index;
    const bytes = [];
    let rest = code;
    for (let i = 0; i < after; i += 1) {
        bytes.unshift(CONTINUATION + (rest % CARRIED));
        rest = Math.floor(rest / CARRIED);
    }
    bytes.unshift(UTF8_LEADS[after] + rest);
    return bytes;
}

function byteHex(byte) {
    return `0${byte.toString(HEX)}`.slice(-BYTE_DIGITS);
}

function byteBits(byte) {
    return `0000000${byte.toString(BINARY)}`.slice(-BYTE_BITS);
}

function point(code) {
    return `U+${digits(code).toUpperCase()}`;
}

// A character's parts, each named by its id, and what's shown of it.
function told(code, names) {
    const named = lookup(code, names);
    const parts = [
        { id: "decimal", side: String(code) },
        { id: "hex", side: `0x${code.toString(HEX)}` },
        { id: "binary", side: `0b${code.toString(BINARY)}` },
        { id: "point", side: point(code) }
    ];
    if (named) {
        parts.push({ id: "name", side: named.name });
    }
    parts.push({ id: "utf8", side: utf8(code).map(byteHex).join(" ") });
    return {
        shown: shownAs(code, named),
        name: named ? named.name : "",
        parts: parts.map(part => ({ id: part.id, side: part.side, copy: part.side }))
    };
}

function noted(names, said) {
    return said && names.version !== "" ? `names from Unicode ${names.version}` : "";
}

// One character, its headline its number (`decimal`) or itself.
function one(code, names, headed) {
    const said = told(code, names);
    const decimal = headed === "decimal";
    return {
        headline: decimal ? String(code) : said.shown,
        copy: decimal ? String(code) : String.fromCodePoint(code),
        rows: said.parts.filter(part => part.id !== headed),
        note: noted(names, said.name !== "")
    };
}

// Several characters, a row each holding its parts, the headline their
// numbers (`decimal`) or the text they make.
function several(codes, names, headed) {
    const rows = codes.map(code => {
        const said = told(code, names);
        return { code, name: said.shown, side: point(code), note: said.name, parts: said.parts };
    });
    const text = codes.map(code => String.fromCodePoint(code)).join("");
    const decimal = headed === "decimal";
    return {
        headline: decimal ? codes.join(" ") : rows.map(row => row.name).join(""),
        copy: decimal ? codes.join(" ") : text,
        rows,
        note: noted(names, rows.some(row => row.note !== ""))
    };
}

function characters(codes, names, headed) {
    if (codes.length === 0 || codes.length > MOST) {
        return null;
    }
    return codes.length === 1 ? one(codes[0], names, headed) : several(codes, names, headed);
}

// A number as written, or null: decimal, 0x, 0o, 0b or U+.
function numbered(number) {
    const m = NUMBER.exec(number);
    if (!m) {
        return null;
    }
    const group = m.slice(1).findIndex(each => each !== undefined);
    return Number.parseInt(m[group + 1], WRITTEN[group]);
}

// The numbers written together, as written.
function written(text) {
    return text.trim().split(BETWEEN).filter(each => each !== "");
}

// Whether what follows a question's word is written as numbers, rather
// than as the characters themselves.
function numberish(text) {
    const all = written(text);
    return all.length > 0 && all.every(each => NUMBER.test(each));
}

// The code points numbers written together stand for, or null when one
// isn't a number or isn't a character.
function numbers(text) {
    const codes = written(text).map(numbered);
    const valid = code => code !== null && code <= LAST
        && !(code >= SURROGATES && code <= SURROGATES_END);
    return codes.length > 0 && codes.every(valid) ? codes : null;
}

// The code points a text is made of.
function points(text) {
    const codes = [];
    let at = 0;
    while (at < text.length) {
        const code = text.codePointAt(at);
        codes.push(code);
        // One past the first plane takes two of the text's units.
        at += code > FIRST_PLANE ? 2 : 1;
    }
    return codes;
}

// A text's UTF-8: its bytes, escaped, percent-encoded and in bits, the
// code points it's made of, and how many bytes.
function encoded(codes) {
    if (codes.length === 0 || codes.length > MOST) {
        return null;
    }
    const bytes = codes.reduce((all, code) => all.concat(utf8(code)), []);
    const hex = bytes.map(byteHex);
    const parts = [
        { id: "escaped", side: hex.map(byte => `\\x${byte}`).join("") },
        { id: "percent", side: hex.map(byte => `%${byte.toUpperCase()}`).join("") },
        { id: "bits", side: bytes.map(byteBits).join(" ") },
        { id: "points", side: codes.map(point).join(" ") },
        { id: "length", side: `${bytes.length} ${bytes.length === 1 ? "byte" : "bytes"}`,
            copy: String(bytes.length) }
    ];
    return {
        headline: hex.join(" "),
        copy: hex.join(" "),
        rows: parts.map(part => ({ id: part.id, side: part.side, copy: part.copy ?? part.side })),
        note: ""
    };
}

// What follows a question's word: itself if it's only spaces, as a space
// is asked after, or with the spaces round it gone.
function argument(rest) {
    return rest.trim() === "" ? rest : rest.trim();
}

// Whether what's typed could be a question of characters, cheaply, so the
// names are read only for one.
function asked(typed) {
    const text = typed.trim();
    return LED.test(typed) || AS_TEXT.test(text) || POINTS.test(text);
}

// What's typed, answered: { headline, copy, rows, note }, or null when it
// isn't plainly a question of characters. `table` is what prepare() made
// of the names, or nothing, for none.
function answer(typed, table) {
    const names = table ?? prepare("");
    const led = LED.exec(typed);
    if (led) {
        const rest = argument(led[2]);
        if (BYTES.test(led[1])) {
            return encoded(points(rest));
        }
        // A number is the character it numbers, unless ord asks its own.
        if (led[1].toLowerCase() !== "ord" && numberish(rest)) {
            const codes = numbers(rest);
            return codes ? characters(codes, names, "character") : null;
        }
        return characters(points(rest), names, "decimal");
    }
    const text = typed.trim();
    const spelled = AS_TEXT.exec(text);
    const codes = spelled ? numbers(spelled[1]) : null;
    if (codes) {
        return characters(codes, names, "character");
    }
    return POINTS.test(text) ? characters(numbers(text), names, "character") : null;
}
