// Drives the launcher's windows (modules/quickshell/launcher/Windows.qml)
// and Capture's (Capture.qml) through Quickshell itself, with niri, pi's
// sessions, the recorder, the shortcuts and what was taken stood in. It
// lists the window Joel's in as This Window, then every window the one
// used last first; a window on another workspace says which; and when the
// focus moves, only the rows of the windows whose focus changed are made
// again, the rest kept as they were, while a window renamed is made again
// with its new name. Capture lists every window but the one Joel's in, and
// keeps a window's row the same way. It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    property string waiting: "the first listing"

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    QtObject {
        id: theLauncher

        property bool open: false
    }

    Launcher.Windows {
        id: windows

        launcher: theLauncher
        settings: ({ name: "windows" })
    }

    Launcher.Capture {
        id: capture

        launcher: theLauncher
        settings: ({ name: "capture", camera: false })
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }

    Timer {
        running: true
        interval: 0
        onTriggered: root.cases()
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function rowOf(id) {
        return windows.things.find(row => row.id === id && row.is !== "windows:this") ?? null;
    }

    function takingOf(name) {
        return capture.windows.find(row => row.name === name) ?? null;
    }

    // Niri's windows changing as its events change them (Niri.qml): the
    // focus said first, then each record changed in place, and the
    // revision moved on.
    function changed(windowsNow, focused) {
        Niri.focused = focused;
        for (const id of Object.keys(windowsNow))
            Object.assign(Niri.windows[id], windowsNow[id]);
        Niri.revision += 1;
    }

    function cases() {
        root.waiting = "the cases";
        try {
            const first = windows.things;
            expect("This Window, then the one used last first",
                names(first) === "This Window, Logbook, Draft, Notes", names(first));
            expect("one elsewhere says its workspace",
                rowOf(12).side === "tester \u00b7 workspace 2", rowOf(12).side);

            // The focus moves from the logbook to the notes, as a click or
            // a key does: niri says both windows again, and the rest as
            // they were. Every closing made each window's row again,
            // 348 on Joel's shell for 8 changes (2026-11-04).
            expect("Capture lists every window but the one Joel's in",
                names(capture.windows) === "Draft, Notes", names(capture.windows));
            const draft = rowOf(12);
            const takingDraft = takingOf("Draft");
            const counted = windows.tally.rows;
            // The view scrolls to the notes, which moves every tile on the
            // workspace but changes no column.
            changed({
                "11": { is_focused: false, layout: { pos_in_scrolling_layout: [1, 1],
                    tile_pos_in_workspace_view: [-400, 0] } },
                "13": { is_focused: true, focus_timestamp: { secs: 300, nanos: 0 },
                    layout: { pos_in_scrolling_layout: [2, 1], tile_pos_in_workspace_view: [0, 0] } }
            }, 13);
            const moved = windows.things;
            expect("the one used last first still", names(moved) === "This Window, Notes, Logbook, Draft",
                names(moved));
            expect("a window whose focus didn't change kept", rowOf(12) === draft, names(moved));
            expect("only This Window and the two whose focus changed made again",
                windows.tally.rows - counted === 3, `${windows.tally.rows - counted} rows`);
            expect("the window Joel's in now is This Window",
                windows.things[0].side === "Notes", windows.things[0].side);
            expect("Capture lists the one Joel left",
                names(capture.windows) === "Logbook, Draft", names(capture.windows));
            expect("Capture keeps a window's row whose title didn't change",
                takingOf("Draft") === takingDraft, names(capture.windows));

            changed({ "12": { title: "Letters" } }, 13);
            expect("a window renamed made again", rowOf(12).name === "Letters", rowOf(12).name);
            expect("Capture's row of it too", takingOf("Letters") !== null, names(capture.windows));
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            fail(error.message);
        }
    }
}
