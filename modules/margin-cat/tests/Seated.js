// The cat as the cases of its plays and the puppy's visits set it down, kept once so the two
// files' cats can't drift apart: the-margin-cat-sits-above-its-column copies it beside them.
.pragma library

// A cat 8 cells of 1 across, sitting at 100, 20 on a window across the whole of a screen 1,000 by
// 600 with no marks lit, thinking 200 ms, with `more` over any of that.
function sat(more) {
    return Object.assign({
        seat: { x: 100, y: 20 }, column: { left: 0, top: 30, width: 1000 }, across: 1000,
        height: 600, marks: 0, gap: 12, cell: 1, wide: 8, tall: 8, pause: 200
    }, more ?? {});
}
