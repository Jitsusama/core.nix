// A full-screen window showing a picture with a square set over it, drawn
// flat (surfaceFormat.flat, modules/quickshell/window-drawn-flat.patch)
// where FLAT is 1 and with Qt Quick's depth and stencil buffers otherwise.
// Run as the shell's root on niri, twice, by
// tests/flat-windows-hold-no-depth-on-niri.sh, which weighs what the
// buffers hold as the difference between the two and looks at what niri
// shows of the flat one. Each says what the process weighs once it's
// drawn, then stays up while niri's screenshot is taken. A see-through
// window over it draws what the margin, the arrival line and the talk line
// draw, flat or not alike, which the script holds the two runs to drawing
// the same.
import Quickshell
import Quickshell.Wayland
import QtQuick
import QtQuick.Effects
import QtQuick.Shapes
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

    // What the lines draw on a see-through window: a card that clips,
    // words cut at its edge and one turned inside it, as a turned mark is,
    // a see-through square over an opaque one, a shadow and rounded
    // corners each drawn through a layer, as Lifted and Cover draw them, and
    // a scaled shape drawn by the curve renderer, as the margin's marks are.
    PanelWindow {
        anchors {
            top: true
            left: true
        }
        implicitWidth: 720
        implicitHeight: 360
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        surfaceFormat.flat: root.flat
        WlrLayershell.layer: WlrLayer.Overlay

        Rectangle {
            x: 8
            y: 8
            width: 704
            height: 344
            radius: 24
            color: "blue"
            clip: true

            // Cut at the card's edge whether a font is found or not.
            Rectangle {
                x: 600
                y: 140
                width: 300
                height: 60
                color: "white"
            }

            Text {
                x: 560
                y: 40
                text: "cut at the edge"
                font.pixelSize: 64
                color: "white"
            }

            Text {
                x: 60
                y: 240
                text: "turned"
                font.pixelSize: 48
                color: "yellow"
                rotation: 180
            }

            Rectangle {
                x: 60
                y: 40
                width: 120
                height: 120
                color: "red"
            }

            Rectangle {
                x: 120
                y: 100
                width: 120
                height: 120
                color: "#8000ff00"
            }
        }

        Item {
            x: 300
            y: 40
            width: 160
            height: 120
            layer.enabled: true
            layer.effect: MultiEffect {
                shadowEnabled: true
                shadowBlur: 1
                shadowColor: "black"
            }

            Rectangle {
                anchors.fill: parent
                anchors.margins: 20
                radius: 12
                color: "orange"
            }
        }

        Rectangle {
            id: corners

            x: 500
            y: 200
            width: 160
            height: 120
            radius: 40
            visible: false
            layer.enabled: true
        }

        Rectangle {
            x: 500
            y: 200
            width: 160
            height: 120
            color: "magenta"
            layer.enabled: true
            layer.effect: MultiEffect {
                maskEnabled: true
                maskSource: corners
            }
        }

        Shape {
            x: 300
            y: 200
            width: 40
            height: 40
            scale: 3
            transformOrigin: Item.TopLeft
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                fillColor: "cyan"
                strokeWidth: -1
                fillRule: ShapePath.OddEvenFill

                PathSvg {
                    path: "M 0 20 A 20 20 0 1 1 40 20 A 20 20 0 1 1 0 20 Z "
                        + "M 10 20 A 10 10 0 1 0 30 20 A 10 10 0 1 0 10 20 Z"
                }
            }
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
