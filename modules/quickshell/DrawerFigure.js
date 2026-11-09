// Where a drawer's fact draws its figure, Meter's stroke or Graph's spark,
// read apart from the drawing (Drawer.qml) so tests/query-keys.qml can
// hold it in Qt's own engine.
.pragma library

const NONE = Object.freeze({ place: "none", wide: 0 });

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
