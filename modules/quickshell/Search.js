// How the shell's lists are searched as Joel's Neovim searches a buffer
// (docs/design.md, Vi's Keys): a pattern typed after / or ? is found as it's
// written, never as a regular expression, so every list searches alike and
// nothing is compiled per key, ignoring case until it has a capital, as his
// ignorecase and smartcase do, and each place it's found is washed as his
// Search draws it. The notices' stack (Notifications.qml) and the launcher's
// rows (Launcher.qml) both search through it, and tests/notice-keys.qml and
// tests/query-keys.qml hold it in Qt's own engine.
.pragma library

const ampersand = /&/g;
const opening = /</g;
const closing = />/g;
const newline = /\n/g;

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
    const looked = byCase ? text : text.toLowerCase();
    const sought = byCase ? pattern : pattern.toLowerCase();
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

// A line as styled text, each place the pattern's found in the wash's
// ink, its ground left to be drawn behind it (Wash.qml): Qt elides styled
// text, as a row cut to its room needs, but not rich text, and only rich
// text draws a ground.
function inked(text, pattern, ink) {
    let at = 0;
    let styled = "";
    for (const [start, end] of found(pattern, text)) {
        styled += escaped(text.slice(at, start))
            + `<font color="${ink}">${escaped(text.slice(start, end))}</font>`;
        at = end;
    }
    return styled + escaped(text.slice(at));
}

// The rows' words, each row's as one text, as they're searched: as they
// are, for a pattern with a capital, and lowered, for one without, letter
// for letter. Made once for a list of rows, so each letter of a search
// typed is only a pass of includes over them. Lowering is most of the
// cost, so a list whose words are lowered already, as the launcher's names
// are for its ranking (Fuzzy.js's folded), gives them.
function searchable(texts, lowered) {
    return { texts, lowered: lowered ?? texts.map(text => text.toLowerCase()) };
}

// Which rows hold a pattern, from their words as searchable() makes them,
// in order: their places, counted from 0.
function holding(rows, pattern) {
    if (pattern === "") {
        return [];
    }
    const byCase = cased(pattern);
    const texts = byCase ? rows.texts : rows.lowered;
    const sought = byCase ? pattern : pattern.toLowerCase();
    const holds = [];
    for (let at = 0; at < texts.length; at++) {
        if (texts[at].includes(sought)) {
            holds.push(at);
        }
    }
    return holds;
}

// Where a search lands from a row, given the rows holding its pattern in
// order: the count'th of them after the row going forward, or before it
// going back, round past the last to the first, as Vim's wrapscan goes,
// so the row itself is found last. Whether it went round to get there is
// said, for the foot to say as Neovim says "search hit BOTTOM". Null when
// no row holds it. A count of 0 is 1.
function landing(holds, from, forward, count) {
    if (holds.length === 0) {
        return null;
    }
    // How many of the rows holding it lie past the row, the way it goes.
    const ahead = forward ? holds.filter(at => at > from).length
        : holds.filter(at => at < from).length;
    const times = Math.max(1, count);
    // Stepping one by one round the list, the times'th step from the row:
    // ahead of it first, then round from the other end.
    const step = (times - 1) % holds.length;
    const first = forward ? holds.length - ahead : ahead - 1;
    const index = forward ? (first + step) % holds.length
        : ((first - step) % holds.length + holds.length) % holds.length;
    return { at: holds[index], wrapped: times > ahead };
}
