// How the drawer reads the texts Qt laid out as the lines its page moves
// on (DrawerPage.js), and where each line ends or runs on into the next,
// read apart from the drawing (Drawer.qml) so tests/query-keys.qml can hold
// it in Qt's own engine.
.pragma library

// How many paragraphs words hold, as the drawer's foot counts those it
// hides, never their lines (VKCF3Z, 2026-11-05). Each line another program
// broke is one, which Qt keeps in a pre-wrap block as a line separator, and
// so is each a styled text breaks; a line Qt wraps is part of its own, and
// a blank one is none.
const PARAGRAPH_BREAK = /[\n\u2028\u2029]/;
function paragraphs(text) {
    return text.split(PARAGRAPH_BREAK).filter(paragraph => paragraph.trim() !== "").length;
}

// What a line Qt wrapped breaks at, which a search runs on across, from
// all it holds up to the next line, `raw`, past its `kept` letters: the
// space it broke at, or "" at a zero-width space or inside a word, or
// null where it ends a paragraph or a line another program broke. A line
// the drawer broke itself, as it lays a run, an identifier or an item's
// value, ends at a break too, so a search stops there as the wash does.
function runsOn(raw, kept) {
    const rest = raw.slice(kept);
    return PARAGRAPH_BREAK.test(rest) ? null : rest;
}

// Pieces as lines, top to bottom, those sharing a baseline one line. A
// line runs on into the next, as its last piece's `on` says, only where
// the next is one piece of the same words: a fact's value runs on from
// the line it shares with its name, but never into a line another text
// shares, nor a search from one text into another but along a line.
function linesOf(pieces) {
    pieces.sort((one, other) => one.base - other.base || one.x - other.x);
    const found = [];
    const lineOf = piece => ({ base: piece.base, top: piece.top, height: piece.height,
        caps: piece.caps, length: piece.length, on: piece.on ?? null, parts: [piece] });
    for (const piece of pieces) {
        const line = found[found.length - 1];
        const together = line && Math.abs(line.base - piece.base) < 2;
        // A blank piece on another's baseline is no part of its line,
        // which would take a letter the eye can't see: a blank line
        // is a line only where it stands alone.
        if (together && piece.length === 0) {
            continue;
        }
        if (together && line.length === 0) {
            found[found.length - 1] = lineOf(piece);
        } else if (together) {
            const bottom = Math.max(line.top + line.height, piece.top + piece.height);
            line.top = Math.min(line.top, piece.top);
            line.height = bottom - line.top;
            line.caps = Math.max(line.caps, piece.caps);
            line.length += piece.length;
            line.parts.push(piece);
        } else {
            found.push(lineOf(piece));
        }
    }
    found.forEach((line, at) => {
        const next = found[at + 1];
        const last = line.parts[line.parts.length - 1];
        const into = next && next.parts.length === 1 && next.parts[0].words === last.words;
        line.on = into ? last.on ?? null : null;
    });
    return found;
}
