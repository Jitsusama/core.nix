// A full-screen window showing a picture with a square set over it, drawn
// flat (surfaceFormat.flat, modules/quickshell/window-drawn-flat.patch)
// where FLAT is 1 and with Qt Quick's depth and stencil buffers otherwise.
// Run as the shell's root on niri, twice, by
// tests/flat-windows-hold-no-depth-on-niri.sh, which weighs what the
// buffers hold as the difference between the two and looks at what niri
// shows of the flat one. Each says what the process weighs once it's
// drawn, then stays up while niri's screenshot is taken.
import Quickshell
import Quickshell.Wayland
import QtQuick
import "Memory.js" as Memory

ShellRoot {
    id: root

    readonly property bool flat: Quickshell.env("FLAT") === "1"

    function fail(why: string): void {
        console.error(why);
        Qt.callLater(() => Qt.exit(1));
    }

    PanelWindow {
        anchors {
            top: true
            bottom: true
            left: true
            right: true
        }
        color: "black"
        surfaceFormat.flat: root.flat
        WlrLayershell.layer: WlrLayer.Background

        // Both opaque, so Qt Quick draws them in its opaque pass: front to
        // back against the depth buffer where there is one, and back to
        // front without, where drawn front to back the picture would cover
        // the square.
        Image {
            id: picture

            anchors.fill: parent
            fillMode: Image.Stretch
            source: "file://" + Quickshell.env("PICTURE")
            onStatusChanged: {
                if (status === Image.Error)
                    root.fail(`The picture wasn't read: ${source}`);
            }
        }

        Rectangle {
            anchors.centerIn: parent
            width: parent.width / 4
            height: parent.height / 4
            color: "lime"
        }
    }

    // Long enough for the window's first frames: on a loaded builder Mesa's
    // rasteriser takes over a second a frame at 2880 by 1800.
    Timer {
        running: true
        interval: 10000
        onTriggered: {
            if (picture.status !== Image.Ready) {
                root.fail(`The picture wasn't ready in ${interval} ms`);
                return;
            }
            gc();
            console.info(`flat windows: weighs ${Math.round(Memory.anonymous() / 1024)} KB`
                + (root.flat ? " drawn flat" : " drawn with depth"));
            done.start();
        }
    }

    // Up while the script takes niri's screenshot.
    Timer {
        id: done

        interval: 20000
        onTriggered: Qt.exit(0)
    }
}
