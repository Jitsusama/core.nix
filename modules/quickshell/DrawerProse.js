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
    const laidOut = laid(letters.plain, wide, width);
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

// One letter of a styled text from `at`: what it draws, a tag nothing or a
// <br> a line's end, and how much of the text it takes.
function letterAt(text, at) {
    const rest = text.slice(at);
    const tag = TAG.exec(rest);
    if (tag) {
        return { drawn: tag[1].toLowerCase() === "br" ? "\n" : "", taken: tag[0].length };
    }
    const entity = ENTITY.exec(rest);
    if (!entity) {
        return { drawn: text[at], taken: 1 };
    }
    const named = entity[1];
    const drawn = named.startsWith("#") ? String.fromCharCode(Number(named.slice(1)))
        : ENTITIES[named] ?? entity[0];
    return { drawn, taken: entity[0].length };
}

// A styled text's letters as drawn, and where in the text each ends.
function lettersOf(text) {
    let plain = "";
    const ends = [];
    for (let at = 0; at < text.length;) {
        const letter = letterAt(text, at);
        at += letter.taken;
        for (const drawn of letter.drawn) {
            plain += drawn;
            ends.push(at);
        }
    }
    return { plain, ends };
}

// The lines plain text wraps in, each where it starts and ends and where
// the spaces between its words begin, a paragraph's end ending its line
// and an empty paragraph a line of its own.
function laid(plain, wide, width) {
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
                line = { start, end, spaces: [] };
            }
        }
        laidOut.push(line ?? { start: from, end: from, spaces: [] });
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
