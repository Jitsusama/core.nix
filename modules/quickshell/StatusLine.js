// What a card's status line works out, the launcher's and held notices'
// alike (StatusLine.qml), here so tests/status-line.qml can hold it in Qt's
// own engine: which hints keep their place as the room runs short.
.pragma library

// The hints that fit the room, each as wide as `wide` measures it and the
// gap between two as wide as `join`: the middle ones give way first, then
// the first, so the last two, the keys list's and Return's at the right end,
// always stay.
function fitted(hints, wide, join, room) {
    const width = some => some.reduce((sum, hint) => sum + wide(hint), 0)
        + Math.max(0, some.length - 1) * join;
    let shown = hints.slice();
    while (shown.length > 2 && width(shown) > room) {
        const going = shown.length > 3 ? 1 : 0;
        shown = shown.slice(0, going).concat(shown.slice(going + 1));
    }
    return shown;
}
