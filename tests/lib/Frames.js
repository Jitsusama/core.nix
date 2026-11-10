// What a QML test's stand-in launcher keeps as the launcher's afterFrame
// does: each thing to run once the next frame is drawn, run when the test
// says a frame is (drawn). A stand-in holds `kept()` in a property and
// pushes onto its `due`; the object is written into, never reassigned, so
// keeping notifies nothing, as Provider.qml's keeper. A test imports it as
// `import "Frames.js" as Frames`, its check copying it beside the test.
.pragma library

function kept() {
    return { due: [] };
}

// A frame drawn: what waited for it runs, and what that asks to wait for
// waits for the frame after.
function drawn(frames) {
    const due = frames.due;
    frames.due = [];
    for (const then of due) {
        then();
    }
}
