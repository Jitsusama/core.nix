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
// one Joel's in, and keeps a window's row the same way, and asks each
// thing taken's drawer of one file-preview it keeps, the newest wanted
// next. It exits 0 once every case holds, and otherwise 1, saying which
// didn't.
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
        // Whether the rows are kept from being made, as a shut launcher's are.
        property bool hiding: false
        function wants(provider) {
            return !hiding;
        }
    }

    Launcher.Windows {
        id: windows

        launcher: theLauncher
        settings: ({ name: "windows" })
        // How often it told the launcher its rows changed.
        property int saidChanged: 0
        onChanged: saidChanged += 1
    }

    Launcher.Capture {
        id: capture

        launcher: theLauncher
        settings: ({ name: "capture", camera: false })
    }

    // One with a file-preview stood in, which answers each picture with its
    // own sides and the process it was asked of.
    Launcher.Capture {
        id: previewer

        launcher: launcher
        settings: ({ name: "previews", camera: false, preview: [Quickshell.env("PREVIEWER")] })
    }

    // One whose file-preview stops before it answers anything.
    Launcher.Previewer {
        id: mute

        command: [Quickshell.env("PREVIEWER"), "silent"]
        onSilentChanged: {
            Expect.expect("one that stops unanswered is said silent", silent,
                `waiting for ${root.waiting}`);
            Expect.same("and what it was asked has nothing said", mute.shown("/g.png"),
                undefined);
            Qt.callLater(() => Qt.exit(0));
        }
    }

    // Which of file-preview's answers the case waits for, and the one
    // file-preview it keeps (Previewer.qml).
    property int previewing: 0
    readonly property var kept: previewer.previews

    Connections {
        target: root.kept

        function onPreviewsChanged() {
            root.previewed();
        }
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

    function fixedNamed(key) {
        return (capture.fixed.find(row => row.key === key) ?? { name: "none" }).name;
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

    // A workspace's record as the daemon says it, on the one screen.
    function space(id, fields) {
        return Object.assign({ key: `workspace:${id}`, id: id, idx: id, name: null,
            output: "eDP-1", active: id === 1, focused: id === 1, urgent: false,
            active_window: null, new: false, inside: [] }, fields ?? {});
    }

    // What the daemon tells, said to both sources as each one's own line,
    // with the desktop when it's given.
    function told(said, desktopNow) {
        const withDesktop = desktopNow === undefined ? said : Object.assign({}, said,
            said.records ? { records: said.records.concat([desktopNow]) }
                : { changed: (said.changed ?? []).concat([desktopNow]) });
        windows.windowsDaemon.said(JSON.stringify(withDesktop));
        capture.windowsDaemon.said(JSON.stringify(withDesktop));
    }

    function cases() {
        root.waiting = "the cases";
        try {
            const logbook = window(11, "Logbook", 1, 200000, true, 1);
            const draft = window(12, "Draft", 2, 100000, false, 1);
            const notes = window(13, "Notes", 1, 50000, false, 2);
            // Nothing listens at the socket here, so the daemon is first
            // greeted as if one did.
            windows.windowsDaemon.said(JSON.stringify({ id: "hello" }));
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

            // Move to Workspace offers the others on the window's screen as
            // the daemon says them, niri's empty last one as new, and
            // heard of them, no window's row is made again. Capture
            // doesn't watch workspaces, so only Windows is told.
            const madeBefore = windows.tally.rows;
            windows.windowsDaemon.said(JSON.stringify({ changed: [
                space(1, { inside: [{ program: "tester", count: 2 }] }),
                space(2, { inside: [{ program: "tester", count: 2 },
                    { program: "foot", count: 1 }] }),
                space(3, { new: true }), space(4, { output: "HDMI-A-1" }),
                Object.assign(desktop(logbook),
                    { spaces: { "eDP-1": [1, 2, 3], "HDMI-A-1": [4] } })] }));
            const madeHearing = windows.tally.rows - madeBefore;
            const offered = windows.workspacesFor(windows.held.windows["window:11"]);
            Expect.same("Move to Workspace offers the others on its screen, saying what's on each",
                offered.map(row => `${row.name}: ${row.side}`).join("; "),
                "Workspace 2: tester \u00d72, foot; A new workspace: ");
            Expect.same("and hearing the workspaces makes no window's row", madeHearing, 0);

            // Super+Tab, Return on Draft, then the notes used while the
            // launcher is shut, all unheard, so the next opening's rows
            // are in the order of the last. None is offered until the
            // watch's answer: drawn, the launcher kept them in that order
            // once it came, and Return went to the logbook.
            windows.windowsDaemon.answeredThisWatch = false;
            Niri.focused = 12;
            Niri.revision += 1;
            // A terminal's walk of the shut launcher ends before any answer
            // can come, so it's offered the rows held, as it was before.
            Expect.same("a walk of the shut launcher is offered the rows held",
                names(windows.everything()), "Logbook, Notes");
            theLauncher.open = true;
            Expect.same("before the answer, no row of the last opening is offered",
                names(windows.everything()), "");
            Expect.same("and it says it's still reading, never that there are none",
                windows.collection.empty, "\u2026");
            told({ id: "all", records: [Object.assign({}, logbook, { focused: false }),
                Object.assign({}, draft, { order: 250000, focused: true }),
                Object.assign({}, notes, { order: 220000 })] }, desktop(draft));
            Expect.same("once answered, the rows in the order used now",
                names(windows.everything()), "Notes, Logbook");
            Expect.same("and its empty line is its own again", windows.collection.empty,
                "No other windows.");
            theLauncher.open = false;
            told({ id: "all", records: [Object.assign({}, logbook, { focused: false }),
                Object.assign({}, draft, { order: 250000, focused: true }), notes] },
                desktop(draft));
            // An answer that changes no row offers the rows held all the
            // same, which the launcher hears of.
            windows.windowsDaemon.answeredThisWatch = false;
            const saidBefore = windows.saidChanged;
            told({ id: "all", records: [Object.assign({}, logbook, { focused: false }),
                Object.assign({}, draft, { order: 250000, focused: true }), notes] },
                desktop(draft));
            Expect.expect("an answer changing nothing still offers the rows",
                names(windows.everything()) === "Logbook, Notes"
                    && windows.saidChanged > saidBefore,
                `${names(windows.everything())}; told ${windows.saidChanged - saidBefore}`);
            Niri.focused = 11;
            Niri.revision += 1;
            told({ id: "all", records: [logbook, draft, notes] }, desktop(logbook));

            // The same whole answer again, as the launcher opens, makes none.
            const counted = windows.tally.rows;
            const draftRow = rowOf(12);
            told({ id: "all", records: [logbook, draft, notes] }, desktop(logbook));
            Expect.same("the same whole answer again makes no row", windows.tally.rows - counted, 0);
            Expect.expect("and keeps each row", rowOf(12) === draftRow, names(windows.things));

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
            Expect.expect("a window whose focus didn't change kept", rowOf(12) === draftRow,
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

            // Print, then Return on This Window, as the launcher opens
            // after a pick moved the keys unheard: until the watch's
            // answer, niri's word on where the keys are is the window
            // taken and recorded, both named by its title.
            capture.windowsDaemon.answeredThisWatch = false;
            Niri.focused = 11;
            Niri.revision += 1;
            Expect.same("before the answer, This Window is where niri says the keys are",
                fixedNamed("capture:window"), "This Window \u00b7 Logbook");
            Expect.same("and Record This Window the same one",
                fixedNamed("capture:record-window"), "Record This Window \u00b7 Logbook");
            told({ id: "all", records: [Object.assign({}, logbook, { focused: false }),
                movedTo] }, desktop(movedTo));
            Expect.same("once answered, the desktop record says it",
                fixedNamed("capture:window"), "This Window \u00b7 Notes");

            // The daemon gone a second: its rows say so, rather than
            // reading on or saying there are none, but not before, since a
            // restart answers within it. The rows stay as last listed.
            windows.windowsDaemon.said(JSON.stringify({ id: "hello" }));
            Expect.same("answering, Windows' row notes nothing", windows.noted(), "");
            const listed = names(windows.things);
            windows.windowsDaemon.gone();
            Expect.same("a daemon just gone isn't said yet", windows.noted(), "");
            windows.quietFor();
            Expect.same("gone a second, it's said on Windows' row", windows.noted(),
                "not answering");
            Expect.same("and in the head while inside", windows.pressing, "not answering");
            Expect.same("and with none ever listed, the empty line", windows.collection.empty,
                "Not answering.");
            Expect.same("while the rows stay as last listed", names(windows.things), listed);
            Expect.same("and are offered, but the one niri says the keys are in",
                names(windows.everything()), "Notes");
            windows.windowsDaemon.said(JSON.stringify({ id: "hello" }));
            Expect.same("and no more once it answers", windows.noted(), "");
            windows.windowsDaemon.gone();
            windows.windowsDaemon.said(JSON.stringify({ id: "hello" }));
            windows.quietFor();
            Expect.same("nor for a restart answered within the second", windows.noted(), "");

            // While the launcher is shut, only when each window was used is
            // told, on a watch of its own, so the opening's first frame
            // offers the rows in the order used now, before the windows'
            // own watch answers; a window opened meanwhile is asked whole.
            const used = windows.usedDaemon;
            const asked = [];
            used.wire = { write: line => asked.push(JSON.parse(line)), flush: () => {} };
            used.said(JSON.stringify({ id: "hello" }));
            Niri.focused = 13;
            Niri.revision += 1;
            theLauncher.open = true;
            Expect.same("before when windows were used is told, an opening offers none",
                names(windows.everything()), "");
            used.said(JSON.stringify({ id: "all", records: [
                { key: "window:11", order: 200000 }, { key: "window:13", order: 300000 }] }));
            Expect.same("told it, an opening offers the rows held before any answer",
                names(windows.everything()), "Logbook");
            const madeShut = windows.tally.rows;
            used.said(JSON.stringify({ changed: [{ key: "window:14", order: 500000 }] }));
            Expect.same("a window opened unheard is asked whole",
                asked.filter(one => one.ask === "get").map(one => one.keys.join()).join(),
                "window:14");
            Expect.same("and until it comes, an opening offers none",
                names(windows.everything()), "");
            theLauncher.hiding = true;
            used.said(JSON.stringify({ id: `find-${used.asked}`,
                records: [window(14, "Ledger", 1, 500000, false, 3)] }));
            Expect.same("the window asked whole while the rows aren't wanted makes no row",
                windows.tally.rows - madeShut, 0);
            theLauncher.hiding = false;
            used.said(JSON.stringify({ changed: [{ key: "window:11", order: 600000 }] }));
            Niri.focused = 11;
            Niri.revision += 1;
            Expect.same("the focus moved unheard, the rows are in the order used now",
                names(windows.everything()), "Ledger, Notes");
            Expect.same("and only the window opened had its row made",
                windows.tally.rows - madeShut, 1);
            theLauncher.open = false;
            used.wire = null;

            // Only the newest wanted waits while file-preview starts, so one
            // it replaces is asked again when its drawer is next shown.
            kept.asked = { "/a.png": true, "/b.png": true, "/c.png": true };
            kept.preview("/a.png");
            kept.preview("/b.png");
            kept.preview("/c.png");
            Expect.same("a picture replaced while file-preview starts is no longer asked",
                Object.keys(kept.asked).join(", "), "/c.png");
            root.previewing = 1;
            root.waiting = "file-preview's first answer";
        } catch (error) {
            Expect.fail(error.message);
        }
    }

    // Then, while it answers one, only the newest wanted waits too, and
    // each is asked of the one file-preview, kept, rather than one started
    // for each.
    function previewed() {
        try {
            const said = kept.previews;
            if (root.previewing === 1 && "/c.png" in said) {
                Expect.same("its answer is the picture's facts", [said["/c.png"].picture,
                    said["/c.png"].across, said["/c.png"].down].join(" "), "/c.png 4 3");
                root.previewing = 2;
                root.waiting = "file-preview's next answers";
                kept.asked = Object.assign({}, kept.asked,
                    { "/d.png": true, "/e.png": true, "/f.png": true });
                kept.preview("/d.png");
                kept.preview("/e.png");
                kept.preview("/f.png");
                Expect.expect("a picture replaced while another is asked is no longer asked",
                    !("/e.png" in kept.asked) && kept.asking === "/d.png" && kept.next === "/f.png",
                    `asking ${kept.asking}, next ${kept.next}`);
            } else if (root.previewing === 2 && "/d.png" in said && "/f.png" in said) {
                Expect.same("each wanted is asked in turn", Object.keys(said).join(", "),
                    "/c.png, /d.png, /f.png");
                const by = Object.values(said).map(shown => shown.by);
                Expect.expect("every picture is asked of one file-preview",
                    by.every(one => one === by[0]), by.join(", "));
                // A picture rewritten in place keeps its path, so it's known
                // changed by its change time, and asked again; and only the
                // newest `most` are kept.
                Expect.same("a picture first drawn with its change time is asked again",
                    kept.shown("/d.png", 1)?.picture, undefined);
                kept.most = 2;
                root.previewing = 3;
                root.waiting = "file-preview's answer for a picture changed";
            } else if (root.previewing === 3 && "/d.png" in said) {
                Expect.same("one changed is asked again", kept.shown("/d.png", 1)?.picture,
                    "/d.png");
                Expect.same("and while it's the same, it's said with nothing asked",
                    [kept.asking, kept.next].join(""), "");
                Expect.same("only the newest kept", Object.keys(said).join(", "),
                    "/f.png, /d.png");
                Expect.expect("and the rest forgotten, asked again when next drawn",
                    !("/c.png" in kept.asked), Object.keys(kept.asked).join(", "));
                Expect.same("and changed again, it's unsaid until asked again",
                    kept.shown("/d.png", 2), undefined);
                // One that can't start says so once, and its drawers say so.
                root.previewing = 4;
                root.waiting = "a file-preview that stops unanswered is said";
                mute.shown("/g.png");
            }
        } catch (error) {
            Expect.fail(error.message);
        }
    }
}
