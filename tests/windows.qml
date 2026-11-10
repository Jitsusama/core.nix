// Drives the launcher's windows (modules/quickshell/launcher/Windows.qml)
// and Capture's (Capture.qml) through Quickshell itself, with what the
// windows daemon tells (modules/windows) given to each source's Daemon as
// its lines, and pi's sessions, the recorder, the shortcuts and what was
// taken stood in. It lists the window Joel's in as This Window, then every
// window the one used last first; a window on another workspace says which;
// and when the focus moves, only the rows of the windows whose focus
// changed are made again, the rest kept as they were, while a window
// renamed is made again with its new name, the same whole answer again
// makes none, and a window closed goes. Capture lists every window but the
// one Joel's in, and keeps a window's row the same way. It exits 0 once
// every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    property string waiting: "the first listing"

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

    // One whose file-preview answers slowly, so what's wanted next waits.
    Launcher.Capture {
        id: previewer

        launcher: launcher
        settings: ({ name: "previews", camera: false, preview: ["sh", "-c", "sleep 5", "preview"] })
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: Expect.fail(`never came: ${root.waiting}`)
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

    // A window's record as the daemon says it: the tester's, which no
    // desktop entry names, on the first workspace or the second.
    function window(id, title, workspace, order, focused, column) {
        return { key: `window:${id}`, order: order, id: id, app_id: "tester", opened: null,
            session: false, title: title, pid: 100 + id, entry: null, program: "tester",
            owner: null, named: title, untitled: false, alike: 1, workspace: workspace,
            workspace_idx: workspace, elsewhere: workspace !== 1, column: column, down: 1,
            floating: false, urgent: false, focused: focused };
    }

    // The desktop's record, saying the window the keys are in.
    function desktop(record) {
        return { key: "desktop", current: record.id, workspace: 1, output: "eDP-1",
            overview: false, settings_failed: false, window: { id: record.id,
                title: record.title, app_id: record.app_id, entry: null, program: "tester" } };
    }

    // What the daemon tells, said to both sources as each one's own line,
    // Capture hearing the desktop too.
    function told(said, desktopNow) {
        windows.windowsDaemon.said(JSON.stringify(said));
        const withDesktop = desktopNow === undefined ? said : Object.assign({}, said,
            said.records ? { records: said.records.concat([desktopNow]) }
                : { changed: (said.changed ?? []).concat([desktopNow]) });
        capture.windowsDaemon.said(JSON.stringify(withDesktop));
    }

    function cases() {
        root.waiting = "the cases";
        try {
            const logbook = window(11, "Logbook", 1, 200000, true, 1);
            const draft = window(12, "Draft", 2, 100000, false, 1);
            const notes = window(13, "Notes", 1, 50000, false, 2);
            Expect.expect("nothing held before the daemon's first answer",
                windows.things.length === 0 && windows.collection.empty === "\u2026",
                windows.collection.empty);
            told({ id: "all", records: [logbook, draft, notes] }, desktop(logbook));
            const first = windows.things;
            Expect.same("This Window, then the one used last first", names(first),
                "This Window, Logbook, Draft, Notes");
            Expect.same("one elsewhere says its workspace", rowOf(12).side,
                "tester \u00b7 workspace 2");
            Expect.same("Capture lists every window but the one Joel's in", names(capture.windows),
                "Draft, Notes");

            // The same whole answer again, as the launcher opens, makes none.
            const counted = windows.tally.rows;
            const kept = rowOf(12);
            told({ id: "all", records: [logbook, draft, notes] }, desktop(logbook));
            Expect.same("the same whole answer again makes no row", windows.tally.rows - counted, 0);
            Expect.expect("and keeps each row", rowOf(12) === kept, names(windows.things));

            // The focus moves from the logbook to the notes, as a click or a
            // key does: the daemon tells of both windows, and of the rest
            // nothing. Every closing made each window's row again, 348 on
            // Joel's shell for 8 changes (2026-11-04).
            const takingDraft = takingOf("Draft");
            const looked = windows.tally.looked;
            const movedTo = Object.assign({}, notes, { order: 300000, focused: true });
            told({ changed: [Object.assign({}, logbook, { focused: false }), movedTo] },
                desktop(movedTo));
            Expect.same("the one used last first still", names(windows.things),
                "This Window, Notes, Logbook, Draft");
            Expect.expect("a window whose focus didn't change kept", rowOf(12) === kept,
                names(windows.things));
            Expect.same("only This Window and the two whose focus changed made again",
                windows.tally.rows - counted, 3);
            Expect.same("only the two told of looked at", windows.tally.looked - looked, 2);
            Expect.same("the window Joel's in now is This Window", windows.things[0].side, "Notes");
            Expect.same("Capture lists the one Joel left", names(capture.windows), "Logbook, Draft");
            Expect.expect("Capture keeps a window's row whose title didn't change",
                takingOf("Draft") === takingDraft, names(capture.windows));

            told({ changed: [Object.assign({}, draft, { title: "Letters", named: "Letters" })] });
            Expect.same("a window renamed made again", rowOf(12).name, "Letters");
            Expect.expect("Capture's row of it too", takingOf("Letters") !== null,
                names(capture.windows));

            told({ gone: ["window:12"] });
            Expect.same("a window closed goes", names(windows.things),
                "This Window, Notes, Logbook");
            Expect.same("from Capture too", names(capture.windows), "Logbook");

            // Only the newest wanted waits for file-preview, so one it
            // replaces is asked again when its drawer is next shown.
            previewer.previewAsked = { "/a.png": true, "/b.png": true, "/c.png": true };
            previewer.preview("/a.png");
            previewer.preview("/b.png");
            previewer.preview("/c.png");
            const asked = Object.keys(previewer.previewAsked).join(", ");
            Expect.expect("a picture replaced while it waits is no longer asked",
                asked === "/a.png, /c.png" && previewer.previewNext === "/c.png",
                `${asked}; next ${previewer.previewNext}`);
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            Expect.fail(error.message);
        }
    }
}
