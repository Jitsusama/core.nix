// How the shell's lists are searched as Joel's Neovim searches a buffer
// (docs/design.md, Vi's Keys): a pattern typed after / or ? is found as it's
// written, never as a regular expression, so every list searches alike and
// nothing is compiled per key, ignoring case until it has a capital, as his
// ignorecase and smartcase do, and each place it's found is washed as his
// Search draws it. The launcher's pages search through it, and
// tests/query-keys.qml holds it in Qt's own engine.
.pragma library

const ampersand = /&/g;
const opening = /</g;
const closing = />/g;
const newline = /\n/g;
// A drawer lays each line of a run that fits with no-break spaces, so Qt
// wraps it only between items; a space typed finds one, or "Pirate flag"
// in Emoji's drawer would be found nowhere. A pattern pasted from one is
// read the same way.
const unbroken = /\u00a0/g;
// It holds a hyphenated word whole with a word joiner after its hyphen,
// and a zero-width space may stand where a line breaks: neither is typed,
// so they're passed over, or /t-shirt would be found nowhere.
const unseen = /[\u2060\u200b]/;

// A text as it's searched: what's passed over taken out, in lower case
// unless `byCase`, with where each of its letters starts and ends in the
// text, so a place found in it is said on the text as it stands. Lower case
// can be longer than the letter it lowers (an I with a dot above is an i
// and a dot), and each of its letters stands where that letter does. Most
// texts hold nothing passed over and lower letter for letter, so they're
// searched as they are, with nothing to map, since a search reads every
// line of a drawer on each key typed.
function seen(text, byCase) {
    const lowered = byCase ? text : text.toLowerCase();
    if (lowered.length === text.length && !unseen.test(text)) {
        return { letters: lowered, starts: null, ends: null };
    }
    let letters = "";
    const starts = [];
    const ends = [];
    let from = 0;
    while (from < text.length) {
        const letter = String.fromCodePoint(text.codePointAt(from));
        if (!unseen.test(letter)) {
            const read = byCase ? letter : letter.toLowerCase();
            letters += read;
            for (let unit = 0; unit < read.length; unit++) {
                starts.push(from);
                ends.push(from + letter.length);
            }
        }
        from += letter.length;
    }
    return { letters, starts, ends };
}

// Whether a pattern is matched by its case: only once it has a capital.
function cased(pattern) {
    return pattern !== pattern.toLowerCase();
}

// Where a pattern is found in a text, as [start, end] each, the first
// first and none overlapping, ignoring case until the pattern has a
// capital, as Joel's smartcase does.
function found(pattern, text) {
    if (pattern === "") {
        return [];
    }
    const byCase = cased(pattern);
    const plain = seen(text, byCase);
    const looked = plain.letters.replace(unbroken, " ");
    const sought = seen(pattern, byCase).letters.replace(unbroken, " ");
    const spans = [];
    if (sought === "") {
        return spans;
    }
    let from = looked.indexOf(sought);
    while (from >= 0) {
        // Its end is where its last letter ends, so a joiner after the
        // place found isn't taken into it.
        const last = from + sought.length - 1;
        spans.push(plain.starts === null
            ? [from, last + 1]
            : [plain.starts[from], plain.ends[last]]);
        from = looked.indexOf(sought, from + sought.length);
    }
    return spans;
}

function escaped(text) {
    return text.replace(ampersand, "&amp;").replace(opening, "&lt;").replace(closing, "&gt;")
        .replace(newline, "<br>");
}

// A text as rich text, each place the pattern's found washed: its ink and
// ground as the wash gives them, as Neovim's Search draws a match.
function washed(text, pattern, wash) {
    let at = 0;
    let html = "";
    for (const [start, end] of found(pattern, text)) {
        html += escaped(text.slice(at, start))
            + `<span style="color:${wash.ink};background-color:${wash.ground}">`
            + `${escaped(text.slice(start, end))}</span>`;
        at = end;
    }
    return html + escaped(text.slice(at));
}
