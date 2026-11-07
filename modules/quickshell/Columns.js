// Where a card's columns stand, measured from the edge of its content
// (Card.qml), so every head and row on every card reads the same ones and
// none places its ink by hand (VKCF3Z, 2026-11-06). Across, in order: the
// gutter, the width it's given, its digits and the tie after them, or 0 on a
// card that isn't a list, as one notice isn't (Vim's nonumber); the mark
// column, as wide as what stands in it; the text column, where every name,
// title and line of words starts; and the far end, where what a row says at
// its right ends. A card's columns differ from another's by their gutters
// and by what stands in their mark columns alone.
.pragma library

// What stands in the mark column: a mark, given the room one takes (a Name
// pitch), or a picture, given its width.
function ofMark(wide) {
    return { wide, picture: false };
}

function ofPicture(wide) {
    return { wide, picture: true };
}

// The columns of a card whose content is `width` across, given its gutter,
// what stands in its mark column, and the theme's rungs. The text column
// follows a mark by the mark rung, since a mark stands on its words, and a
// picture by apart, since a picture is a thing beside them (VKCF3Z,
// 2026-11-06).
function across(width, gutter, held, rungs) {
    const gutterWidth = Math.max(0, gutter);
    const after = held.picture ? rungs.apart : rungs.mark;
    return {
        gutter: 0,
        gutterWidth,
        mark: gutterWidth,
        markWidth: held.wide,
        text: gutterWidth + held.wide + after,
        end: width,
    };
}
