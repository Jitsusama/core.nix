// Where a line of text stands in a band, the room between two drawn edges
// (the frame and a hairline, or two hairlines), so it reads centred by eye:
// its capitals' middle and its x-height's averaged on the band's middle,
// rather than its font's ascent and descent, which carry it high (Joel,
// 2026-11-05: "the launcher find box text is not vertically aligned within
// its box, it is more heavily top aligned"). Each line centres its own
// largest level, everything on it taking that one baseline (docs/design.md:
// text in a band centres in it, and one baseline a line).
.pragma library

// How far the baseline stands below the band's middle: halfway between
// the capitals' middle and the x-height's, each half its height above it.
function lift(caps, xHeight) {
    return (caps + xHeight) / 4;
}

// Where the baseline stands in a band `tall` high, down from its top edge.
function baseline(tall, caps, xHeight) {
    return tall / 2 + lift(caps, xHeight);
}

// How tall a band is whose baseline the rows' rhythm sets `base` below its
// top, so the line reads centred in it: where a frame's bottom falls when
// a status line closes it (docs/design.md, A frame's bottom is set).
function around(base, caps, xHeight) {
    return holding(base, base, caps, xHeight);
}

// How tall a band is that holds lines from a first baseline `first` below
// its top to a last one `last`, so they read centred in it: their capitals'
// middle and their small letters' averaged on the band's, the first line's
// capitals and small letters setting both. Where no rows under it set the
// band, its first capital stands the frame's inset under its top, and this
// says where the hairline under it falls (docs/design.md: text in a band
// centres in it).
function holding(first, last, caps, xHeight) {
    return first + last - 2 * lift(caps, xHeight);
}

// How tall a band over a row is, closed by an `edge` thick hairline, whose
// line, raised `lifted` as lift() raises it, stands a `row` over the row's
// baseline, its capitals `rowCaps` tall centred in the row: the launcher's
// head (docs/design.md: a band's height comes from the rows' rhythm). The
// head's baseline is its top edge, half the band and the lift down; the
// row's is that edge, the band, the hairline and half the row and its
// capitals down; the two a row apart leave this.
function above(row, rowCaps, lifted, edge) {
    return row - rowCaps + 2 * lifted - 2 * edge;
}
