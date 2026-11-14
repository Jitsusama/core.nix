// How the cat reads niri's layout (MarginCatTiles.js), in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-margin-cat-sits-above-its-column` check copies modules/margin-cat's
// scripts beside this file and runs `qml -platform offscreen
// margin-cat-tiles.qml`.
import QtQuick
import "MarginCatTiles.js" as Tiles

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const failures = [];
        const expect = (name, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${name}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };
        tiles(expect);
        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // A window as niri reports it, its tile placed across the view by its
    // id, at `place` in the scrolling layout, or floating where that's null.
    function reported(id, workspace, place) {
        return {
            id,
            workspace_id: workspace,
            layout: {
                tile_pos_in_workspace_view: [20 + 400 * id, 12],
                tile_size: [380, 740],
                pos_in_scrolling_layout: place
            }
        };
    }

    function tiles(expect) {
        const windows = {
            1: reported(1, 7, [1, 1]),
            2: reported(2, 7, [1, 2]),
            3: reported(3, 7, null),
            4: reported(4, 8, [1, 1])
        };
        const workspaces = {
            7: { output: "eDP-1", is_active: true },
            8: { output: "eDP-1", is_active: false }
        };
        expect("a window on the screen's shown workspace has its tile",
            Tiles.tile(windows, workspaces, "eDP-1", 1), { left: 420, top: 12, width: 380 });
        expect("a window on another screen has none",
            Tiles.tile(windows, workspaces, "DP-1", 1), null);
        expect("a window on a workspace not shown has none",
            Tiles.tile(windows, workspaces, "eDP-1", 4), null);
        expect("no window has none", Tiles.tile(windows, workspaces, "eDP-1", null), null);
        expect("a workspace's tiles say their column and row, a floating one's null",
            Tiles.of(windows, 7).map(one => [one.id, one.column, one.row]),
            [[1, 1, 1], [2, 1, 2], [3, null, null]]);
        expect("no workspace has no tiles", Tiles.of(windows, null), []);
        // niri says a workspace shown before its windows' tiles there, as
        // it slides to one or takes a new scale, so a window not laid out
        // yet still keeps its workspace from being empty.
        const unlaid = { 5: { id: 5, workspace_id: 9, layout: {} } };
        expect("a workspace with a window not laid out yet isn't empty",
            Tiles.empty(unlaid, 9), false);
        expect("a workspace no window is on is empty", Tiles.empty(windows, 9), true);
        expect("a workspace with windows isn't empty", Tiles.empty(windows, 8), false);
    }
}
