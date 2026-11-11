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

// How many faces were named, each reading eleven of a font's properties,
// which tests/rows.qml holds a row given new words to naming none.
const faces = { named: 0 };

function facesNamed() {
    return faces.named;
}

// A face as one key: everything of a font that changes how wide words
// are drawn in it.
function faceOf(font) {
    faces.named += 1;
    return JSON.stringify([font.family, font.styleName ?? "", font.pointSize ?? -1,
        font.pixelSize ?? -1, font.weight, font.italic === true, font.letterSpacing ?? 0,
        font.wordSpacing ?? 0, font.capitalization ?? 0, font.kerning ?? true,
        font.features ?? null]);
}

// A face as a Text draws it, its variable axes included, which a
// FontMetrics' measure leaves out, and how the Text reads its words.
function drawnFace(font, format) {
    return `${faceOf(font)}${JSON.stringify(font.variableAxes ?? null)}${format}\u241d`;
}

// How wide a Text drew `words` in `face` (drawnFace), or undefined until
// one has, kept with the rest, so a head drawn again for a place shown
// before makes no Text to measure it: each kept opening's head made three
// or four, and worked out its path six times, about 3 ms of a Windows
// opening on a loaded builder (ZINAWZ, 2026-11-11).
function drawnWidth(face, words) {
    const key = face + words;
    return measures.latest[key] ?? measures.earlier[key];
}

// Keeps how wide a Text drew `words` in `face`, which a font loading late
// changes, so the newest said is kept.
function drew(face, words, wide) {
    const key = face + words;
    remembered(key, () => wide);
    measures.latest[key] = wide;
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
