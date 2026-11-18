// y's flash, timed from the first frame that draws it rather than from its
// key, for the launcher's rows and its drawer (Launcher.qml). On a loaded
// builder niri's frames came a median 604 ms apart (2026-11-18), so a
// 200 ms flash timed from its key began and ended between two frames and
// was never seen.
.pragma library

// Starts a flash's time once a frame has drawn it, if no yank has come
// since: `later` runs a function as the turn ends (Qt.callLater),
// `afterFrame` once the frame begun after that is drawn, `newest` says
// whether this yank is still the last, and `start` starts the time. It's
// asked for the way the showing report is (Launcher.qml's showingTurnDue),
// a turn later on each side of the frame, so the time never starts before
// the report that says it flashed is made.
function fromItsFrame(later, afterFrame, newest, start) {
    later(() => afterFrame(() => later(() => {
        if (newest()) {
            start();
        }
    })));
}
