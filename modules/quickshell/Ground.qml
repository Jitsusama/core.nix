// A row's ground, its whole band, as Vim's cursorline fills its line: from
// the frame's inside edge to its other, the gutter's numbers and the far
// end's ink an inset of padding inside it, square, so a ground meets the
// rows above and below (VKCF3Z, 2026-11-06, for Joel's "rows in the
// launcher should have padding instead of margins"). It replaced a ground
// close round the ink, a margin round a highlight. Every list's rows draw
// theirs through it, the launcher's and the held notices', so none can
// draw one of its own (every-card-is-the-one-card). Whoever stands it says
// how wide and tall its band is and where it is down the list, which the
// launcher's selection stands on; it says only where it starts across
// and what it marks.
import QtQuick

Rectangle {
    // How far left of where it stands the frame's inside edge is, which it
    // reaches out to.
    property real from: 0
    // What it marks, each in its role: "selection", the cursor or what's
    // chosen; "raised", what's marked while the keys are elsewhere, as the
    // drawer's or a command line's targets; "flash", a yank, as his
    // Neovim's on_yank draws one in the accent; or "" for nothing.
    property string ink: ""

    x: -from
    color: ink === "flash" ? Theme.accent
        : ink === "selection" ? Qt.alpha(Theme.selection, Theme.floating.selection)
        : ink === "raised" ? Qt.alpha(Theme.raised, Theme.floating.selection)
        : "transparent"
}
