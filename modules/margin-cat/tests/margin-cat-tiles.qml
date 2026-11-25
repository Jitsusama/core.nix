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
            Tiles.tile(windows, workspaces, "eDP-1", 1),
            { left: 420, top: 12, width: 380, floats: false });
        expect("a floating window's tile says it floats",
            Tiles.tile(windows, workspaces, "eDP-1", 3)?.floats, true);
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
        // niri says the window it sat on closed before it lays the column
        // out again, and sitting, the shell hears no other tile: the walk
        // off rides from the tiles that still held its seat.
        const sat = Tiles.of(windows, 7);
        const shut = sat.filter(one => one.id !== 2);
        expect("its seat closed, the walk off rides from the tiles that held it",
            Tiles.before(shut, sat, 2), sat);
        expect("its seat still there, from the tiles last seen",
            Tiles.before(shut, sat, 1), shut);
        expect("sitting nowhere, from the tiles last seen", Tiles.before(shut, sat, null), shut);
        // niri's word of when each tile's resize began, as its patch says it.
        const moving = (one, animation, began) => Object.assign({}, one, { layout:
            Object.assign({}, one.layout, { tile_animation: animation,
                tile_animation_began_ms_ago: began }) });
        const growing = Object.assign({}, windows, { 1: moving(windows[1], "Resize", 4.5),
            2: moving(windows[2], "Resize", 0.1), 3: moving(windows[3], "TileMove", 90),
            4: moving(windows[4], "Resize", 200) });
        expect("a resize under way says when it began, the longest ago on its workspace",
            [Tiles.animating(growing, 7, "Resize"), Tiles.animating(windows, 7, "Resize"),
                Tiles.animating(growing, 9, "Resize")], [4.5, null, null]);
        const scrolling = Object.assign({}, windows, { 3: moving(windows[3], "ViewMove", 30) });
        expect("a scroll under way says when it began, as a resize does, and a tile's move its own",
            [Tiles.animating(scrolling, 7, "ViewMove"), Tiles.animating(growing, 7, "TileMove"),
                Tiles.animating(growing, 7, "ViewMove")], [30, 90, null]);
        // niri says a scroll as it begins and never as it ends, so its word
        // stays on a window as another's layout comes, a float dragged.
        const word = Tiles.told(scrolling, 7, "ViewMove");
        const dragged = Object.assign({}, scrolling, { 1: moving(windows[1], null, null) });
        const again = Object.assign({}, scrolling, { 3: moving(windows[3], "ViewMove", 0) });
        expect("a scroll's word stays as another window moves, and changes as niri tells again",
            [word === "", Tiles.told(dragged, 7, "ViewMove") === word,
                Tiles.told(again, 7, "ViewMove") === word, Tiles.told(windows, 7, "ViewMove")],
            [false, true, false, ""]);
        const first = Tiles.hears(null, word, 1000);
        expect("a word is heard when first told, and kept as it's told again",
            [first, Tiles.hears(first, word, 1400), Tiles.hears(first, "other", 1400)],
            [{ word, at: 1000 }, { word, at: 1000 }, { word: "other", at: 1400 }]);
        expect("a scroll runs for its spring's length after it's heard, and none runs untold",
            [Tiles.runs(first, 1229, 230), Tiles.runs(first, 1230, 230),
                Tiles.runs({ word: "", at: 1000 }, 1000, 230), Tiles.runs(null, 1000, 230)],
            [true, false, false, false]);
    }
}
