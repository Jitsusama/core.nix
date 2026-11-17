// Whether what the launcher holds of Joel's own, a copy's contents, hides
// because another program may be seeing the screen (Copies.qml's hiding):
// any screencast niri streams through the portal, as a call's share is,
// starting or streaming, the shell's own recording of one window aside;
// and, until niri has said which casts there are, every time, since the
// shell can't yet tell that nothing sees the screen (ISXEG8, 2026-11-17).
.pragma library

// `heard` whether niri has said its casts, `casts` them by stream as
// Niri.qml holds them, `recording` the session of the window Recorder.qml
// records, or null.
function hidden(heard, casts, recording) {
    if (heard !== true) {
        return true;
    }
    return Object.values(casts ?? {}).some(cast => cast.kind === "PipeWire"
        && (!recording || cast.session_id !== recording));
}
