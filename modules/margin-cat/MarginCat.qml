// The margin's cat: the empty places' cat (the theme's `pixels`), sitting
// on the top edge of the column Joel is in, in the gap the windows leave
// above them, where MarginCat.js says (VKCF3Z's fun for Joel, 2026-11-12:
// "something that crawls around the screen, unobtrusive and cute"). It
// lives on a layer of its own that takes no keys and no pointer and moves no
// window, gone while the screen is shared, and gone where the
// gap can't hold it, as in fullscreen or the overview. It's
// still: it draws a frame only as the column under it changes, and none at
// rest, so the closed shell draws nothing for it. After ten minutes with
// no keys it curls up asleep, one frame, and wakes as they come again; as
// the screen unlocks it stretches, then sits.
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
        : asleep ? Theme.pixels.curled : Theme.pixels.nothingHere) ?? []
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

    // The windows daemon says where tiles are only while something wants
    // them, and the cat follows the one Joel's in.
    Component.onCompleted: Niri.wantTiles("cat", true)
    Component.onDestruction: Niri.wantTiles("cat", false)

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
            // The window whose column the cat sits over: the one the keys
            // are in, at once. It walks nowhere until a walk moves smoothly
            // at the display's rate, with character (Joel, 2026-11-12: "the
            // cat is not animated, it just jerkily moves").
            readonly property var sat: wanted
            // Its column, as niri says its tile will come to rest, so it
            // moves with the column as niri lays them out again.
            readonly property var column: {
                void root.settled;
                void Niri.revision;
                return root.shared || Niri.overview ? null : tile(sat);
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
            // Where it sits over its column.
            readonly property var place: seat(column)

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
            visible: place !== null && root.drawing.length > 0
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
            // can count them on the screen.
            function say(): void {
                if (!Options.reports)
                    return;
                const at = place === null ? "nowhere" : JSON.stringify({
                    x: place.x,
                    y: place.y,
                    size: window.size,
                    air: window.cell,
                    cell: Math.round(window.cell * window.ratio),
                    cells: root.drawing.length,
                    lit: root.drawing.join("").split("#").length - 1,
                    hue: `${Theme.voice}`,
                    window: sat,
                    asleep: root.asleep,
                    stretching: root.stretching
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
            Component.onCompleted: say()

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
                    model: Pixels.runs(root.drawing)

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
