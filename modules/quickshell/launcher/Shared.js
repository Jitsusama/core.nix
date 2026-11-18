// Whether what the launcher holds of Joel's own, a copy's contents or what
// a secret ask asks for, hides because another program may be seeing the
// screen (Copies.qml's hiding, and secret-asks' Asks.qml's): any cast niri says, starting or streaming, but the shell's own recording
// of one window. That's a call's share through the portal, and a screen or
// region wf-recorder reads, since a recording is a share made later, its
// file going wherever it goes (5GLTPH, 2026-11-17), and a kind niri adds
// later too. A recorder copying every frame without damage, as wf-recorder
// -D does, isn't a cast to niri, so nothing here can hide from it. And,
// until niri has said which casts there are, every time, since the
// shell can't yet tell that nothing sees the screen (ISXEG8, 2026-11-17).
.pragma library

// `heard` whether niri has said its casts, `casts` them by stream as
// Niri.qml holds them, `recording` the session of the window Recorder.qml
// records, or null.
function hidden(heard, casts, recording) {
    if (heard !== true) {
        return true;
    }
    return Object.values(casts ?? {}).some(cast => cast.kind !== "PipeWire"
        || !recording || cast.session_id !== recording);
}

// A copy as the history `served` it, saying only its kind and when it was
// copied, for its row while copies hide: the history's glyph tells a link,
// an address, a path or a number apart, and its side gives a text's lines
// or a picture's format and size, each read off what was copied (#5360's
// council, 2026-11-18). Its acts stay, so Copy and Paste still work.
function shown(served) {
    return Object.assign({}, served, {
        glyph: served?.kind === "picture" ? "picture" : "clipboard",
        // Only the item saying when it came, not any timed one, so an item
        // the history adds later is hidden until it's named here.
        side: (served?.side ?? []).filter(item => item?.as === "came")
    });
}
