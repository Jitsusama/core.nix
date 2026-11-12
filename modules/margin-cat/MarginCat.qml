// The margin's cat: the empty places' cat (the theme's `pixels`), sitting
// on the top edge of the column Joel is in, in the gap the windows leave
// above them, where MarginCat.js says (VKCF3Z's fun for Joel, 2026-11-12:
// "something that crawls around the screen, unobtrusive and cute"). It
// lives on a layer of its own that takes no keys and no pointer and moves no
// window, gone while the screen is shared, and gone where the
// gap can't hold it, as in fullscreen or the overview. It's
// still: it draws a frame only as the column under it changes, and none at
// rest, so the closed shell draws nothing for it.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Wayland
import QtQuick
import "MarginCat.js" as MarginCat
import "../Pixels.js" as Pixels

Scope {
    id: root

    // Sitting, its tail curled up beside it.
    readonly property var drawing: Theme.pixels.nothingHere ?? []
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

            // The window the keys are in, as niri says its tile will come to
            // rest, when it's on this screen's shown workspace.
            readonly property var column: {
                void root.settled;
                void Niri.revision;
                if (root.shared || Niri.overview || Niri.current === null)
                    return null;
                const shown = Niri.windows[Niri.current];
                const at = shown?.layout?.tile_pos_in_workspace_view;
                const workspace = shown ? Niri.workspaces[shown.workspace_id] : null;
                if (!at || workspace?.output !== modelData.name || !workspace.is_active)
                    return null;
                return { left: at[0], top: at[1], width: shown.layout.tile_size[0] };
            }
            // Each cell in whole device pixels, and the cat's size from them.
            // The window's own ratio, which a fractional scale sets, rather
            // than the screen's, as Hairline.qml reads it.
            readonly property real ratio: cat.Window.window?.devicePixelRatio
                ?? (cat.Screen.devicePixelRatio > 0 ? cat.Screen.devicePixelRatio : 1)
            readonly property real cell: MarginCat.cell(Theme.gap, ratio) / ratio
            readonly property real size: Math.max(1, root.drawing.length) * cell
            readonly property var place: MarginCat.sits({
                column,
                // The screen's, since the window's own is 0 until it's shown.
                across: modelData.width,
                marks: MarginMarks.drawn,
                cell: Theme.gap,
                wide: size,
                tall: size,
                air: cell
            })

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
                    hue: `${Theme.voice}`
                });
                Report.say(`sits cat ${modelData.name} ${at} over ${JSON.stringify(column)}`);
            }
            onPlaceChanged: say()
            onColumnChanged: say()
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
