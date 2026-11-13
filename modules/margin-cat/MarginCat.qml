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
// Do Not Disturb, on a call, locked, and where no screen draws it. This is
// the scene every screen shares; each screen's cat, where it sits and how
// it moves and is drawn, is MarginCatScreen.qml's.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick
import "MarginCatPlays.js" as Plays

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
    // What's left of the scene under way, each beat as Plays.scene says,
    // and the scene waiting comes to, or null where none comes.
    property var beats: []
    property var coming: null
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
        const next = Plays.next(lively, Math.random, played);
        coming = next?.scene ?? null;
        if (next === null) {
            waiting.stop();
        } else {
            // Sooner by its settings' pace, as a test asks to see scenes.
            waiting.interval = Math.max(1, Math.round(next.after / (settings.pace ?? 1)));
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
            if (root.coming === null)
                return;
            root.played = root.coming.name;
            root.beats = root.coming.beats;
            root.coming = null;
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

        MarginCatScreen {
            scene: root
        }
    }
}
