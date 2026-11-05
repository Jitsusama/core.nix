// One hairline (docs/design.md, Space, The Ladder): a logical pixel snapped
// to the device, in edge, across the frame from its inside edge to its
// inside edge. At a scale of 1.5 a logical pixel is a pixel and a half, so
// two rules a logical pixel thick fell one device pixel thick in one place
// and two in the next, by where they landed. Drawn whole device pixels
// thick, with every band between them whole device pixels tall (`whole`),
// each rule falls the same.
import QtQuick
import QtQuick.Window

Rectangle {
    id: rule

    // The window's own ratio, which a fractional scale sets, rather than
    // the screen's: under Wayland the screen says its output's whole scale,
    // 2 where the window draws at 1.5.
    readonly property real ratio: Window.window?.devicePixelRatio
        ?? (Screen.devicePixelRatio > 0 ? Screen.devicePixelRatio : 1)

    // A length rounded to whole device pixels, for a band beside a rule.
    function whole(length: real): real {
        return Math.round(length * ratio) / ratio;
    }

    implicitHeight: Math.max(1, Math.round(Theme.border * ratio)) / ratio
    color: Theme.edge
}
