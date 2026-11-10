// The desktop's facts from the overview's pieces (Facts.qml), worked out
// apart so tests/facts.qml can hold it in Qt's own engine.
.pragma library

// One fact, { entry, piece }, for each overview entry, in its order, the
// piece null until it loads. A piece that stands for several things, as
// the Bluetooth devices' batteries do, gives them as `each`, a list of
// things each shaped as a piece reads, and is a fact for each, its mark
// `<mark>:<key>`. Every reader knows a fact by its mark (Arrivals'
// notice, the margin's cell, Now's row), so each thing is said, drawn and
// listed on its own, and none while there are none.
// Each fact keeps its `place` in that order, counting from 0, since the
// entries leave their own order out (Pieces.qml) and the margin places its
// marks by it (Marks.arranged); without it, two of a kind fell in
// whichever order they came true.
// Qt's JavaScript has no flatMap, so each is pushed in turn.
function facts(entries, pieces) {
    const all = [];
    entries.forEach((entry, index) => {
        const piece = pieces[index] ?? null;
        if (piece === null || !Array.isArray(piece.each)) {
            all.push({ entry, piece, place: all.length });
            return;
        }
        for (const thing of piece.each) {
            all.push({
                entry: Object.assign({}, entry, {
                    mark: entry.mark === null ? null : `${entry.mark}:${thing.key}`
                }),
                piece: thing,
                place: all.length
            });
        }
    });
    return all;
}
