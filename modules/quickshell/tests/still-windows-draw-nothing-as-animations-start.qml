// Two windows, one still and one whose square starts moving ten times, run
// as the shell's root on niri by
// tests/still-windows-draw-nothing-as-animations-start-on-niri.sh
// (modules/quickshell/qtdeclarative-animations-wake-only-their-windows.patch).
// Once both have settled it counts each window's frames while the square
// moves, and fails if the still window drew any or the moving one none.
import Quickshell
import QtQuick

ShellRoot {
    id: root

    property int stillFrames: 0
    property int movingFrames: 0
    property bool counting: false
    property int starts: 0
    readonly property int startsAsked: 10

    function fail(why: string): void {
        console.error(why);
        Qt.callLater(() => Qt.exit(1));
    }

    PanelWindow {
        anchors {
            top: true
            left: true
        }
        implicitWidth: 200
        implicitHeight: 100
        color: "black"

        Item {
            id: stillCounted

            Connections {
                target: stillCounted.Window.window
                enabled: root.counting

                function onFrameSwapped() {
                    root.stillFrames += 1;
                }
            }
        }
    }

    PanelWindow {
        anchors {
            top: true
            right: true
        }
        implicitWidth: 200
        implicitHeight: 100
        color: "black"

        Rectangle {
            id: square

            width: 20
            height: 20
            color: "white"

            Connections {
                target: square.Window.window
                enabled: root.counting

                function onFrameSwapped() {
                    root.movingFrames += 1;
                }
            }

            NumberAnimation {
                id: moving

                target: square
                property: "x"
                from: 0
                to: 180
                duration: 100
            }
        }
    }

    // Long enough for both windows' first frames on a loaded builder, whose
    // software rasteriser can take most of a second a frame.
    Timer {
        running: true
        interval: 5000
        onTriggered: {
            root.counting = true;
            started.running = true;
        }
    }

    // Each animation ends well before the next starts, so every start is
    // one the animation driver hears as animations starting again.
    Timer {
        id: started

        interval: 400
        repeat: true
        onTriggered: {
            if (root.starts === root.startsAsked) {
                running = false;
                counted.running = true;
                return;
            }
            root.starts += 1;
            moving.restart();
        }
    }

    Timer {
        id: counted

        interval: 1000
        onTriggered: {
            root.counting = false;
            console.log(`Over ${root.starts} starts the still window drew ${root.stillFrames} frames and the moving one ${root.movingFrames}.`);
            if (root.movingFrames === 0) {
                root.fail("The moving window drew no frame: nothing was shown moving.");
                return;
            }
            if (root.stillFrames > 0) {
                root.fail("The still window drew frames that showed nothing new.");
                return;
            }
            Qt.callLater(() => Qt.exit(0));
        }
    }
}
