// The bar Volume.qml and Brightness.qml show for a moment at the bottom of the
// screen: a level from 0 to 1 in the accent colour, or in the muted colour
// while what it measures is off.
import Quickshell
import Quickshell.Wayland
import QtQuick

PanelWindow {
    id: meter

    // Names the surface to niri, which is how the desktop test finds it.
    required property string name
    property real level: 0
    property bool off: false

    anchors.bottom: true
    margins.bottom: Theme.gap * 6
    implicitWidth: 320
    implicitHeight: Theme.gap * 4
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: meter.name

    Rectangle {
        anchors.fill: parent
        color: Theme.background
        border.color: Theme.accent
        border.width: Theme.border
        radius: Theme.radius

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.margins: Theme.gap
            width: (parent.width - Theme.gap * 2) * Math.max(0, Math.min(meter.level, 1))
            color: meter.off ? Theme.muted : Theme.accent
            radius: Theme.radius
        }
    }
}
