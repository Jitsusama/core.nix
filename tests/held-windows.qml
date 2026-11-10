// How the launcher keeps the windows the windows daemon says, run in Qt's
// own JavaScript engine, the one the shell runs it in. The
// `the-launcher-places-each-window-it-hears` check copies
// launcher/HeldWindows.js and tests/lib/Expect.js beside this file and
// runs `qml -platform offscreen held-windows.qml` over records made up
// here, as modules/windows says them.
import QtQuick
import "launcher/HeldWindows.js" as HeldWindows
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

    function record(id, order, fields) {
        return Object.assign({
            key: `window:${id}`, order: order, id: id, app_id: "foot", opened: "foot",
            session: false, title: `window ${id}`, pid: 100 + id, entry: null,
            program: "foot", owner: null, named: `window ${id}`, untitled: false, alike: 1,
            workspace: 1, workspace_idx: 1, elsewhere: false, column: 1, down: 1,
            floating: false, urgent: false, focused: false
        }, fields ?? {});
    }

    function run() {
        const held = HeldWindows.kept();
        Expect.same("a whole answer differs in every window it says",
            HeldWindows.take(held, [record(1, 300), record(2, 100), record(3, 200)], []),
            ["window:1", "window:2", "window:3"]);
        Expect.same("the windows are held the one used last first",
            held.order, ["window:1", "window:3", "window:2"]);
        Expect.same("the same whole answer again differs in none",
            HeldWindows.take(held, [record(1, 300), record(2, 100), record(3, 200)], []), []);
        Expect.same("a retitle differs in its window alone, which stays in its place",
            HeldWindows.take(held, [record(3, 200, { title: "renamed" })], []), ["window:3"]);
        Expect.same("and keeps its place", held.order, ["window:1", "window:3", "window:2"]);
        HeldWindows.take(held, [record(2, 400, { focused: true })], []);
        Expect.same("a window looked at goes first, the rest kept as they were",
            held.order, ["window:2", "window:1", "window:3"]);
        HeldWindows.take(held, [record(4, 400)], []);
        Expect.same("of two looked at at once, the newer window goes first",
            held.order, ["window:4", "window:2", "window:1", "window:3"]);
        HeldWindows.take(held, [], ["window:1", "window:9"]);
        Expect.same("a window gone is let go, and one never held changes nothing",
            held.order, ["window:4", "window:2", "window:3"]);
        Expect.expect("its record goes with it", !("window:1" in held.records)
            && !("window:1" in held.windows), JSON.stringify(Object.keys(held.records)));

        const window = HeldWindows.asWindow(record(5, 1000500,
            { workspace: 2, column: 3, down: 2, floating: true }));
        Expect.same("a window reads as niri describes one", [window.workspace_id,
            window.layout.pos_in_scrolling_layout, window.is_floating, window.focus_timestamp],
            [2, [3, 2], true, { secs: 1000, nanos: 500000000 }]);
        const floating = HeldWindows.asWindow(record(6, 0, { column: null, down: null }));
        Expect.same("one never looked at, and floating, has no time and no column",
            [floating.focus_timestamp, floating.layout.pos_in_scrolling_layout], [null, null]);

        const space = (id, output) => ({ key: `workspace:${id}`, id: id, output: output });
        const said = {
            "workspace:1": space(1, "eDP-1"), "workspace:2": space(2, "eDP-1"),
            "workspace:3": space(3, "eDP-1"), "workspace:4": space(4, "HDMI-A-1"),
            desktop: { key: "desktop", spaces: { "eDP-1": [3, 1, 2], "HDMI-A-1": [4] } }
        };
        Expect.same("a window's workspaces are the others on its screen, in the desktop's order",
            HeldWindows.spacesFrom(said, 1).map(each => each.id), [3, 2]);
        Expect.same("and one alone on its screen has none to go to",
            HeldWindows.spacesFrom(said, 4), []);
        Expect.same("nor has one on a workspace the daemon hasn't said",
            HeldWindows.spacesFrom(said, 9), []);
        Expect.same("nor any, before the desktop is said",
            HeldWindows.spacesFrom({ "workspace:1": space(1, "eDP-1") }, 1), []);
    }
}
