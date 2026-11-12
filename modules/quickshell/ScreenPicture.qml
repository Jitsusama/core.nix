// A picture filling a screen by crop, as the wallpaper shows its own, read
// at the screen's own pixels and no larger than the file itself
// (Wallpaper.js). The lock screen's picture is read as it was: at exactly
// its screen's pixels it held more than at the screen's whole-number
// ratio, 82 MB against 76, though nothing Qt was asked says why. The screen says only a whole number for its ratio,
// 2 for 1.5, so the scale is asked of niri, and the file's own size is read
// from its header. Until both are known nothing is read, and the size is
// set before the source, so the picture is read once rather than started
// at one size and again at the next.
import Quickshell
import Quickshell.Io
import QtQuick
import "Wallpaper.js" as Pixels

Image {
    id: picture

    // The screen it fills, and the picture's path, "" for none.
    required property var screen
    required property string file

    // niri's scale for the screen's output: undefined until niri answers,
    // null where it names none or doesn't answer in time.
    property var scale: undefined
    // The file's own size: undefined until its header is read, null where
    // it can't be, when the screen's pixels stand.
    property var own: undefined

    readonly property var pixels: scale === undefined || own === undefined
        ? ({ width: 0, height: 0 }) : Pixels.pixels(screen, scale, own)

    // Asked again by what holds it when the window's own ratio changes, as
    // it does when Joel scales the screen, so it's read again at its pixels.
    function askScale(): void {
        const name = picture.screen?.name ?? "";
        // A window that learns its screen only after it's made asks
        // again then.
        if (name === "")
            return;
        Niri.request("Outputs", answer => {
            picture.scale = Pixels.scaleOf(answer?.Outputs, name);
        }, () => {
            picture.scale = null;
        });
    }

    sourceSize: Qt.size(pixels.width, pixels.height)
    // From the size as it's set, not from what it's worked out from, so the
    // source is never set while the size still holds the one before.
    source: file !== "" && sourceSize.width > 0 ? "file://" + file : ""
    fillMode: Image.PreserveAspectCrop
    asynchronous: true
    // Read once for each window, so nothing is kept after.
    cache: false

    Component.onCompleted: askScale()
    onScreenChanged: askScale()

    Timer {
        // A niri that never answers leaves the screen's own ratio.
        running: picture.scale === undefined
        interval: 2000
        onTriggered: picture.scale = null
    }

    // Held only until the header is read, since it holds the whole file.
    Loader {
        active: picture.own === undefined && picture.file !== ""
        sourceComponent: FileView {
            path: picture.file
            printErrors: false
            onLoaded: picture.own = Pixels.ownSize(data())
            onLoadFailed: picture.own = null
        }
    }
}
