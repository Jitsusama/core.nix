// How a drawer's prose is cut to the lines it shows, the rest left to its
// fold, read apart from the drawing (Drawer.qml) so tests/query-keys.qml
// can hold it in Qt's own engine.
.pragma library

const ELLIPSIS = "\u2026";
const ENTITIES = Object.freeze({
    amp: "&",
    apos: "'",
    gt: ">",
    lt: "<",
    nbsp: "\u00a0",
    quot: "\""
});
// Tags that hold nothing, so nothing closes them.
const EMPTY_TAGS = new Set(["br", "hr", "img"]);
const TAG = /^<\/?([a-zA-Z0-9]+)[^>]*>/;
const ENTITY = /^&(#?\w+);/;
const TAGS = /<(\/?)([a-zA-Z0-9]+)[^>]*>/g;
const WORDS = /\S+/g;

// A drawer's prose, `{ text, styled }`, as `most` lines at most no wider
// than `wide` by `width(text)`, laid as Qt wraps words, at spaces, a word
// wider than a line standing alone. Where it runs past, `said` holds what
// the lines hold, cut after the last word the last line has room for with
// the ellipsis that says there's more, and `cut` is true; the rest is the
// fold's (VKCF3Z's wrap rule 4, 76ba12d0). A styled text is measured by
// its letters, a <br> a line's end, and cut where they end, the tags still
// open there closed. A last word with no room beside it is kept, as one
// word runs past the edge rather than split.
function lines(prose, wide, most, width) {
    const text = String(prose.text ?? "");
    const letters = prose.styled === true ? lettersOf(text) : { plain: text, ends: null };
    // A line past the most says whether it runs past; none further is laid,
    // since each word laid is measured in Qt, and a notice of 34 kB laid
    // whole held its drawer 3.2 s (2026-11-25).
    const laidOut = laid(letters.plain, wide, width, most + 1);
    if (laidOut.length <= most) {
        return { said: text, cut: false };
    }
    const end = lastEnd(letters.plain, laidOut[most - 1], wide, width);
    if (letters.ends === null) {
        return { said: `${text.slice(0, end)}${ELLIPSIS}`, cut: true };
    }
    const kept = text.slice(0, letters.ends[end - 1]);
    return { said: `${kept}${ELLIPSIS}${closing(kept)}`, cut: true };
}

// Where the last line shown ends: after its last word that leaves room for
// the ellipsis, or its first word where none does.
function lastEnd(plain, line, wide, width) {
    let end = line.end;
    for (let at = line.spaces.length - 1; at >= 0; at -= 1) {
        if (width(`${plain.slice(line.start, end)}${ELLIPSIS}`) <= wide) {
            return end;
        }
        end = line.spaces[at];
    }
    return end;
}

// The most an entity takes, its & and ; with them, as the longest named
// one here and a numbered one do.
const ENTITY_MOST = 12;

// One letter of a styled text from `at`: what it draws, a tag nothing or a
// <br> a line's end, and how much of the text it takes. Only a tag's or an
// entity's own letters are read past `at`, never the rest of the text, so
// a long one is read through once rather than once a letter.
function letterAt(text, at) {
    const first = text[at];
    const closes = first === "<" ? text.indexOf(">", at) : -1;
    const tag = closes < 0 ? null : TAG.exec(text.slice(at, closes + 1));
    if (tag) {
        return { drawn: tag[1].toLowerCase() === "br" ? "\n" : "", taken: tag[0].length };
    }
    const entity = first === "&" ? ENTITY.exec(text.slice(at, at + ENTITY_MOST)) : null;
    if (!entity) {
        return { drawn: first, taken: 1 };
    }
    const named = entity[1];
    const drawn = named.startsWith("#") ? String.fromCharCode(Number(named.slice(1)))
        : ENTITIES[named] ?? entity[0];
    return { drawn, taken: entity[0].length };
}

// A styled text's letters as drawn, and where in the text each ends.
function lettersOf(text) {
    const plain = [];
    const ends = [];
    for (let at = 0; at < text.length;) {
        const letter = letterAt(text, at);
        at += letter.taken;
        for (const drawn of letter.drawn) {
            plain.push(drawn);
            ends.push(at);
        }
    }
    return { plain: plain.join(""), ends };
}

// The lines plain text wraps in, each where it starts and ends and where
// the spaces between its words begin, a paragraph's end ending its line
// and an empty paragraph a line of its own: `most` of them at most, those
// before the rest, and every one where it's not given.
function laid(plain, wide, width, most = Infinity) {
    const laidOut = [];
    let from = 0;
    for (const paragraph of plain.split("\n")) {
        let line = null;
        WORDS.lastIndex = 0;
        for (let word = WORDS.exec(paragraph); word; word = WORDS.exec(paragraph)) {
            const start = from + word.index;
            const end = start + word[0].length;
            if (line && width(plain.slice(line.start, end)) <= wide) {
                line.spaces.push(line.end);
                line.end = end;
            } else {
                if (line) {
                    laidOut.push(line);
                }
                if (laidOut.length >= most) {
                    return laidOut;
                }
                line = { start, end, spaces: [] };
            }
        }
        laidOut.push(line ?? { start: from, end: from, spaces: [] });
        if (laidOut.length >= most) {
            return laidOut;
        }
        from += paragraph.length + 1;
    }
    return laidOut;
}

// The closing tags of those a cut styled text leaves open, the last first.
function closing(kept) {
    const open = [];
    TAGS.lastIndex = 0;
    for (let tag = TAGS.exec(kept); tag; tag = TAGS.exec(kept)) {
        const name = tag[2].toLowerCase();
        if (EMPTY_TAGS.has(name) || tag[0].endsWith("/>")) {
            continue;
        }
        if (tag[1] === "") {
            open.push(name);
        } else if (open.lastIndexOf(name) >= 0) {
            open.splice(open.lastIndexOf(name), 1);
        }
    }
    return open.reverse().map(name => `</${name}>`).join("");
}
