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
    const plain = text.replace(unbroken, " ");
    const looked = byCase ? plain : plain.toLowerCase();
    const sought = (byCase ? pattern : pattern.toLowerCase()).replace(unbroken, " ");
    const spans = [];
    let from = looked.indexOf(sought);
    while (from >= 0) {
        spans.push([from, from + sought.length]);
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
