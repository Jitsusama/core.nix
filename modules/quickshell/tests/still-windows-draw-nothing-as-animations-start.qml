// Three windows, run as the shell's root on niri by
// tests/still-windows-draw-nothing-as-animations-start-on-niri.sh
// (modules/quickshell/qtdeclarative-animations-wake-only-their-windows.patch):
// one still, one whose square a NumberAnimation moves on the GUI thread,
// and one whose square an XAnimator slides on the render thread, as the
// shell's reading slide does (ReadsOn.qml). Once each has drawn and all
// have gone quiet, both squares start moving ten times. It fails, exiting
// 2, if the still window drew any frame then, if either moving window drew
// fewer than two frames for any start, or if the still window draws none
// when its colour then changes, which shows it's still shown and still
// draws for a change of its own once the starts no longer ask it.
import Quickshell
import QtQuick

ShellRoot {
    id: root

    // Each window's frames since the shell started.
    property int stillDrawn: 0
    property int movingDrawn: 0
    property int slidingDrawn: 0
    // When any window last drew, so the counting starts once all are quiet.
    property real lastDrawn: 0
    property real began: Date.now()
    // Each window's frames as the counting, or the latest start, began.
    property int stillBefore: 0
    property int movingBefore: 0
    property int slidingBefore: 0
    property int starts: 0
    // The starts a moving window drew too few frames for, as "3 (moving 1)".
    property var shortStarts: []
    readonly property int startsAsked: 10
    // A start's first frame, and at least one of its ticks drawn: a 250 ms
    // animation draws about fifteen at 60 Hz, so a loaded builder has room.
    readonly property int fewestEachStart: 2

    function fail(why: string): void {
        console.error(why);
        Qt.callLater(() => Qt.exit(2));
    }

    function drew(): void {
        root.lastDrawn = Date.now();
    }

    // How many frames each moving window drew for the start just ended.
    function startEnded(): void {
        const movedFrames = root.movingDrawn - root.movingBefore;
        const slidFrames = root.slidingDrawn - root.slidingBefore;
        if (movedFrames < root.fewestEachStart || slidFrames < root.fewestEachStart)
            root.shortStarts.push(
                `${root.starts} (moving ${movedFrames}, sliding ${slidFrames})`);
    }

    PanelWindow {
        id: stillWindow

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

                function onFrameSwapped() {
                    root.stillDrawn += 1;
                    root.drew();
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

                function onFrameSwapped() {
                    root.movingDrawn += 1;
                    root.drew();
                }
            }

            NumberAnimation {
                id: moving

                target: square
                property: "x"
                from: 0
                to: 180
                duration: 250
            }
        }
    }

    PanelWindow {
        anchors {
            bottom: true
            left: true
        }
        implicitWidth: 200
        implicitHeight: 100
        color: "black"

        Rectangle {
            id: slider

            width: 20
            height: 20
            color: "white"

            Connections {
                target: slider.Window.window

                function onFrameSwapped() {
                    root.slidingDrawn += 1;
                    root.drew();
                }
            }

            // No `from`, as ReadsOn.qml's slide back has none, so starting
            // it changes nothing on the GUI thread: only the animator's own
            // sync asks its window for a frame.
            XAnimator {
                id: sliding

                target: slider
                duration: 250
            }
        }
    }

    // Waits on what the windows did, not on a clock: each has drawn its
    // first frame, and none has drawn for a second since, so a late first
    // frame or niri's configure on a loaded builder isn't counted.
    Timer {
        id: settling

        running: true
        interval: 250
        repeat: true
        onTriggered: {
            const drawn = root.stillDrawn > 0 && root.movingDrawn > 0 && root.slidingDrawn > 0;
            if (drawn && Date.now() - root.lastDrawn >= 1000) {
                running = false;
                root.stillBefore = root.stillDrawn;
                started.running = true;
                return;
            }
            if (Date.now() - root.began > 60000) {
                running = false;
                root.fail("The windows never settled in 60 s: the still one drew "
                    + `${root.stillDrawn} frames, the moving one ${root.movingDrawn} `
                    + `and the sliding one ${root.slidingDrawn}.`);
            }
        }
    }

    // Each animation ends well before the next starts, so every start is
    // one the animation driver hears as animations starting again.
    Timer {
        id: started

        interval: 600
        repeat: true
        onTriggered: {
            if (root.starts > 0)
                root.startEnded();
            if (root.starts === root.startsAsked) {
                running = false;
                root.counted();
                return;
            }
            root.starts += 1;
            root.movingBefore = root.movingDrawn;
            root.slidingBefore = root.slidingDrawn;
            moving.restart();
            sliding.to = slider.x < 90 ? 180 : 0;
            sliding.restart();
        }
    }

    function counted(): void {
        const still = root.stillDrawn - root.stillBefore;
        console.log(`Over ${root.starts} starts the still window drew ${still} frames; `
            + `since the shell started, the moving one drew ${root.movingDrawn} `
            + `and the sliding one ${root.slidingDrawn}.`);
        if (root.shortStarts.length > 0) {
            root.fail(`A moving window drew fewer than ${root.fewestEachStart} frames `
                + `for starts ${root.shortStarts.join(", ")}.`);
            return;
        }
        if (still > 0) {
            root.fail("The still window drew frames that showed nothing new.");
            return;
        }
        root.stillBefore = root.stillDrawn;
        stillWindow.color = "#202020";
        heard.running = true;
    }

    // A change of its own must draw: the settling already showed its frames
    // are heard, and this shows that after ten starts it's still shown and
    // still answers its own update, so the 0 above isn't a window niri hid
    // or one the patch left deaf to its own changes.
    Timer {
        id: heard

        interval: 2000
        onTriggered: {
            if (root.stillDrawn === root.stillBefore) {
                root.fail("The still window drew nothing when its colour changed, "
                    + "so it no longer draws for its own changes.");
                return;
            }
            Qt.callLater(() => Qt.exit(0));
        }
    }
}
