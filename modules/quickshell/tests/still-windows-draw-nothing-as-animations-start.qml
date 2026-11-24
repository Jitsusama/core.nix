// Three windows, run as the shell's root on niri by
// tests/still-windows-draw-nothing-as-animations-start-on-niri.sh
// (modules/quickshell/qtdeclarative-animations-wake-only-their-windows.patch):
// one still, one whose square a NumberAnimation moves on the GUI thread,
// and one whose square an XAnimator slides on the render thread, as the
// shell's reading slide does (ReadsOn.qml). Once each has drawn and all
// have gone quiet, both squares start moving ten times. It fails, exiting
// 2, if the still window drew any frame then, if either moving window drew
// no frame for a start, or fewer than two for more than one start, or if
// the still window draws none when its colour then changes, which shows
// it's still shown and still draws for a change of its own once the starts
// no longer ask it.
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
    // The starts a moving window drew no frame for, and those it drew
    // fewer than two for, as "3 (moving 0, sliding 4)".
    property var emptyStarts: []
    property var thinStarts: []
    readonly property int startsAsked: 10
    // When the latest start began, so a start that never draws still ends,
    // and when its animations last stopped, on the GUI thread's clock.
    property real startedAt: 0
    property real stoppedAt: 0
    // How long a start may wait for its frames: on a clock of 600 ms a
    // start lost frames that came late on a loaded builder and drew none
    // (22 November), so a start ends on what the windows did, or, past
    // this, quiet without both moving windows having drawn twice.
    readonly property int startWaitsFor: 3000
    // How long a start may take to go quiet at all. Every start ends
    // quiet, so a capped start's late frames are counted as its own, and
    // land in the next one only when they come more than quietFor after
    // the one before; a window still drawing at this point draws without
    // anything moving, which fails the case.
    readonly property int quietWaitsFor: 6000
    // How long the windows stay quiet, after both animations stopped and
    // after the last frame, before a start ends: past the 350 ms the old
    // clock left after a 250 ms animation, which late frames still
    // outran, so one queued behind a stall isn't counted in the next.
    readonly property int quietFor: 500
    // Each start's first frame drawn, and one of its ticks beside it in
    // every start but one. A start ends early only once both moving windows
    // drew twice, so a thin one is a start that waited its 3 s: a 250 ms
    // animation draws about fifteen at 60 Hz, but a loaded builder once
    // drew one in a start (21 November, on the 600 ms clock), and one stall
    // that long is allowed, a window that draws once in every start isn't.
    // A start that drew nothing at all in its 3 s still fails: no stall
    // the builders have shown held back every frame of a start.
    // Its first frame alone may be the NumberAnimation's `from` written on
    // the GUI thread, so only the second shows the start's ticks reached it.
    readonly property int thinAllowed: 1

    function fail(why: string): void {
        console.error(why);
        Qt.callLater(() => Qt.exit(2));
    }

    function drew(): void {
        root.lastDrawn = Date.now();
    }

    // How many frames each moving window drew for the start just ended,
    // said as it ends, so a run cut off by the script's timeout still says
    // how far it got, and whether it waited its longest, which leaves the
    // next start's timer line and frames to read beside it.
    function startEnded(how: string): void {
        const movedFrames = root.movingDrawn - root.movingBefore;
        const slidFrames = root.slidingDrawn - root.slidingBefore;
        const said = `${root.starts} (moving ${movedFrames}, sliding ${slidFrames})`;
        console.log(`Start ${said} drew, ending ${how}.`);
        if (movedFrames < 1 || slidFrames < 1)
            root.emptyStarts.push(said);
        if (movedFrames < 2 || slidFrames < 2)
            root.thinStarts.push(said);
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

                onRunningChanged: {
                    if (!running)
                        root.stoppedAt = Date.now();
                }
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

                onRunningChanged: {
                    if (!running)
                        root.stoppedAt = Date.now();
                }
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

    // How the latest start ended, or "" while it goes on: both animations
    // have stopped and no window has drawn, nor an animation stopped, for
    // quietFor, once both moving windows have drawn twice since it began
    // or it has waited startWaitsFor. The reason is worked out here alone,
    // so what the log says is what decided it.
    function startOver(): string {
        const waited = Date.now() - root.startedAt;
        const quietSince = Math.max(root.lastDrawn, root.stoppedAt);
        const quiet = !moving.running && !sliding.running
            && Date.now() - quietSince >= root.quietFor;
        if (!quiet)
            return "";
        const drawnTwice = root.movingDrawn - root.movingBefore >= 2
            && root.slidingDrawn - root.slidingBefore >= 2;
        if (drawnTwice)
            return "quiet";
        return waited >= root.startWaitsFor ? `quiet after its ${root.startWaitsFor} ms` : "";
    }

    // Each animation ends, and its windows go quiet, before the next
    // starts, so every start is one the animation driver hears as
    // animations starting again. Each says so before it starts them, so
    // the script can tell which of Qt's timer starts came in which of the
    // case's starts; the script matches that line's wording and the
    // count's, "Over N starts".
    Timer {
        id: started

        interval: 100
        repeat: true
        onTriggered: {
            if (root.starts > 0) {
                const how = root.startOver();
                if (how === "") {
                    if (Date.now() - root.startedAt < root.quietWaitsFor)
                        return;
                    running = false;
                    root.fail(`Start ${root.starts} never went quiet in ${root.quietWaitsFor} ms: `
                        + `the moving window drew ${root.movingDrawn - root.movingBefore} frames `
                        + `and the sliding one ${root.slidingDrawn - root.slidingBefore} in it, `
                        + `the still one ${root.stillDrawn - root.stillBefore} `
                        + "since the counting began, "
                        + `and an animation was ${moving.running || sliding.running ? "" : "not "}`
                        + "still running.");
                    return;
                }
                root.startEnded(how);
            }
            if (root.starts === root.startsAsked) {
                running = false;
                root.counted();
                return;
            }
            root.starts += 1;
            console.log(`The case starts its squares moving, start ${root.starts}.`);
            root.movingBefore = root.movingDrawn;
            root.slidingBefore = root.slidingDrawn;
            root.startedAt = Date.now();
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
        if (root.emptyStarts.length > 0) {
            root.fail(`A moving window drew no frame for starts ${root.emptyStarts.join(", ")}.`);
            return;
        }
        if (root.thinStarts.length > root.thinAllowed) {
            root.fail(`A moving window drew fewer than two frames for starts `
                + `${root.thinStarts.join(", ")}, more than ${root.thinAllowed}.`);
            return;
        }
        if (still > 0) {
            root.fail("The still window drew frames that showed nothing new.");
            return;
        }
        root.stillBefore = root.stillDrawn;
        stillWindow.color = "white";
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
