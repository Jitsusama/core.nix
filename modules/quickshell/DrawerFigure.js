// How a drawer's fact lays a value made of items on its lines, and where
// it draws its figure, Meter's stroke or Graph's spark, read apart from the
// drawing (Drawer.qml) so tests/query-keys.qml can hold it in Qt's own
// engine.
.pragma library

const NONE = Object.freeze({ place: "none", wide: 0 });
const DOTTED = " \u00b7 ";

// A value made of items, "40% \u00b7 resets Monday 09:00", as lines no wider
// than `wide` by `width(text)`: as many items to a line as fit, a dot
// apart, and where the run breaks between two the dot goes with the break,
// so no line ends or starts on one (VKCF3Z, 2026-11-09). An item wider than
// a line of its own stands on its own, for its words to wrap.
function runLines(words, wide, width) {
    const lines = [];
    for (const item of String(words).split(DOTTED)) {
        const last = lines.length - 1;
        if (last >= 0 && width(`${lines[last]}${DOTTED}${item}`) <= wide) {
            lines[last] = `${lines[last]}${DOTTED}${item}`;
        } else {
            lines.push(item);
        }
    }
    return lines;
}

// How wide a figure stands in `room`: a stroke, having no letters, takes
// what room there is up to its `usual` width; a spark keeps its usual
// width whatever the room.
function wideIn(figure, room) {
    return figure.shape === "stroke" ? Math.min(figure.usual, room) : figure.usual;
}

// Where a fact's figure stands, and how wide: beside its value, on the
// value's one line, while `room` is left there for it at `least`, short of
// which a share no longer reads; else on the line under the value, from the
// value's left at its usual width, no wider than the value may run
// (`under`), so one window's meter reads as its neighbour's rather than
// going (VKCF3Z, 2026-11-09). Only a figure that won't fit even there, or
// none, is "none".
function place(figure, lines, room, under) {
    if (!figure) {
        return NONE;
    }
    const beside = wideIn(figure, room);
    if (lines <= 1 && beside <= room && beside >= figure.least) {
        return { place: "beside", wide: beside };
    }
    const below = wideIn(figure, under);
    return below <= under && below >= figure.least ? { place: "under", wide: below } : NONE;
}
