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
// closed shell draws nothing for it. After three minutes with
// no keys it curls up asleep, one frame, and wakes as they come again; as
// the screen unlocks it stretches, then sits. Between, it's livelier the
// less Joel is engaged (MarginCat.js's level): while he types, one small
// move held a moment, about once a minute; ten seconds with no keys,
// gentle plays in place every few seconds, at random; a minute, sometimes
// a hop or a short wander and back, on the same walk. Each wait is one
// timer, never a poll, a key ends any play at once, and it keeps still in
// Do Not Disturb, on a call, locked, and where no screen draws it.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick
import "MarginCat.js" as MarginCat
import "MarginCatPlays.js" as Plays
import "../Pixels.js" as Pixels

Scope {
    id: root

    // Asleep once nothing's been typed or moved for as long as its settings
    // say, three minutes on the laptop, since it's livelier the less Joel is
    // engaged ("mild movements when i'm active, increeeasing as I'm less
    // engaged", 2026-11-12) and plays about two minutes once he's gone,
    // then sleeps rather than playing for an empty room (VKCF3Z, eb64260b);
    // whatever keeps the screen awake, since a video playing is still no
    // keys.
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
    // seconds, idle a minute, or asleep; still in Do Not Disturb, on a
    // call, locked, or drawn on no screen.
    readonly property string lively: Plays.level({
        paused: pausing.isIdle,
        idle: idling.isIdle,
        asleep: root.asleep,
        quiet: Quiet.on,
        onCamera: root.onCamera,
        locked: root.lockSaid === "locked",
        unseen: root.placedOn === 0
    })
    // How many screens draw it now, each window counting itself in and out.
    property int placedOn: 0
    // Whether a camera is in use, as on a call, from the machine daemon's
    // camera records, which it tells only as a program comes or goes, so
    // watching them wakes nothing in between.
    readonly property bool onCamera: {
        void cameras.revision;
        return Object.values(cameras.held).some(camera => (camera.programs ?? []).length > 0);
    }
    // The pose a scene holds now, by the theme's name, or "" for sitting.
    property string posed: ""
    // The play under way or last played, so none comes twice running.
    property string played: ""
    // What's left of the scene under way, each beat as Plays.scene says.
    property var beats: []
    // A hop under way, its rise's `ms` and how many `cells` up, and how far
    // up it is now, from none to its height (1), each frame the screen
    // draws; null and 0 on the ground.
    property var hop: null
    property real lift: 0
    // A wander's cells and way, for the screen whose cat takes it.
    property var wandering: null
    // Where a stroll asks the cat to walk, "wander" or "column", or "" when
    // none is under way; and whether a screen's cat took the walk, so a
    // stroll nothing could take moves on rather than waiting.
    property string strollTo: ""
    property bool strollTaken: false

    // Each level a fresh rest, so a key ends whatever scene was under way,
    // a stroll included, and the cat sits at once.
    onLivelyChanged: begin()

    function begin(): void {
        beating.stop();
        land();
        beats = [];
        posed = "";
        strollTo = "";
        const wait = Plays.rest(lively, Math.random());
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
        land();
        const next = beats[0];
        beats = beats.slice(1);
        if (next.walk === true) {
            posed = "";
            strollTaken = false;
            if (next.to === "wander")
                wandering = { cells: next.cells, way: next.way };
            strollTo = next.to;
            if (!strollTaken)
                beat();
            return;
        }
        posed = next.pose;
        if (next.hop === true) {
            hop = { ms: next.ms, cells: next.cells };
            hopper.restart();
        }
        beating.interval = next.ms;
        beating.restart();
    }

    // Back on the ground, a hop over or cut short.
    function land(): void {
        hopper.stop();
        hop = null;
        lift = 0;
    }

    // A hop's rise each frame the screen draws, and none on the ground.
    FrameAnimation {
        id: hopper
        onTriggered: {
            if (root.hop !== null)
                root.lift = Plays.hopped(hopper.elapsedTime * 1000, root.hop.ms, 1);
        }
    }

    Daemon {
        id: cameras

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: "camera"
        keptThroughGap: false
        answersWhole: true
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
            const next = Plays.scene(root.lively, Math.random, root.played);
            root.played = next.name;
            root.beats = next.beats;
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
        timeout: root.settings.dozes ?? 180
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
            // many it drew.
            property int begunAt: 0
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
            // The move between workspaces under way (MarginCat.moves), the
            // window it climbs to once niri says it, and its quarter turn as it
            // climbs, head first.
            property var moving: null
            property var movingTo: null
            property int turn: 0
            // What it's drawn as: a pose of its walk, turned the way it
            // goes and the way it climbs, or the scope's, sitting, asleep or
            // stretching.
            readonly property var drawn: pose
                ? MarginCat.quartered(
                    MarginCat.turned(Theme.pixels[pose] ?? root.drawing, facing), turn)
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
                    // Its wander forgotten a moment later, outside this
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
                    begunAt = window.FramePresentation.framesBegun;
                }
                going = plan;
                walker.restart();
                if (window.FramePresentation.available)
                    window.FramePresentation.ask();
            }

            // A move to the workspace `way` below (1) or above (-1): it rides
            // its column up the whole slide, or stays as it falls away, from
            // where it sits, and its new seat comes once niri says the new
            // workspace's column (steer).
            function moveFrom(way: int): void {
                halt();
                resting.stop();
                leaving.stop();
                movingTo = null;
                moving = plan(way, place, place);
                shown = [];
                costs = [];
                unseen = 0;
                begunAt = window.FramePresentation.framesBegun;
                mover.restart();
                if (window.FramePresentation.available)
                    window.FramePresentation.ask();
            }

            // A move's plan from `from` to its seat `to`, over a workspace's
            // height and niri's gap between two, a tenth of the screen's
            // height in whole device pixels, as niri's monitor.rs lays them.
            function plan(way: int, from: var, to: var): var {
                const gap = Math.max(1, Math.round(modelData.height * 0.1 * ratio)) / ratio;
                return MarginCat.moves(from, to, {
                    way,
                    step: modelData.height + gap,
                    tall: size,
                    cell,
                    ratio,
                    facing,
                    slowdown: Theme.motion.slowdown,
                    spring: Theme.motion.spring
                });
            }

            // Where a move goes as niri says the new workspace's column: on
            // the ride, to its seat, again each time that moves, since none of
            // the ride depends on it; once it climbs, a seat moved or the
            // keys gone to another window seat it there at once, as a second
            // move does (VKCF3Z, 074321bd).
            function steer(): void {
                if (there === null)
                    return;
                const riding = mover.elapsedTime * 1000 < moving.beats[0].ends;
                const seat = moving.seat ?? moving.to;
                const same = Math.round(there.x * ratio) === Math.round(seat.x * ratio)
                    && Math.round(there.y * ratio) === Math.round(seat.y * ratio);
                if (movingTo !== null && wanted !== movingTo) {
                    arrive();
                } else if (riding) {
                    if (movingTo === null || !same) {
                        movingTo = wanted;
                        moving = plan(moving.way, moving.from, there);
                    }
                } else if (movingTo === null || !same) {
                    arrive();
                }
            }

            // Done moving, or cut short: it sits where the keys' column has
            // it, at once, or nowhere, as it would have with no move; or, a
            // move up whose seat was too far to walk to at once, where it
            // stayed, until the keys rest and it walks there as ever.
            function arrive(): void {
                tell("walk");
                const rests = moving?.rests ?? false;
                mover.stop();
                moving = null;
                movingTo = null;
                turn = 0;
                halt();
                if (rests && place !== null && there !== null) {
                    follow();
                    return;
                }
                sat = wanted;
                place = there;
            }

            onSpaceChanged: {
                const was = spaceWas;
                spaceWas = space;
                if (was === null || space === null || was.id === space.id)
                    return;
                const way = Math.sign(space.idx - was.idx);
                if (moving !== null)
                    arrive();
                else if (place !== null && way !== 0 && !root.asleep && !Niri.overview
                    && !root.shared)
                    moveFrom(way);
            }

            // How its walk or hop (`what`) reached the screen, in the
            // journal, so one on Joel's screen says whether it kept to its
            // rate.
            function tell(what: string): void {
                if (!window.FramePresentation.available)
                    return;
                const told = MarginCat.walked(shown, costs, unseen,
                    { begun: window.FramePresentation.framesBegun - begunAt, what });
                console.info(told.line);
                if (Options.reports)
                    Report.say(`${what === "hop" ? "hopped" : "walked"} cat `
                        + `${modelData?.name ?? ""} ${JSON.stringify(told.said)}`);
            }

            // A hop drawn here, its frames counted as a walk's are; and how
            // far up it has the cat, in logical pixels: its height in cells,
            // or what room the gap leaves above it, if less.
            readonly property bool hopping: root.hop !== null && !!place
            readonly property real lifted: !place || root.hop === null ? 0
                : root.lift * Math.min(root.hop.cells, Math.max(0, place.y / cell)) * cell
            onHoppingChanged: {
                if (hopping) {
                    shown = [];
                    costs = [];
                    unseen = 0;
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
            onPlacedChanged: root.placedOn += placed ? 1 : -1
            Component.onDestruction: {
                if (placed)
                    root.placedOn -= 1;
            }

            // Each frame of a walk or a hop, as niri showed it or didn't,
            // asking after the next until it sits. Asked only here, once niri
            // says the last was shown, the ask lands after the next frame has
            // begun where frames come close together, so only every other
            // frame was heard of: on a GPU at 60 Hz, and on Joel's screen at
            // 120. Each step asks too (walker, and a hop's lift), before its
            // frame is drawn. Asked only there, a GPU at 120 Hz drew every
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

            // A scene's walk done, handing the scene its next beat.
            function sits(): void {
                if (!handsBack)
                    return;
                handsBack = false;
                Qt.callLater(root.beat);
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
                    root.strollTaken = true;
                    handsBack = true;
                    roams = false;
                    strolled = null;
                    return;
                }
                const spot = Plays.wander({
                    seat: seat(column),
                    across: modelData.width,
                    marks: MarginMarks.drawn,
                    cell: Theme.gap,
                    wide: size,
                    step: cell,
                    cells: root.wandering?.cells ?? 0,
                    way: root.wandering?.way ?? 1
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
                        window.tell("walk");
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
                    // Asked as the step is taken, before its frame is drawn,
                    // so the frame it moves is the one niri is asked about.
                    if (window.FramePresentation.available)
                        window.FramePresentation.ask();
                }
            }

            // A step of a move each time the screen draws, as the walker's.
            FrameAnimation {
                id: mover
                onTriggered: {
                    const now = MarginCat.moved(window.moving, mover.elapsedTime * 1000);
                    // A ride with no column at its end leaves it nowhere, as
                    // moving to an empty workspace did before it rode.
                    if (now.done
                        || (now.beat !== window.moving.beats[0].name && window.movingTo === null)) {
                        window.arrive();
                        return;
                    }
                    if (now.facing !== window.facing)
                        window.facing = now.facing;
                    if ((now.pose ?? "") !== window.pose)
                        window.pose = now.pose ?? "";
                    if (now.turn !== window.turn)
                        window.turn = now.turn;
                    window.stepped = now;
                    window.place = { x: now.x, y: now.y };
                    if (window.FramePresentation.available)
                        window.FramePresentation.ask();
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
                    lift: Math.round(window.lifted * window.ratio),
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
                    },
                    move: moving === null || stepped === null ? null : {
                        t: Math.round(mover.elapsedTime * 10000) / 10,
                        beat: stepped.beat,
                        pose: stepped.pose,
                        turn: stepped.turn,
                        way: moving.way
                    }
                });
                Report.say(`sits cat ${modelData?.name ?? ""} ${at} over ${JSON.stringify(column)}`);
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
            // Nor does a hop's height, which says each frame of it, and asks
            // niri of the frame it moves, as a walk's step does.
            onLiftedChanged: {
                say();
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
    }
}
