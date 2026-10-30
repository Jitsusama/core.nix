// The picture behind every workspace, the theme's desktop wallpaper. It sits
// on niri's background layer and niri places it in the overview's backdrop
// (config.kdl), while the workspaces are drawn clear, so the picture stands
// still as they move over it rather than sliding with each one. It fades in
// once it's read, rather than appearing half drawn, and is never dimmed:
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
                anchors.fill: parent
                source: "file://" + Theme.wallpaper.desktop
                fillMode: Image.PreserveAspectCrop
                // Read at the screen's own pixels, not the file's. A screen
                // that goes away is read once more before its window does.
                sourceSize: Qt.size(window.width * (window.modelData?.devicePixelRatio ?? 1),
                    window.height * (window.modelData?.devicePixelRatio ?? 1))
                asynchronous: true
                cache: false
                opacity: status === Image.Ready ? 1 : 0

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
