// The margin's cat: the empty places' cat (the theme's `pixels`), sitting
// on the top edge of the column Joel is in, in the gap the windows leave
// above them, where MarginCat.js says (VKCF3Z's fun for Joel, 2026-11-12:
// "something that crawls around the screen, unobtrusive and cute"). It
// lives on a layer of its own that takes no keys and no pointer and moves no
// window, gone while the screen is shared, and gone where the
// gap can't hold it, as in fullscreen or the overview. Once the keys rest
// two seconds on another column it walks there, a frame each time the
// screen draws, with the beats MarginCat.js gives it; otherwise it draws a
// frame only as the column under it changes, and none at rest, so the
// closed shell draws nothing for it. After ten minutes with
// no keys it curls up asleep, one frame, and wakes as they come again; as
// the screen unlocks it stretches, then sits. Between, it's livelier the
// less Joel is engaged (MarginCat.js's level): while he types, one small
// move held a moment, about once a minute; ten seconds with no keys, mild
// scenes a little more often; a minute, sometimes a stroll to the gutter
// beside its column and back, on the same walk. Each step comes from an
// idle timer, never a poll, a key ends any scene at once, and it keeps
// still in Do Not Disturb.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick
import "MarginCat.js" as MarginCat
import "../Pixels.js" as Pixels

