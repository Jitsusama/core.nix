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
        const scrolling = Object.assign({}, windows, { 1: moving(windows[1], "ViewMove", 30) });
        expect("a scroll under way says when it began, as a resize does, and a tile's move its own",
            [Tiles.animating(scrolling, 7, "ViewMove"), Tiles.animating(growing, 7, "TileMove"),
                Tiles.animating(growing, 7, "ViewMove")], [30, 90, null]);
        // niri puts a scroll's word on every tiled window of its workspace,
        // never a floating one, as it begins and never as it ends, so it
        // stays on a window until that window's next layout.
        const viewing = Object.assign({}, windows, { 1: moving(windows[1], "ViewMove", 30),
            2: moving(windows[2], "ViewMove", 30) });
        expect("a scroll is told on each window that says it, by how long before it began",
            [Tiles.told(viewing, 7, "ViewMove"), Tiles.told(viewing, 8, "ViewMove"),
                Tiles.told(growing, 7, "ViewMove")]
                .map(each => each.map(one => ({ id: one.id, ago: one.ago }))),
            [[{ id: 1, ago: 30 }, { id: 2, ago: 30 }], [], []]);
        // niri puts a column's move on each tile of the columns it moves,
        // so the cat reads only the word of the window it's carried by, and
        // every window's word is heard as its layout comes, so one asked of
        // long after keeps its own begin and reads as ended (#7194's round,
        // F4).
        const besides = Object.assign({}, windows, { 1: moving(windows[1], "ColumnMove", 20),
            2: moving(windows[2], "ColumnMove", 40) });
        const movedHeard = Tiles.hears(null, Tiles.told(besides, 7, "ColumnMove"), 1000);
        const movedLater = Tiles.hears(movedHeard, Tiles.told(besides, 7, "ColumnMove"), 9000);
        expect("a column's move is dated for each window it moved, and for none unmoved",
            [Tiles.heardOf(movedHeard, 2).at, Tiles.heardOf(movedHeard, 1).at,
                Tiles.heardOf(movedHeard, 3).at, Tiles.heardOf(null, 2).at],
            [960, 980, null, null]);
        expect("a column's move asked of long after keeps its begin and reads as ended",
            [Tiles.heardOf(movedLater, 2).at,
                Tiles.runs(Tiles.heardOf(movedLater, 2), 9000, 500),
                Tiles.runs(Tiles.heardOf(movedHeard, 2), 1000, 500)],
            [960, false, true]);
        const first = Tiles.hears(null, Tiles.told(viewing, 7, "ViewMove"), 1000);
        expect("a scroll first heard is dated by niri's begin, not when it was heard",
            first.at, 970);
        // Its scroll long done, one window's layout comes again with no
        // word, a retitle or a column resized: the word left on the other
        // keeps its old begin, so no scroll runs.
        const resized = Object.assign({}, viewing, { 2: moving(windows[2], "Resize", 0) });
        const later = Tiles.hears(first, Tiles.told(resized, 7, "ViewMove"), 2000);
        expect("a scroll's word left on a window as another's layout comes keeps its begin",
            [later.at, Tiles.runs(later, 2000, 230)], [970, false]);
        // Sent again mid-scroll, a window's layout says the same begin
        // later on, which keeps the scroll's own.
        const resent = Object.assign({}, viewing, { 1: moving(windows[1], "ViewMove", 130) });
        expect("a layout sent again mid-scroll keeps the scroll's begin",
            Tiles.hears(first, Tiles.told(resent, 7, "ViewMove"), 1100).at, 970);
        const again = Object.assign({}, resized, { 1: moving(windows[1], "ViewMove", 5) });
        expect("a new scroll on any window is dated by its own begin, the latest",
            Tiles.hears(later, Tiles.told(again, 7, "ViewMove"), 3000).at, 2995);
        // The old scroll's word still left on one window, the new one's on
        // the other: the new begin is the scroll's, never the earliest.
        const mixed = Object.assign({}, viewing, { 2: moving(windows[2], "ViewMove", 5) });
        expect("a new scroll beside an old one's word left on another window is dated by the new",
            Tiles.hears(first, Tiles.told(mixed, 7, "ViewMove"), 3000).at, 2995);
        // A new scroll said 30 ms after it began, as the last was: niri's
        // new layout says it, so it's dated anew, never taken for the old.
        const twice = Object.assign({}, viewing, { 1: moving(windows[1], "ViewMove", 30) });
        expect("a new scroll said as long after its begin as the last is dated by its own",
            Tiles.hears(first, Tiles.told(twice, 7, "ViewMove"), 3000).at, 2970);
        expect("a scroll runs for its spring's length after it began, and none runs untold",
            [Tiles.runs(first, 1199, 230), Tiles.runs(first, 1200, 230),
                Tiles.runs(Tiles.hears(first, [], 1000), 1000, 230),
                Tiles.runs(null, 1000, 230)],
            [true, false, false, false]);
        expect("a clock stepped back to before a scroll began runs none",
            Tiles.runs(first, 900, 230), false);
    }
}
