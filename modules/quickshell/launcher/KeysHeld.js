.pragma library

// What the open launcher makes of its keys moving (Launcher.qml's
// keysMoved), called as its card's activation or niri's focused window
// changes, in either order. A click or a touch on a window takes the keys
// from an open launcher by niri's on-demand rule, and one left open that
// hears nothing reads as broken (2026-11-09), so once it has held them, a
// window taking them closes it. A notice held over it takes them with no
// window focused, and it stays open beneath.
//
// It holds them only once niri says no window has them as well as the
// card being active. A closing's letting go reaches niri only with the
// card's next commit, which can be the next opening's: until then the card
// still reads active from before, and niri's focus on the window the
// closing gave the keys to comes late. Held on that word alone, the
// opening then closed itself within 10 ms of the card's late deactivation
// (vm-desktop-looks, 3 of 6 runs, 2026-11-11); Joel meets it closing and
// opening it again quickly.
//
// `seen` is { active, focused, lettingGo }: whether the card is active,
// the window niri says is focused (null for none, as while a layer has the
// keys), and whether the launcher let them go on purpose. It answers
// "held" where the launcher has the keys, "taken" where a window took
// keys it held, which closes it, and "" where nothing follows.
function moved(held, seen) {
    if (seen.active) {
        return seen.focused === null ? "held" : "";
    }
    if (seen.lettingGo) {
        return "";
    }
    return held && seen.focused !== null ? "taken" : "";
}
