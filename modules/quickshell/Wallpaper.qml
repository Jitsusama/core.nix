// The picture behind every workspace, the theme's desktop wallpaper. It sits
// on niri's background layer and niri places it in the overview's backdrop
// (config.kdl), while the workspaces are drawn clear, so the picture stands
// still as they move over it rather than sliding with each one. It's shown
// only once it's read, so at boot niri keeps the splash until the picture
// itself is drawn (niri's hold-first-frame.patch) rather than going dark
// first; read again later, it fades back in rather than appearing half
// drawn. It's never dimmed:
// text set on it is lifted by shadows instead (Lifted.qml). A theme without
// one draws nothing here, and niri keeps its workspaces in the deepest
// colour.
import Quickshell
import Quickshell.Wayland
import QtQuick

Scope {
    Variants {
        model: Theme.wallpaper.desktop === "" ? []
            : Quickshell.screens.filter(screen => screen.name !== "")

        PanelWindow {
            id: window

            Counted { name: `wallpaper ${window.modelData?.name ?? ""}` }

            required property ShellScreen modelData

            screen: modelData
            // Mapped once the picture is read, and kept mapped after.
            visible: picture.read
            anchors {
                top: true
                bottom: true
                left: true
                right: true
            }
            // niri takes only a background surface that ignores the space
            // others reserve into the backdrop.
            exclusionMode: ExclusionMode.Ignore
            color: Theme.deepest
            WlrLayershell.layer: WlrLayer.Background
            WlrLayershell.namespace: "wallpaper"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.None

            Image {
                id: picture

                // Whether it's been read once, or failed to be, when the
                // window shows the deepest colour instead.
                property bool read: false

                anchors.fill: parent
                source: "file://" + Theme.wallpaper.desktop
                fillMode: Image.PreserveAspectCrop
                // Read at the screen's own pixels, not the file's, from the
                // screen rather than the window, which has no size until
                // it's shown. A screen that goes away is read once more
                // before its window does.
                sourceSize: Qt.size((window.modelData?.width ?? 0) * (window.modelData?.devicePixelRatio ?? 1),
                    (window.modelData?.height ?? 0) * (window.modelData?.devicePixelRatio ?? 1))
                asynchronous: true
                cache: false
                // Whole the first time it's shown, since it's read by then.
                opacity: status === Image.Ready || !read ? 1 : 0
                onStatusChanged: {
                    if (status === Image.Ready || status === Image.Error)
                        read = true;
                }

                Behavior on opacity {
                    NumberAnimation {
                        duration: Theme.motion.long
                        easing.type: Easing.BezierSpline
                        easing.bezierCurve: Theme.motion.curve
                    }
                }
            }

        }
    }
}
