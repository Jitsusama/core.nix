// The wallpaper's picture as screens come and go, in an offscreen
// Quickshell: ScreenPicture.qml reads its picture once its size is known,
// lets it go when its screen goes, and reads it again when the screen comes
// back, without a binding loop. Bound apart, its source read its
// sourceSize, which Qt says is the picture's own while none is set, so a
// screen going away looped twice a run (vm-screens-keep-the-launcher,
// 2026-11-12). The `screen-picture-follows-its-screen` check copies
// ScreenPicture.qml and Wallpaper.js beside this file, with a stand-in
// Niri.qml that never answers, and a 1 by 1 picture.png, and runs
// `qs -p shell.qml`, failing on any warning it gives.
import Quickshell
import QtQuick

ShellRoot {
    id: root

    // A screen as Quickshell gives one, while it's there.
    property bool there: true
    readonly property var screen: there
        ? ({ name: "DP-1", width: 4, height: 2, devicePixelRatio: 1 }) : null

    // Each step, waited for in turn, then what it holds.
    property int step: 0
    readonly property var steps: [
        {
            what: "a screen's picture is read at its pixels, capped at its file's",
            ready: () => picture.status === Image.Ready,
            holds: () => picture.sourceSize.width === 1 && picture.sourceSize.height === 1
                && root.reads === 1,
            then: () => {
                root.there = false;
            }
        },
        {
            what: "a screen gone lets its picture go",
            ready: () => picture.status === Image.Null,
            holds: () => String(picture.source) === "",
            then: () => {
                root.there = true;
            }
        },
        {
            what: "a screen come back reads its picture again",
            ready: () => picture.status === Image.Ready,
            holds: () => picture.sourceSize.width === 1 && root.reads === 2,
            then: () => {}
        }
    ]

    // How many times a read of the picture began: once for each time its
    // screen is there.
    property int reads: 0

    ScreenPicture {
        id: picture

        onStatusChanged: {
            if (status === Image.Loading)
                root.reads += 1;
        }

        screen: root.screen
        file: `${Quickshell.env("PWD")}/picture.png`
    }

    // Each step is looked at a frame at a time, since the picture is read
    // on a thread of its own and says so as it's done.
    Timer {
        interval: 16
        repeat: true
        running: true
        onTriggered: {
            const now = root.steps[root.step];
            if (!now.ready())
                return;
            if (!now.holds()) {
                console.error(`FAIL ${now.what}: ${picture.status}, ${picture.source},`
                    + ` ${picture.sourceSize.width} by ${picture.sourceSize.height},`
                    + ` ${root.reads} reads`);
                Qt.exit(1);
                return;
            }
            console.info(`ok ${now.what}`);
            now.then();
            root.step += 1;
            if (root.step === root.steps.length)
                Qt.exit(0);
        }
    }

    // A step that never comes says which.
    Timer {
        interval: 20000
        running: true
        onTriggered: {
            console.error(`FAIL never came: ${root.steps[root.step].what}`);
            Qt.exit(1);
        }
    }
}