Scope {
    id: root

    // Asleep once nothing's been typed or moved for as long as its settings
    // say, ten minutes on the laptop, since it's livelier the less Joel is
    // engaged and sleeps only once he's gone ("mild movements when i'm
    // active, increeeasing as I'm less engaged", 2026-11-12); whatever keeps
    // the screen awake, since a video playing is still no keys.
    readonly property bool asleep: dozing.isIdle
    // Its settings, from its module (modules/margin-cat/home.nix), read once
    // as it starts.
    readonly property var settings: {
        try {
            return JSON.parse(settingsFile.text());
        } catch (_) {
            return {};
        }
    }
    // Stretching for a moment as the lock lets the screens go ("when you
    // unlock, it stretches"), one pose held and then sitting, no tween.
    property bool stretching: false
    // What the lock last said it was, so only a lock let go stretches it,
    // never the shell starting unlocked.
    property string lockSaid: ""
    // Sitting, its tail curled up beside it, curled up alone asleep, a z
    // above it, or stretching: the theme's own poses, never copies of them.
    // Not For You's empty tray, which means nothing's for him (VKCF3Z
    // [a3b1144a]).
    readonly property var drawing: (stretching ? Theme.pixels.stretching
        : asleep ? Theme.pixels.curled
        : posed !== "" ? Theme.pixels[posed] ?? Theme.pixels.nothingHere
        : Theme.pixels.nothingHere) ?? []
    // How lively it may be, from how engaged Joel is: typing, paused ten
    // seconds, idle a minute, asleep, or still in Do Not Disturb.
    readonly property string lively: MarginCat.level({
        paused: pausing.isIdle,
        idle: idling.isIdle,
        asleep: root.asleep,
        quiet: Quiet.on
    })
    // The pose a scene holds now, by the theme's name, or "" for sitting.
    property string posed: ""
    // What's left of the scene under way, each beat as MarginCat.scene says.
    property var beats: []
    // Where a stroll asks the cat to walk, "gutter" or "column", or "" when
    // none is under way; and whether a screen's cat took the walk, so a
    // stroll nothing could take moves on rather than waiting.
    property string strollTo: ""
    property bool strollTaken: false

    // Each level a fresh rest, so a key ends whatever scene was under way,
    // a stroll included, and the cat sits at once.
    onLivelyChanged: begin()

    function begin(): void {
        beating.stop();
        beats = [];
        posed = "";
        strollTo = "";
        const wait = MarginCat.rest(lively, Math.random());
        if (wait === null) {
            waiting.stop();
        } else {
            // Sooner by its settings' pace, as a test asks to see scenes.
            waiting.interval = Math.max(1, Math.round(wait / (settings.pace ?? 1)));
            waiting.restart();
        }
    }

    // The next beat of the scene, or a fresh rest once it's done. A walk's
    // beat goes to the screen whose cat takes it, which hands back the next
    // as it sits; one nothing takes moves straight on.
    function beat(): void {
        if (beats.length === 0) {
            begin();
            return;
        }
        const next = beats[0];
        beats = beats.slice(1);
        if (next.walk === true) {
            posed = "";
            strollTaken = false;
            strollTo = next.to;
            if (!strollTaken)
                beat();
            return;
        }
        posed = next.pose;
        beating.interval = next.ms;
        beating.restart();
    }

    IdleMonitor {
        id: pausing
        timeout: root.settings.pauses ?? 10
        respectInhibitors: false
    }

    IdleMonitor {
        id: idling
        timeout: root.settings.idles ?? 60
        respectInhibitors: false
    }

    Timer {
        id: waiting
        onTriggered: {
            root.beats = MarginCat.scene(root.lively, Math.random());
            root.beat();
        }
    }

    Timer {
        id: beating
        onTriggered: root.beat()
    }
    // While the screen is shared, as a call shares it through the portal,
    // the cat isn't there to be seen: a layer niri keeps from a capture is a
    // black square in it, so home.nix's layer rule only covers the moment
    // before the shell hears. A screencopy, as grim's screenshot or
    // wf-recorder's recording, is Joel's own picture, and shows it as it
    // shows the margin; each shot is a cast to niri, and leaving for one
    // would leave it out of every one.
    readonly property bool shared: Object.values(Niri.casts)
        .some(cast => cast.is_active && cast.kind === "PipeWire")
    // Counts niri's layout changes, since a window moving says so only
    // through layoutsChanged, not through its windows' revision.
    property int settled: 0

    // The windows daemon says where a tile is only while something wants
    // it, and the cat follows the one Joel's in, so it hears that tile
    // alone, and none while it's away for a share or the overview.
    readonly property var over: shared || Niri.overview ? null : Niri.current
    onOverChanged: Niri.wantTile("cat", over)
    Component.onCompleted: {
        Niri.wantTile("cat", over);
        begin();
    }
    Component.onDestruction: Niri.wantTile("cat", null)

    IdleMonitor {
        id: dozing
        timeout: root.settings.dozes ?? 600
        respectInhibitors: false
    }

    FileView {
        id: settingsFile
        path: Quickshell.shellPath("surfaces/MarginCat.json")
        blockAllReads: true
    }

    function heardLock(word: string): void {
        if (root.lockSaid === "locked" && word === "unlocked") {
            root.stretching = true;
            stretched.restart();
        }
        root.lockSaid = word;
    }

    // The lock's own word, written as it takes and lets go of the screens
    // (Lock.qml), watched rather than asked again, so it adds no wakes.
    FileView {
        id: lockWord
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/quickshell-locked`
        watchChanges: true
        blockAllReads: true
        printErrors: false
        onLoaded: root.heardLock(lockWord.text().trim())
        onFileChanged: {
            lockWord.reload();
            root.heardLock(lockWord.text().trim());
        }
    }

    // About half a second, long enough to be seen as a stretch and short of
    // the keys he'll press next (XY9H5C's pose: "held about half a second").
    Timer {
        id: stretched
        interval: 500
        onTriggered: root.stretching = false
    }

    Connections {
        target: Niri

        function onLayoutsChanged() {
            root.settled++;
        }
    }

    Variants {
        model: Quickshell.screens.filter(screen => screen.name !== "")

        PanelWindow {
            id: window

            required property ShellScreen modelData

            // The window the keys are in, where it's on this screen's shown
            // workspace, and the cat's to sit over.
            readonly property var wanted: {
                void root.settled;
                void Niri.revision;
                if (root.shared || Niri.overview || Niri.current === null)
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
                void root.settled;
                void Niri.revision;
                return root.shared || Niri.overview ? null : tile(wanted);
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
            readonly property real size: Math.max(1, root.drawing.length) * cell
            // Where it'd sit over the keys' column, or in the gutter beside
            // it while a stroll has it there, and where it's drawn: there, a
            // step of its walk, or still where it sat while it waits for the
            // keys to rest.
            // It reads nothing follow() writes, since a write there would
            // work it out again inside its own change, a binding loop.
            readonly property var there: column === null ? null
                : strolled ?? seat(column)
            // A stroll's: the gutter seat it walks to or sits at, or null for
            // its column; whether the walk under way is a scene's, which
            // hands the scene its next beat as it sits; and whether the next
            // move of its seat walks rather than carries it there, hurried
            // (true, a key bringing it back) or not (false), or null for a
            // column moving, which carries it.
            property var strolled: null
            property bool handsBack: false
            property var roams: null
            property var place: null
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
            // What it's drawn as: a pose of its walk, turned the way it
            // goes, or the scope's, sitting, asleep or stretching.
            readonly property var drawn: pose
                ? MarginCat.turned(Theme.pixels[pose] ?? root.drawing, facing)
                : root.drawing

            // Where the keys' column now has it: nowhere, once it's held
            // nowhere a moment (leaving); from
            // nowhere, there at once, since nothing walks in from nowhere;
            // over the same window, carried as its column moves, or, on
            // the way there, on from where it is where its seat moved a
            // pixel; and over another, still where it is until the keys rest
            // there two seconds. niri says the layout again for any window's
            // change, a title's too, so the same seat neither sets a walk off
            // again nor restarts its rest.
            function follow(): void {
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
                    walkFrom(hurried);
                } else if (place === null || (going === null && wanted === sat)) {
                    resting.stop();
                    sat = wanted;
                    place = there;
                } else if (going !== null && wanted === goingTo) {
                    if (Math.round(there.x * ratio) !== Math.round(going.to * ratio))
                        walkFrom(true);
                } else if (going !== null || !resting.running || wanted !== waitingOn) {
                    halt();
                    // Its gutter forgotten a moment later, outside this
                    // change of `there`, which forgetting it changes again.
                    Qt.callLater(endStroll);
                    place = { x: place.x, y: there.y };
                    waitingOn = wanted;
                    resting.restart();
                }
            }

            // Sets off from where it's drawn to where it'd sit: `hurried`,
            // its seat moved on the way, without turning, thinking or
            // crouching again, or, setting off, after a thought of 150 to
            // 450 ms, so no two walks begin alike.
            function walkFrom(hurried: bool): void {
                roams = null;
                if (place === null || there === null)
                    return;
                const plan = MarginCat.walk(place.x, there.x, {
                    cell,
                    ratio,
                    pause: 150 + Math.random() * 300,
                    slowdown: Theme.motion.slowdown,
                    hurried,
                    facing: stepped?.facing ?? 0
                });
                goingTo = wanted;
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
                }
                going = plan;
                walker.restart();
                if (window.FramePresentation.available)
                    window.FramePresentation.ask();
            }

            // How its walk reached the screen, in the journal, so a walk on
            // Joel's screen says whether it kept to its rate.
            function tell(): void {
                if (!window.FramePresentation.available)
                    return;
                const told = MarginCat.walked(shown, costs, unseen);
                console.info(told.line);
                if (Options.reports)
                    Report.say(`walked cat ${modelData.name} ${JSON.stringify(told.said)}`);
            }

            // Each frame of a walk, as niri showed it or didn't, asking after
            // the next until it sits.
            FramePresentation.onPresented: (frame, at) => {
                if (going === null)
                    return;
                shown.push(at);
                const times = window.FramePresentation.timesOf(frame);
                if (times.rendered > 0 && times.synchronizing > 0)
                    costs.push(times.rendered - times.synchronizing);
                window.FramePresentation.ask();
            }
            FramePresentation.onDiscarded: () => {
                if (going === null)
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

            // A scene's walk done, handing the scene its next beat.
            function sits(): void {
                if (!handsBack)
                    return;
                handsBack = false;
                Qt.callLater(root.beat);
            }

            // A stroll ended by its column going or another's taking the
            // keys: its gutter is forgotten, and the scene moves on rather
            // than waiting on a walk that's gone.
            function endStroll(): void {
                strolled = null;
                roams = null;
                sits();
            }

            // A stroll's walk, where the scene says, taken only by the screen
            // whose cat sits still over the column the keys are in: to the
            // gutter beside it, then back over it, each with a thought and a
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
                if (place === null || going !== null || resting.running || wanted !== sat)
                    return;
                if (to === "column") {
                    if (strolled === null)
                        return;
                    root.strollTaken = true;
                    handsBack = true;
                    roams = false;
                    strolled = null;
                    return;
                }
                const spot = MarginCat.gutter({
                    column,
                    across: modelData.width,
                    marks: MarginMarks.drawn,
                    cell: Theme.gap,
                    wide: size,
                    tall: size,
                    air: cell
                });
                if (spot === null)
                    return;
                root.strollTaken = true;
                handsBack = true;
                roams = false;
                strolled = spot;
            }

            Connections {
                target: root

                function onStrollToChanged() {
                    window.stroll(root.strollTo);
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
                onTriggered: window.walkFrom(false)
            }

            // A step each time the screen draws, at whatever rate it draws,
            // and none once it sits.
            FrameAnimation {
                id: walker
                onTriggered: {
                    const now = MarginCat.step(window.going, walker.elapsedTime * 1000);
                    if (now.done) {
                        window.tell();
                        window.halt();
                        window.sat = window.goingTo;
                        window.place = window.there ?? window.place;
                        window.sits();
                        return;
                    }
                    if (now.facing !== window.facing)
                        window.facing = now.facing;
                    if (now.pose !== window.pose)
                        window.pose = now.pose;
                    window.stepped = now;
                    window.place = { x: now.x, y: window.there?.y ?? window.place.y };
                }
            }

            // A window's tile, as niri says it will come to rest, where it's on
            // this screen's shown workspace; null where it isn't.
            function tile(id: var): var {
                const shown = id === null ? undefined : Niri.windows[id];
                const at = shown?.layout?.tile_pos_in_workspace_view;
                const workspace = shown ? Niri.workspaces[shown.workspace_id] : null;
                if (!at || workspace?.output !== modelData.name || !workspace.is_active)
                    return null;
                return { left: at[0], top: at[1], width: shown.layout.tile_size[0] };
            }

            function seat(over: var): var {
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
            visible: !!place && root.drawing.length > 0
            anchors.top: true
            anchors.left: true
            anchors.right: true
            implicitHeight: Theme.gap
            exclusionMode: ExclusionMode.Ignore
            color: "transparent"
            WlrLayershell.layer: WlrLayer.Overlay
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
                    pose: root.posed,
                    strolling: handsBack || strolled !== null,
                    window: sat,
                    asleep: root.asleep,
                    stretching: root.stretching,
                    walk: going === null || stepped === null ? null : {
                        t: Math.round(walker.elapsedTime * 10000) / 10,
                        beat: stepped.beat,
                        pose: stepped.pose,
                        facing: stepped.facing,
                        stride: going.stride,
                        to: going.to
                    }
                });
                Report.say(`sits cat ${modelData.name} ${at} over ${JSON.stringify(column)}`);
            }
            onPlaceChanged: say()
            onColumnChanged: say()
            // A new scale can leave its logical place and size as they were
            // while its cells take more device pixels, so it says so too.
            onRatioChanged: say()
            readonly property bool asleep: root.asleep
            onAsleepChanged: say()
            readonly property bool stretching: root.stretching
            onStretchingChanged: say()
            // A scene's pose changes nothing of its place, so it says so too.
            readonly property string posed: root.posed
            onPosedChanged: say()
            Component.onCompleted: {
                follow();
                say();
            }

            Item {
                id: cat

                // On the screen's own pixels, so every cell's edge is sharp.
                x: window.place ? Math.round(window.place.x * window.ratio) / window.ratio : 0
                y: window.place
                    ? Math.max(0, Math.round(Math.min(window.place.y, window.height - window.size)
                        * window.ratio) / window.ratio)
                    : 0
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
    }
}
