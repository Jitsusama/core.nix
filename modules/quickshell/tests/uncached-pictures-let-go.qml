// A full-screen window showing the picture in PICTURE, read uncached from
// its file, as the wallpaper reads its own. Run as the shell's root on niri
// by tests/uncached-pictures-let-go-on-niri.sh
// (modules/quickshell/qtdeclarative-uncached-pictures-let-go.patch). Once
// it's drawn the file is written over with the picture in AFTER, and the
// picture moves to a second window, as the wallpaper's does when its window
// is made again for another screen. What the second window shows is written
// to the file in SECOND_WINDOW: the picture in AFTER where its first
// texture let the file's pixels go and they were read again, the one first
// read where they were kept, and nothing where they were let go for good.
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick

ShellRoot {
    id: root

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
        WlrLayershell.layer: WlrLayer.Background

        Image {
            id: picture

            anchors.fill: parent
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
            cache: false
            sourceSize: Qt.size(2880, 1800)
            source: "file://" + Quickshell.env("PICTURE")
            onStatusChanged: {
                if (status === Image.Error)
                    root.fail(`The picture wasn't read: ${source}`);
            }
        }
    }

    PanelWindow {
        id: second

        anchors {
            top: true
            left: true
        }
        implicitWidth: 480
        implicitHeight: 300
        color: "black"
    }

    // Long enough for the window's first frames and the picture's texture:
    // on a loaded builder Mesa's rasteriser takes over a second a frame at
    // 2880 by 1800.
    Timer {
        running: true
        interval: 10000
        onTriggered: {
            if (picture.status !== Image.Ready) {
                root.fail(`The picture wasn't ready in ${interval} ms`);
                return;
            }
            writtenOver.running = true;
        }
    }

    // Written over whole, through cp, so it is never read half written.
    Process {
        id: writtenOver

        command: ["cp", Quickshell.env("AFTER"), Quickshell.env("PICTURE")]
        onExited: code => {
            if (code !== 0) {
                root.fail(`The picture couldn't be written over: cp exited ${code}`);
                return;
            }
            picture.parent = second.contentItem;
            moved.start();
        }
    }

    // The picture in its second window, once it has drawn there.
    Timer {
        id: moved

        interval: 5000
        onTriggered: picture.grabToImage(result => {
            if (!result.saveToFile(Quickshell.env("SECOND_WINDOW"))) {
                root.fail("What the second window shows couldn't be written");
                return;
            }
            Qt.callLater(() => Qt.exit(0));
        })
    }
}
