// How the shell's niri (modules/quickshell/Niri.qml) holds what the
// windows daemon tells, run in Qt's own JavaScript engine, the one the
// shell runs it in. The `niri-is-held-as-the-windows-daemon-tells-it`
// check copies NiriRecords.js and tests/lib/Expect.js beside this file and
// runs `qml -platform offscreen niri-records.qml` over records made up
// here, as modules/windows says them to a watch of NiriRecords'
// WINDOW_FIELDS.
import QtQuick
import "NiriRecords.js" as NiriRecords
import "Expect.js" as Expect

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`${error}`);
        }
    }

    function window(id, fields) {
        return Object.assign({
            key: `window:${id}`, id: id, order: 1000500 + id, title: `window ${id}`,
            app_id: "foot", opened: "foot", session: false, pid: 100 + id, workspace: 1,
            floating: false, urgent: false, column: id, down: 1
        }, fields ?? {});
    }

    function workspace(id, fields) {
        return Object.assign({
            key: `workspace:${id}`, id: id, idx: id, name: null, output: "eDP-1",
            active: id === 1, focused: id === 1, urgent: false, active_window: id === 1 ? 1 : null
        }, fields ?? {});
    }

    function run() {
        const held = NiriRecords.held();
        let told = NiriRecords.take(held, [
            window(1),
            window(2, { app_id: "kitty", opened: "pi-session.3", session: true }),
            window(3, { floating: true, column: null, down: null, order: 0 }),
            workspace(1), workspace(2),
            { key: "focus", window: 1 },
            { key: "desktop", overview: false, settings_failed: true },
            { key: "screenshot", path: "/tmp/old.png", taken: 4 }
        ], []);
        Expect.expect("a whole answer is a change a list shows", told.listed, "it isn't");
        Expect.expect("and makes the workspaces anew", told.spaces, "it doesn't");
        Expect.same("and says every window opened", told.opened.map(w => w.id), [1, 2, 3]);
        Expect.same("a window in niri's shape", held.windows[1], {
            id: 1, title: "window 1", app_id: "foot", opened: "foot", pid: 101,
            workspace_id: 1, is_focused: true, is_floating: false, is_urgent: false,
            focus_timestamp: { secs: 1000, nanos: 501000000 },
            layout: { pos_in_scrolling_layout: [1, 1] }
        });
        Expect.same("a session's window is kitty's, saying so",
            [held.windows[2].app_id, held.windows[2].opened, held.windows[2].session],
            ["kitty", "pi-session.3", true]);
        Expect.same("a floating window has no place in the scrolling layout, nor a use",
            [held.windows[3].layout.pos_in_scrolling_layout, held.windows[3].focus_timestamp,
                held.windows[3].is_floating], [null, null, true]);
        Expect.same("a workspace in niri's shape", held.workspaces[1], {
            id: 1, idx: 1, name: null, output: "eDP-1", is_active: true, is_focused: true,
            is_urgent: false, active_window_id: 1
        });
        Expect.same("the window niri says has the keys", held.focused, 1);
        Expect.expect("niri's settings failing is said", held.settingsFailed, "it isn't");
        Expect.expect("the screenshot taken before is said to nobody", !told.shot, "it's said");

        const windows = held.windows;
        told = NiriRecords.take(held, [window(2, { app_id: "kitty", opened: "pi-session.3",
            session: true, title: "pi: letters" })], []);
        Expect.expect("a retitle is a change a list shows", told.listed, "it isn't");
        Expect.same("and says its window changed", told.opened.map(w => w.title), ["pi: letters"]);
        Expect.expect("in place, as niri's events changed it", held.windows === windows,
            "the windows were made anew");
        Expect.expect("and not the workspaces", !told.spaces, "they were made anew");

        told = NiriRecords.take(held, [window(1, { order: 2000000 })], []);
        Expect.expect("a window used is no change a list shows", !told.listed, "it is");
        Expect.same("and is said to nothing", told.opened, []);
        Expect.same("its use kept", held.windows[1].focus_timestamp, { secs: 2000, nanos: 0 });

        told = NiriRecords.take(held, [{ key: "focus", window: null }], []);
        Expect.same("the launcher taking the keys leaves no window with them",
            [held.focused, held.windows[1].is_focused], [null, false]);
        Expect.expect("which is no change a list shows", !told.listed, "it is");
        Expect.same("and says no window changed", told.opened, []);

        told = NiriRecords.take(held, [{ key: "focus", window: 2 }], []);
        Expect.same("a window given the keys is said, as it comes forward",
            told.opened.map(w => w.id), [2]);
        Expect.same("and says it has them alone",
            [1, 2, 3].map(id => held.windows[id].is_focused), [false, true, false]);

        told = NiriRecords.take(held, [window(1, { order: 2000000, column: 4 })], []);
        Expect.expect("a window moved to another column moves the layout", told.layouts,
            "it doesn't");
        Expect.expect("and is no change a list shows", !told.listed, "it is");
        Expect.same("its place", held.windows[1].layout.pos_in_scrolling_layout, [4, 1]);

        told = NiriRecords.take(held, [{ key: "layout:1", order: 1, id: 1,
            pos_in_scrolling_layout: [4, 1], tile_size: [900, 800],
            tile_pos_in_workspace_view: [10, 0] }], []);
        Expect.expect("a tile told moves the layout", told.layouts, "it doesn't");
        Expect.same("and is held as niri lays it out", held.windows[1].layout, {
            pos_in_scrolling_layout: [4, 1], tile_size: [900, 800],
            tile_pos_in_workspace_view: [10, 0]
        });
        told = NiriRecords.take(held, [window(1, { order: 2000000, column: 4,
            title: "Letters" })], []);
        Expect.same("a window retitled keeps its tile", held.windows[1].layout.tile_size,
            [900, 800]);
        NiriRecords.take(held, [], ["layout:1"]);
        Expect.same("a tile let go leaves only its place",
            held.windows[1].layout, { pos_in_scrolling_layout: [4, 1] });

        const spaces = held.workspaces;
        told = NiriRecords.take(held, [workspace(1, { active: false, focused: false }),
            workspace(2, { active: true, focused: true })], []);
        Expect.expect("a workspace activated is a change a list shows", told.listed,
            "it isn't");
        Expect.expect("in place, as niri said it", !told.spaces && held.workspaces === spaces,
            "the workspaces were made anew");
        told = NiriRecords.take(held, [workspace(3)], []);
        Expect.expect("a workspace added makes them anew, as niri said them whole",
            told.spaces && held.workspaces !== spaces && held.workspaces[3].idx === 3,
            "they weren't");

        told = NiriRecords.take(held, [{ key: "cast:7", order: 7, stream_id: 7,
            kind: "PipeWire", is_active: true, pid: 300 }], []);
        Expect.same("a cast is held by its stream, as niri says it", held.casts,
            { 7: { stream_id: 7, kind: "PipeWire", is_active: true, pid: 300 } });
        Expect.expect("and says the casts changed", told.casts, "it doesn't");
        NiriRecords.take(held, [], ["cast:7"]);
        Expect.same("a cast stopped is let go", held.casts, {});

        told = NiriRecords.take(held, [{ key: "screenshot", path: "/tmp/new.png", taken: 5 }],
            []);
        Expect.same("a screenshot taken is said with its path", [told.shot, told.path],
            [true, "/tmp/new.png"]);
        told = NiriRecords.take(held, [{ key: "screenshot", path: null, taken: 6 }], []);
        Expect.same("one only copied is said with none", [told.shot, told.path], [true, null]);

        told = NiriRecords.take(held, [{ key: "desktop", overview: true,
            settings_failed: false }], []);
        Expect.same("the overview and the settings as the desktop says them",
            [held.overview, held.settingsFailed, told.listed], [true, false, false]);

        told = NiriRecords.take(held, [], ["window:3"]);
        Expect.same("a window closed is said", told.closed, [3]);
        Expect.expect("and is a change a list shows", told.listed, "it isn't");
        Expect.expect("and is let go", !(3 in held.windows), "it's still held");
    }
}
