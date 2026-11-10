// How wide words are in a face, each measured once and kept across the
// launcher's openings, and the words Reading.js fits to a room, kept as
// well. A row is made anew at each opening, and fitting its words measured
// every cut of them again, about a tenth of an opening's thread on the
// laptop (2026-11-10). This is the shell's own drawing, which no daemon
// can do for it, since only the shell that draws the words knows the face
// they're drawn in. Tested by tests/reading.qml.
.pragma library

// How many widths and fits are kept, the oldest let go first: a few
// openings' rows and every cut of their words, so a long run of new words
// never holds the shell's memory.
const MEASURES_KEPT = 4000;
// The widths and fits kept, in plain objects rather than a Map, whose keys
// Qt's engine searches one by one: 4000 of them took 112 ms to set and get
// in a Map, and 5 in an object. The newest half (`latest`) is asked first,
// and once it's full it becomes the oldest (`earlier`), whose last are let
// go, so none is ever deleted one at a time; one asked for again from the
// oldest is kept among the newest. A theme's faces change only with the
// shell's files, which start a new engine, and these with it, so nothing
// is ever read in a face it wasn't measured in.
const measures = { latest: Object.create(null), earlier: Object.create(null), count: 0 };

// What `key` holds, made by `make` the first time it's asked for.
function remembered(key, make) {
    const held = measures.latest[key];
    if (held !== undefined) {
        return held;
    }
    const earlier = measures.earlier[key];
    const made = earlier === undefined ? make() : earlier;
    if (measures.count >= MEASURES_KEPT / 2) {
        measures.earlier = measures.latest;
        measures.latest = Object.create(null);
        measures.count = 0;
    }
    measures.latest[key] = made;
    measures.count += 1;
    return made;
}

// How many widths and fits are kept now, both halves together, which
// tests/reading.qml holds to at most MEASURES_KEPT however many are asked.
function keptCount() {
    return Object.keys(measures.earlier).length + measures.count;
}

// A face as one key: everything of a font that changes how wide words
// are drawn in it.
function faceOf(font) {
    return JSON.stringify([font.family, font.styleName ?? "", font.pointSize ?? -1,
        font.pixelSize ?? -1, font.weight, font.italic === true, font.letterSpacing ?? 0,
        font.wordSpacing ?? 0, font.capitalization ?? 0, font.kerning ?? true,
        font.features ?? null]);
}

// How wide words are in a FontMetrics' face, as its advanceWidth says,
// or with a pixel to spare, rounded up, where `spared`, as Qt elides; each
// measured once and kept. The face is read as the measurer is made, so a
// binding that makes one is made again when its face changes. Its `kept`
// keeps what Reading.js fits in its face by a key naming the fit, the room
// and the words, made by `fitting` the first time.
function measurer(metrics, spared = false) {
    const face = `${faceOf(metrics.font)}${spared ? "+1" : ""}\u241f`;
    const width = words => remembered(face + words, () => {
        const wide = metrics.advanceWidth(words);
        return spared ? Math.ceil(wide) + 1 : wide;
    });
    width.kept = (key, fitting) => remembered(`${face}\u241e${key}`, fitting);
    return width;
}
