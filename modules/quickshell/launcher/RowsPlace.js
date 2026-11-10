.pragma library

// Where the rows drawn stand, as each row reads it: the glyph and hue of
// the thing the last step holds, which an act of its own without them
// wears, and whether they're home's, where a row says its kind. Each row
// read it from the launcher's held, so every assignment of held told
// every row drawn: a step redrew the rows of the place left for the one
// they were about to give way to, and a refresh, which makes held again
// for the place it stays in, redrew every row for nothing, 2 to 6 ms of
// each in keeps-pace's loaded sandbox (2026-11-10). Said as plain values
// instead, a row is told only when one of them changes.

// The place the rows of `held`, the launcher's steps, stand in.
function of(held) {
    const thing = held.length > 0 ? held[held.length - 1].row : null;
    return {
        glyph: String(thing?.glyph ?? ""),
        hue: String(thing?.hue ?? ""),
        home: held.length === 0,
    };
}

// Says it on `rows`, the launcher, whose rowsGlyph, rowsHue and rowsAtHome
// the rows read; a value that hasn't changed tells no row.
function tell(rows, held) {
    const place = of(held);
    rows.rowsGlyph = place.glyph;
    rows.rowsHue = place.hue;
    rows.rowsAtHome = place.home;
}
