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

        // While the launcher is shut, only when each window was used is
        // told, and each is placed by it; one not held is answered, to be
        // asked whole.
        Expect.same("when windows were used places them, answering those not held",
            HeldWindows.used(held, [{ key: "window:3", order: 500 },
                { key: "window:4", order: 400 }, { key: "window:7", order: 600 }]),
            { unknown: ["window:7"], turned: [], reworded: [] });
        Expect.same("the one used last goes first", held.order,
            ["window:3", "window:4", "window:2"]);
        Expect.same("and its window reads the time", held.windows["window:3"].focus_timestamp,
            { secs: 0, nanos: 500000000 });
        Expect.same("a whole answer saying the same after differs in none",
            HeldWindows.take(held, [record(3, 500, { title: "renamed" })], []), []);
        // The focus moving is told of the window it left as well as the
        // one it went to, so the windows held say which has it, and the
        // rows of both are answered, to be made before they're next shown.
        Expect.same("the focus moving turns the window it went to and the one it left",
            HeldWindows.used(held, [{ key: "window:3", order: 800, focused: true },
                { key: "window:2", order: 400, focused: false }]),
            { unknown: [], turned: ["window:3", "window:2"], reworded: [] });
        Expect.same("which their windows read", [held.windows["window:3"].is_focused,
            held.windows["window:2"].is_focused], [true, false]);
        Expect.same("so a whole answer after the focus moved differs in none",
            HeldWindows.take(held, [record(3, 800, { title: "renamed", focused: true }),
                record(2, 400), record(4, 400)], []), []);
        // A workspace switched to, or a window retitled, turns what its
        // row says, so those rows are answered to be made again, and the
        // opening's whole answer after finds them the same.
        Expect.same("a switched workspace and a retitle reword their windows alone",
            HeldWindows.used(held, [{ key: "window:3", order: 800, elsewhere: true },
                { key: "window:2", order: 400, title: "later", named: "later" },
                { key: "window:4", order: 900 }]),
            { unknown: [], turned: [], reworded: ["window:3", "window:2"] });
        Expect.same("what a row says is what the shut launcher hears beside the order",
            HeldWindows.worded(), ["elsewhere", "named", "title", "workspace",
                "workspace_idx", "column", "down", "alike"]);
        Expect.same("so a whole answer after the switch differs in none",
            HeldWindows.take(held, [record(3, 800, { title: "renamed", focused: true,
                elsewhere: true }), record(2, 400, { title: "later", named: "later" }),
                record(4, 900)], []), []);
        // A window moved between workspaces behind the front one stays
        // elsewhere, and one below a workspace niri removed is renumbered,
        // but its row says the number; one moved a column, or joined by
        // another of its program, says its column (9KI56O on #3530).
        Expect.same("a moved, renumbered or newly alike window rewords its row",
            HeldWindows.used(held, [{ key: "window:3", order: 800, workspace: 2,
                workspace_idx: 2 }, { key: "window:2", order: 400, workspace_idx: 3 },
                { key: "window:4", order: 900, column: 2, alike: 2 }]),
            { unknown: [], turned: [], reworded: ["window:3", "window:2", "window:4"] });

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

        // A workspace's side says each program once, counted when it has
        // more than one window there, in the words given.
        const words = { empty: "empty", counted: "{program} \u00d7{count}" };
        Expect.same("a program with three windows is said once, counted",
            HeldWindows.insideSaid([{ program: "kitty", count: 3 },
                { program: "Google Chrome", count: 1 }], words),
            "kitty \u00d73, Google Chrome");
        Expect.same("and a workspace with none says it's empty",
            HeldWindows.insideSaid([], words), "empty");

        // Applications draws a program's count and Go To again only when
        // its windows came, went or another was used last.
        const one = { id: 1, title: "a" };
        const two = { id: 2, title: "b" };
        const kitty = list => ({ "kitty.desktop": list });
        Expect.expect("the same windows move nothing",
            !HeldWindows.ownersMoved(kitty([one, two]), kitty([one, two])), "moved");
        Expect.expect("nor does a retitle alone", !HeldWindows.ownersMoved(kitty([one, two]),
            kitty([one, { id: 2, title: "renamed" }])), "moved");
        Expect.expect("a window opened moves its program",
            HeldWindows.ownersMoved(kitty([one]), kitty([one, two])), "unmoved");
        Expect.expect("so does another used last",
            HeldWindows.ownersMoved(kitty([one, two]), kitty([two, one])), "unmoved");
        Expect.expect("and a program's last window closing",
            HeldWindows.ownersMoved(kitty([one]), {}), "unmoved");
        Expect.expect("and a program's first opening",
            HeldWindows.ownersMoved({}, kitty([one])), "unmoved");
    }
}
