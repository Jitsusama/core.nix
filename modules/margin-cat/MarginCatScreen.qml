// One screen's margin cat (MarginCat.qml makes one for each): where on this
// screen it sits, over the column Joel's keys are in, how it walks, hops
// and wanders there and gets about the windows between workspaces, by the
// ways MarginCatPaths.js finds, and how it's drawn, a frame each time the
// screen draws while it moves and none at rest. What it plays, and when, is the
// scene's (MarginCat.qml), which every screen's cat shares, so a stroll is
// taken by one screen alone.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Wayland
import QtQuick
import "MarginCat.js" as MarginCat
import "MarginCatPaths.js" as Paths
import "MarginCatPlays.js" as Plays
import "../Pixels.js" as Pixels

PanelWindow {
    id: window

    required property ShellScreen modelData
    // The scene every screen's cat shares: how lively it is, the pose it
    // holds, and the stroll or hop under way (MarginCat.qml).
    required property var scene

    // The window the keys are in, where it's on this screen's shown
    // workspace, and the cat's to sit over.
    readonly property var wanted: {
        void window.scene.settled;
        void Niri.revision;
        if (window.scene.shared || Niri.overview || Niri.current === null)
            return null;
        return tile(Niri.current) === null ? null : Niri.current;
    }
    // The window whose column the cat sits over, or last sat over
    // while it walks or waits to: the one the keys rested in for two
    // seconds, so tapping h and l along the columns never sets it
    // walking.
    property var sat: null
    // The column of the window the keys are in, as niri says its tile
    // will come to rest, so it moves with the column as niri lays
    // them out again. The shell hears that tile alone, so where the
    // cat walks from is where it's drawn, never the column it left.
    readonly property var column: {
        void window.scene.settled;
        void Niri.revision;
        return window.scene.shared || Niri.overview ? null : tile(wanted);
    }
    // Each cell in whole device pixels, and the cat's size from them.
    // The window's own ratio, which a fractional scale sets, rather
    // than the screen's, as Hairline.qml reads it. Taken a moment
    // late: showing or hiding the window sets its ratio, so read at
    // once it would change the place that showed it, a loop Joel's
    // shell said 327 times at 1.5 on 2026-11-12 from 15:12 to 15:57,
    // three around each launcher opening.
    property real ratio: cat.Screen.devicePixelRatio > 0
        ? cat.Screen.devicePixelRatio : 1
    Binding on ratio {
        value: cat.Window.window?.devicePixelRatio
            ?? (cat.Screen.devicePixelRatio > 0 ? cat.Screen.devicePixelRatio : 1)
        delayed: true
    }
    readonly property real cell: MarginCat.cell(Theme.gap, ratio) / ratio
    readonly property real size: Math.max(1, window.scene.drawing.length) * cell
    // Where it'd sit over the keys' column, or along the margin from
    // it while a wander has it there, and where it's drawn: there, a
    // step of its walk, or still where it sat while it waits for the
    // keys to rest.
    // It reads nothing follow() writes, since a write there would
    // work it out again inside its own change, a binding loop.
    readonly property var there: column === null ? null
        : strolled ?? seat(column)
    // A stroll's: the seat it wanders to or sits at, or null for
    // its column; whether the walk under way is a scene's, which
    // hands the scene its next beat as it sits; and whether the next
    // move of its seat walks rather than carries it there, hurried
    // (true, a key bringing it back) or not (false), or null for a
    // column moving, which carries it.
    property var strolled: null
    property bool handsBack: false
    property var roams: null
    property var place: null
    // The column it last sat over or set off for, so its window grown,
    // shrunk or moved up or down under it walks it rather than carries
    // it (follow), and the windows as follow() last saw them.
    property var under: null
    property var seen: []
    // The walk under way, the window it walks to, and the step it's
    // on: its pose, by the theme's name, and the way it faces.
    property var going: null
    property var goingTo: null
    // The window it waits on the keys to rest in.
    property var waitingOn: null
    property var stepped: null
    // Its pose and the way it faces, set only as they change, so
    // its cells are laid again a few times a walk rather than on
    // every frame, which dropped five of a walk's frames on a
    // builder.
    property string pose: ""
    property int facing: 0
    // When niri showed each frame of the walk under way, how long the
    // shell took to draw each, and how many niri never showed, said
    // once it sits (MarginCat.walked). Kept as plain arrays, pushed
    // to, so a frame's answer notifies nothing.
    property var shown: []
    property var costs: []
    property int unseen: 0
    // How many frames had begun as it set off, so it can say how
    // many it drew, and how many steps it has taken since, so it can
    // say whether it moved on each.
    property int begunAt: 0
    property int steps: 0
    // How often this screen draws, in ms, as its last walk showed.
    property real frameMs: 1000 / 60
    // The workspace this screen shows, its id and index, so a move
    // between two has it ride its column off with the world and climb
    // to its new seat (VKCF3Z, 074321bd), and the one it showed.
    readonly property var space: {
        void Niri.revision;
        const name = modelData?.name ?? "";
        const shows = Object.values(Niri.workspaces)
            .find(workspace => workspace.output === name && workspace.is_active);
        return shows ? { id: shows.id, idx: shows.idx } : null;
    }
    property var spaceWas: null
    // The move between workspaces under way (Paths.moves), the
    // workspace it left and the window it goes to once niri says it,
    // and its quarter turn as it climbs.
    property var moving: null
    property var movingFrom: null
    property var movingTo: null
    property int turn: 0
    // The margin's marks, where they're drawn (Paths.marks), which it
    // hops across one at a time, perched on each.
    readonly property var marksLit: modelData === null ? null
        : Paths.marks(MarginMarks.drawn, modelData.width, Theme.gap)
    // What it's drawn as: a pose of its walk, turned the way it
    // goes and the way it climbs, or the scope's, sitting, asleep or
    // stretching.
    readonly property var drawn: pose
        ? MarginCat.quartered(
            MarginCat.turned(Theme.pixels[pose] ?? window.scene.drawing, facing), turn)
        : window.scene.drawing
    // Whether it's on its way to another column or workspace, waiting
    // to set off or going, while which every tile of the workspaces it
    // crosses is wanted, its ways being over them, and as which it wants
    // them; at rest, and on a scene's wander, which stays over its own
    // window, only the tile under it is, so the shut shell hears no
    // other (2026-11-12: watching every tile made it hear each of 20
    // resizes).
    readonly property bool travels: (going !== null && !handsBack) || moving !== null
        || resting.running
    property string wantsAs: ""
    // Wanted once the change is done: niri's windows notify as they're
    // wanted, and set it going or moving again inside the change, a
    // binding loop (the GPU rig, 2026-11-13).
    onTravelsChanged: Qt.callLater(wantsTiles)
    // Set as it's destroyed, so a want called later wants nothing.
    property bool gone: false
    function wantsTiles(): void {
        if (gone)
            return;
        if (travels && wantsAs === "") {
            wantsAs = `cat ${modelData?.name ?? ""}`;
            Niri.wantTiles(wantsAs, true);
        } else if (!travels && wantsAs !== "") {
            Niri.wantTiles(wantsAs, false);
            wantsAs = "";
        }
    }

    // Where the keys' column now has it: nowhere, once it's held
    // nowhere a moment (leaving); from
    // nowhere, there at once, since nothing walks in from nowhere;
    // over the same window, carried as its column scrolls, or, its
    // window grown, shrunk or moved up or down under it, on its way
    // there from where it is, as it is where its seat moved a pixel on
    // the way there; and over another, where it is or on to where its
    // walk was going, until the keys rest there two seconds, never
    // put anywhere it didn't get to (WCJQSG 2bd7c7c6). niri says the
    // layout again for any window's change, a title's too, so the same
    // seat neither sets a walk off again nor restarts its rest.
    function follow(): void {
        const saw = seen;
        seen = tilesOf(space?.id);
        if (moving !== null) {
            steer();
            return;
        }
        if (there === null) {
            if (place !== null)
                leaving.restart();
            return;
        }
        leaving.stop();
        if (place !== null && going === null && wanted === sat && roams !== null) {
            const hurried = roams;
            roams = null;
            resting.stop();
            walkFrom(hurried, saw);
        } else if (place === null || (going === null && wanted === sat)) {
            resting.stop();
            const reshaped = place !== null && wanted === sat && under !== null
                && column !== null && (Math.abs(column.width - under.width) > 0.5
                    || Math.abs(column.top - under.top) > 0.5);
            sat = wanted;
            under = column;
            if (reshaped)
                walkFrom(true, saw);
            else
                place = there;
        } else if (going !== null && wanted === goingTo) {
            if (Math.round(there.x * ratio) !== Math.round(going.to.x * ratio)
                || Math.round(there.y * ratio) !== Math.round(going.to.y * ratio))
                walkFrom(true, saw);
        } else if (going !== null || !resting.running || wanted !== waitingOn) {
            // Its wander forgotten a moment later, outside this
            // change of `there`, which forgetting it changes again.
            Qt.callLater(endStroll);
            waitingOn = wanted;
            resting.restart();
        }
    }

    // Sets off from where it's drawn to where it'd sit: `hurried`,
    // its seat moved on the way, without turning, thinking or
    // crouching again, or, setting off, after a thought of 150 to
    // 450 ms, so no two walks begin alike; riding what niri moved of
    // the windows it `saw` (MarginCatFalls.js's rides).
    function walkFrom(hurried: bool, saw: var): void {
        roams = null;
        if (place === null || there === null)
            return;
        const plan = Paths.walk({ x: place.x, y: place.y }, { x: there.x, y: there.y }, {
            cell,
            ratio,
            wide: size,
            tall: size,
            air: cell,
            across: modelData.width,
            pause: 150 + Math.random() * 300,
            slowdown: Theme.motion.slowdown,
            hurried,
            facing: stepped?.facing ?? 0,
            height: modelData.height,
            cols: Paths.columns(tilesOf(space?.id), 0),
            before: saw ? Paths.columns(saw, 0) : null,
            spring: Theme.motion.spring,
            ahead: MarginCat.behind(frameMs, "resize"),
            frame: frameMs,
            marks: marksLit
        });
        goingTo = wanted;
        under = column;
        if (plan === null) {
            halt();
            sat = wanted;
            place = there;
            sits();
            return;
        }
        if (going === null) {
            shown = [];
            costs = [];
            unseen = 0;
            steps = 0;
            begunAt = window.FramePresentation.framesBegun;
        }
        going = plan;
        walker.restart();
        if (window.FramePresentation.available)
            window.FramePresentation.ask();
    }

    // A move from workspace `was` to the one `way` below (1) or above (-1): it rides its column
    // the whole slide from where it sits, and its way to its new seat comes once niri says the
    // new workspace's column (steer).
    function moveFrom(way: int, was: var): void {
        halt();
        resting.stop();
        leaving.stop();
        movingTo = null;
        movingFrom = was;
        moving = plan(way, { x: place.x, y: place.y }, null);
        shown = [];
        costs = [];
        unseen = 0;
        steps = 0;
        begunAt = window.FramePresentation.framesBegun;
        mover.restart();
        if (window.FramePresentation.available)
            window.FramePresentation.ask();
    }

    // A move's plan from `from` to its seat `to`, or only its ride where
    // `to` is null, over a workspace's height and niri's gap between
    // two, a tenth of the screen's height in whole device pixels, as
    // niri's monitor.rs lays them, and over the windows of both.
    function plan(way: int, from: var, to: var): var {
        const gap = Math.max(1, Math.round(modelData.height * 0.1 * ratio)) / ratio;
        return Paths.moves(from, to, {
            way,
            step: modelData.height + gap,
            wide: size,
            tall: size,
            air: cell,
            cell,
            ratio,
            facing,
            across: modelData.width,
            height: modelData.height,
            slowdown: Theme.motion.slowdown,
            spring: Theme.motion.spring,
            ahead: MarginCat.behind(frameMs, "slide"),
            frame: frameMs,
            was: tilesOf(movingFrom),
            now: tilesOf(space?.id),
            marks: marksLit
        });
    }

    // Where a move goes as niri says the new workspace's columns: on
    // the ride, its way found again each time niri says any, since
    // none of the ride depends on them; once it's on its way, a seat
    // moved or the keys gone to another window cut it short, and it
    // walks on from where it's got to (cut()).
    function steer(): void {
        if (there === null)
            return;
        const riding = mover.elapsedTime * 1000 + moving.ahead < moving.beats[0].ends;
        const same = Math.round(there.x * ratio) === Math.round(moving.to.x * ratio)
            && Math.round(there.y * ratio) === Math.round(moving.to.y * ratio);
        if (riding) {
            movingTo = wanted;
            moving = plan(moving.way, moving.from, { x: there.x, y: there.y });
        } else if (movingTo === null || wanted !== movingTo || !same) {
            cut();
        }
    }

    // A move cut short past its ride, the keys gone to another window
    // or its seat moved: on from wherever it's got to, as a walk, so
    // it's never put anywhere it didn't get to (WCJQSG 2bd7c7c6).
    function cut(): void {
        tell("move");
        mover.stop();
        moving = null;
        movingFrom = null;
        movingTo = null;
        // No halt(): its pose and facing carry into the walk, never a
        // frame of it sitting between.
        walkFrom(true, null);
    }

    // Done moving, or cut short: it sits where the keys' column has
    // it, at once, or nowhere, as it would have with no move.
    function arrive(): void {
        tell("move");
        mover.stop();
        moving = null;
        movingFrom = null;
        movingTo = null;
        turn = 0;
        halt();
        sat = wanted;
        place = there;
    }

    onSpaceChanged: {
        const was = spaceWas;
        spaceWas = space;
        if (was === null || space === null || was.id === space.id)
            return;
        const way = Math.sign(space.idx - was.idx);
        // A move begun on a move under way rides on from where it's got
        // to, rather than being put on its seat first.
        if (place !== null && way !== 0 && !window.scene.asleep && !Niri.overview
            && !window.scene.shared)
            moveFrom(way, was.id);
        else if (moving !== null)
            arrive();
    }

    // How its walk or hop (`what`) reached the screen, in the
    // journal, so one on Joel's screen says whether it kept to its
    // rate.
    function tell(what: string): void {
        if (!window.FramePresentation.available)
            return;
        const told = MarginCat.walked(shown, costs, unseen,
            { begun: window.FramePresentation.framesBegun - begunAt, stepped: steps, what });
        console.info(told.line);
        if (told.said.frames > 10)
            frameMs = told.said.median;
        if (Options.reports)
            Report.say(`${what === "hop" ? "hopped" : "walked"} cat `
                + `${modelData?.name ?? ""} ${JSON.stringify(told.said)}`);
    }

    // A hop drawn here, its frames counted as a walk's are; and how
    // far up it has the cat, in logical pixels: its height in cells,
    // or what room the gap leaves above it, if less.
    readonly property bool hopping: window.scene.hop !== null && !!place
    readonly property real lifted: !place || window.scene.hop === null ? 0
        : window.scene.lift * Math.min(window.scene.hop.cells, Math.max(0, place.y / cell)) * cell
    onHoppingChanged: {
        if (hopping) {
            shown = [];
            costs = [];
            unseen = 0;
            steps = 0;
            begunAt = window.FramePresentation.framesBegun;
            if (window.FramePresentation.available)
                window.FramePresentation.ask();
        } else if (going === null) {
            tell("hop");
        }
    }
    // Counted among the screens that draw it, so a cat drawn on none
    // keeps still.
    readonly property bool placed: !!place
    onPlacedChanged: window.scene.placedOn += placed ? 1 : -1
    Component.onDestruction: {
        gone = true;
        if (placed)
            window.scene.placedOn -= 1;
        if (wantsAs !== "")
            Niri.wantTiles(wantsAs, false);
    }

    // Each frame of a walk or a hop, as niri showed it or didn't, asking after the next until
    // it sits. Asked only here, once niri says the last was shown, the ask lands after the next
    // frame has begun where frames come close together, so only every other frame was heard
    // of: on a GPU at 60 Hz, and on Joel's screen at 120. Each step asks too (walker, and a
    // hop's lift), before its frame is drawn. Asked only there, a GPU at 120 Hz drew every
    // other frame, so both stay.
    FramePresentation.onPresented: (frame, at) => {
        if (going === null && moving === null && !hopping)
            return;
        shown.push(at);
        const times = window.FramePresentation.timesOf(frame);
        if (times.rendered > 0 && times.synchronizing > 0)
            costs.push(times.rendered - times.synchronizing);
        window.FramePresentation.ask();
    }
    FramePresentation.onDiscarded: () => {
        if (going === null && moving === null && !hopping)
            return;
        unseen += 1;
        window.FramePresentation.ask();
    }

    function halt(): void {
        walker.stop();
        going = null;
        stepped = null;
        pose = "";
    }

    // A step of a walk or a move, `now` (Paths.frame), taken: its pose,
    // the way it faces and turns, and its place, each set only as it
    // changes.
    function took(now: var): void {
        if (now.facing !== facing)
            facing = now.facing;
        if ((now.pose ?? "") !== pose)
            pose = now.pose ?? "";
        if ((now.turn ?? 0) !== turn)
            turn = now.turn ?? 0;
        stepped = now;
        steps += 1;
        place = { x: now.x, y: now.y };
        // Asked as the step is taken, before its frame is drawn, so the
        // frame it moves is the one niri is asked about.
        if (window.FramePresentation.available)
            window.FramePresentation.ask();
    }

    // A scene's walk done, handing the scene its next beat.
    function sits(): void {
        if (!handsBack)
            return;
        handsBack = false;
        Qt.callLater(window.scene.beat);
    }

    // A stroll ended by its column going or another's taking the
    // keys: its wander is forgotten, and the scene moves on rather
    // than waiting on a walk that's gone.
    function endStroll(): void {
        strolled = null;
        roams = null;
        sits();
    }

    // A stroll's walk, where the scene says, taken only by the screen
    // whose cat sits still over the column the keys are in: a few
    // cells along the margin, then back over it, each with a thought and a
    // crouch as any walk; or, as a key ends the scene (`to` ""),
    // straight back over its column, hurried.
    function stroll(to: string): void {
        if (to === "") {
            handsBack = false;
            if (strolled !== null) {
                roams = true;
                strolled = null;
            }
            return;
        }
        if (place === null || column === null || going !== null || resting.running
            || wanted !== sat)
            return;
        if (to === "column") {
            if (strolled === null)
                return;
            window.scene.strollTaken = true;
            handsBack = true;
            roams = false;
            strolled = null;
            return;
        }
        const spot = Plays.wander({
            seat: seat(column),
            column,
            across: modelData.width,
            marks: MarginMarks.drawn,
            cell: Theme.gap,
            wide: size,
            step: cell,
            cells: window.scene.wandering?.cells ?? 0,
            way: window.scene.wandering?.way ?? 1
        });
        if (spot === null)
            return;
        window.scene.strollTaken = true;
        handsBack = true;
        roams = false;
        strolled = spot;
    }

    Connections {
        target: window.scene

        function onStrollToChanged() {
            window.stroll(window.scene.strollTo);
        }
    }

    onThereChanged: follow()

    // Nowhere, once it's held a moment: niri can say a column moving
    // to the keys' as nowhere between two of its events, which gone at
    // once had it vanish and land on the new one with no walk (a
    // builder, 2026-11-12, 2 of 4 runs), and a column truly gone, to
    // fullscreen or another workspace, takes it a moment later.
    Timer {
        id: leaving
        interval: MarginCat.LEAVES
        onTriggered: {
            if (window.there !== null)
                return;
            window.halt();
            window.endStroll();
            resting.stop();
            window.sat = window.wanted;
            window.place = null;
        }
    }

    // Until the keys rest on the column, so a tap along them never
    // sets it walking.
    Timer {
        id: resting
        interval: MarginCat.RESTS
        onTriggered: window.walkFrom(false, null)
    }

    // A step each time the screen draws, at whatever rate it draws,
    // and none once it sits.
    FrameAnimation {
        id: walker
        onTriggered: {
            const now = Paths.frame(window.going, walker.elapsedTime * 1000);
            if (now.done) {
                window.tell("walk");
                window.halt();
                window.sat = window.goingTo;
                // Where its walk took it, which is its seat unless the keys
                // went elsewhere on the way, whose seat it walks to once
                // they rest there.
                window.place = { x: now.x, y: now.y };
                window.sits();
                return;
            }
            window.took(now);
        }
    }

    // A step of a move each time the screen draws, as the walker's.
    FrameAnimation {
        id: mover
        onTriggered: {
            const now = Paths.frame(window.moving, mover.elapsedTime * 1000);
            // A ride with no column at its end leaves it nowhere, as
            // moving to an empty workspace did before it rode.
            if (now.done
                || (now.beat !== window.moving.beats[0].name && window.movingTo === null)) {
                window.arrive();
                return;
            }
            window.took(now);
        }
    }

    // A window's tile, as niri says it will come to rest, where it's on
    // this screen's shown workspace; null where it isn't, and while
    // the screen itself is null, as Variants leaves it a moment as
    // screens come and go (Joel's shell, 2026-11-12 20:11, "Cannot
    // read property 'name' of null").
    function tile(id: var): var {
        if (modelData === null)
            return null;
        const shown = id === null ? undefined : Niri.windows[id];
        const at = shown?.layout?.tile_pos_in_workspace_view;
        const workspace = shown ? Niri.workspaces[shown.workspace_id] : null;
        if (!at || workspace?.output !== modelData.name || !workspace.is_active)
            return null;
        return { left: at[0], top: at[1], width: shown.layout.tile_size[0] };
    }

    // The tiles of a workspace's windows, as Paths.columns takes them:
    // each where it's in that workspace's own view, the screen's for the
    // one shown (modules/niri/tiled-positions.patch), and its column in
    // niri's scrolling layout, none for a floating one. Only the tile
    // under the cat is held while it's at rest (travels).
    function tilesOf(workspaceId: var): var {
        if (workspaceId === null || workspaceId === undefined)
            return [];
        return Object.values(Niri.windows)
            .filter(one => one.workspace_id === workspaceId
                && one.layout?.tile_pos_in_workspace_view && one.layout?.tile_size)
            .map(one => ({
                left: one.layout.tile_pos_in_workspace_view[0],
                top: one.layout.tile_pos_in_workspace_view[1],
                width: one.layout.tile_size[0],
                height: one.layout.tile_size[1],
                column: one.layout.pos_in_scrolling_layout?.[0] ?? null,
                row: one.layout.pos_in_scrolling_layout?.[1] ?? null,
                id: one.id
            }));
    }

    // Nowhere over no column, or on a screen going away, whose width
    // is null (vm-screens-keep-the-launcher, "Cannot read property
    // 'width' of null", 4 runs of 4 on 2026-11-12).
    function seat(over: var): var {
        if (!over || modelData === null)
            return null;
        return MarginCat.sits({
            column: over,
            // The screen's, since the window's own is 0 until it's shown.
            across: modelData.width,
            marks: MarginMarks.drawn,
            cell: Theme.gap,
            wide: size,
            tall: size,
            air: cell
        });
    }

    Counted { name: `cat ${window.modelData?.name ?? ""}` }

    screen: modelData
    // A place is undefined, not null, while a binding works it out
    // again, so both are nowhere (Joel's shell, 2026-11-12: "Cannot
    // read property 'x' of undefined").
    visible: !!place && window.scene.drawing.length > 0
    anchors.top: true
    anchors.left: true
    anchors.right: true
    // The screen's height, as it gets about the windows below, even at
    // rest: grown only as a move set off, its first frames waited on
    // niri's new size, so its ride began out of sight and showed only as
    // it ended (the GPU rig, 2026-11-13: 141, 331, then 801 px down).
    // Nothing of it takes the pointer, and Qt draws it only as it moves.
    implicitHeight: modelData.height
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    // Under the margin's marks, the launcher and notices, on Overlay, so
    // a mark's ink lies over its paws as it perches there and it never
    // hides a mark or a notice (VKCF3Z, 3d799798).
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "cat"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    // Nothing of it takes the pointer, so the gap stays the windows'.
    mask: Region {}

    // Where it sits, or that it's nowhere, for a test to read.
    // It says the column it saw too, so a test that finds it nowhere
    // says why, and its cells in device pixels and its hue, so one
    // can count them on the screen. On a walk it says each frame's
    // step, its time since it set off, beat and pose, so a test can
    // see a frame dropped or a stride out of step.
    function say(): void {
        if (!Options.reports)
            return;
        const at = !place ? "nowhere" : JSON.stringify({
            x: place.x,
            y: place.y,
            size: window.size,
            air: window.cell,
            cell: Math.round(window.cell * window.ratio),
            cells: window.drawn.length,
            lit: window.drawn.join("").split("#").length - 1,
            hue: `${Theme.voice}`,
            pose: window.scene.posed,
            strolling: handsBack || strolled !== null,
            lift: Math.round(window.lifted * window.ratio),
            window: sat,
            asleep: window.scene.asleep,
            stretching: window.scene.stretching,
            walk: going === null || stepped === null ? null : {
                t: Math.round(walker.elapsedTime * 10000) / 10,
                beat: stepped.beat,
                pose: stepped.pose,
                facing: stepped.facing,
                stride: going.stride,
                to: going.to.x,
                beats: going.beats.map(beat => beat.name)
            },
            move: moving === null || stepped === null ? null : {
                t: Math.round(mover.elapsedTime * 10000) / 10,
                beat: stepped.beat,
                pose: stepped.pose,
                turn: stepped.turn,
                facing: stepped.facing,
                way: moving.way,
                beats: moving.beats.map(beat => beat.name)
            },
            // The layer's height, the screen's on a move, so it's drawn
            // wherever its way takes it.
            reach: window.implicitHeight
        });
        Report.say(`sits cat ${modelData?.name ?? ""} ${at} over ${JSON.stringify(column)}`);
    }
    onPlaceChanged: say()
    onColumnChanged: say()
    // A new scale can leave its logical place and size as they were
    // while its cells take more device pixels, so it says so too.
    onRatioChanged: say()
    readonly property bool asleep: window.scene.asleep
    onAsleepChanged: say()
    readonly property bool stretching: window.scene.stretching
    onStretchingChanged: say()
    // A scene's pose changes nothing of its place, so it says so too.
    readonly property string posed: window.scene.posed
    onPosedChanged: say()
    // Nor does a hop's height, which says each frame of it, and asks
    // niri of the frame it moves, as a walk's step does.
    onLiftedChanged: {
        say();
        if (hopping)
            steps += 1;
        if (hopping && window.FramePresentation.available)
            window.FramePresentation.ask();
    }
    Component.onCompleted: {
        follow();
        say();
    }

    Item {
        id: cat

        // On the screen's own pixels, so every cell's edge is sharp,
        // a hop's height too.
        x: window.place ? Math.round(window.place.x * window.ratio) / window.ratio : 0
        // Held inside the margin, lifted by a hop, but on a move down,
        // carried up off it and climbing back.
        y: !window.place ? 0
            : window.moving !== null
            ? Math.round(window.place.y * window.ratio) / window.ratio
            : Math.max(0, Math.round((Math.min(window.place.y, window.height - window.size)
                - window.lifted) * window.ratio) / window.ratio)
        width: window.size
        height: window.size

        Repeater {
            model: Pixels.runs(window.drawn)

            Rectangle {
                required property var modelData

                x: modelData.x * window.cell
                y: modelData.y * window.cell
                width: modelData.across * window.cell
                height: window.cell
                antialiasing: false
                color: Theme.voice
            }
        }
    }
}
