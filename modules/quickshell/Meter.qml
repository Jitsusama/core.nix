// What Volume.qml and Brightness.qml show for a moment at the bottom of the
// screen while the level changes. It's the meter kind (docs/design.md, Shape
// and Kinds): the edge frame, since it never takes the keys; the machine's
// glyph; a hairline bar, square, as it's part of no thing of its own; and
// the level in figures, which hold still as it changes. The bar goes muted
// while what it measures is off.
import Quickshell
import Quickshell.Wayland
import QtQuick

PanelWindow {
    id: meter

    // Names the surface to niri, which is how the desktop test finds it.
    required property string name
    property string glyph
    property real level: 0
    property bool off: false

    // Wide enough for the bar to read as a scale at a glance.
    readonly property int length: 320
    readonly property real shown: Math.max(0, Math.min(level, 1))

    anchors.bottom: true
    implicitWidth: length + 2 * Theme.floating.reach
    implicitHeight: card.height + Theme.space.xxxl + Theme.floating.reach
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    mask: card.area
    BackgroundEffect.blurRegion: card.area
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: meter.name

    Floating {
        id: card
        x: Theme.floating.reach
        y: Theme.floating.reach
        width: meter.length

        Item {
            width: parent.width
            height: Theme.space.xxxl - 2 * Theme.border

            Text {
                id: mark
                x: Theme.space.l - Theme.border
                anchors.verticalCenter: parent.verticalCenter
                width: Theme.space.xl
                text: meter.glyph
                color: Theme.machine
                font.family: Theme.mono.family
                font.pointSize: Theme.mono.body
            }

            Rectangle {
                anchors.left: mark.right
                anchors.right: value.left
                anchors.rightMargin: Theme.space.l
                anchors.verticalCenter: parent.verticalCenter
                height: Theme.space.xs
                color: Theme.edge

                Rectangle {
                    width: parent.width * meter.shown
                    height: parent.height
                    color: meter.off ? Theme.muted : Theme.text
                }
            }

            Text {
                id: value
                anchors.right: parent.right
                anchors.rightMargin: Theme.space.l - Theme.border
                anchors.verticalCenter: parent.verticalCenter
                text: `${Math.round(meter.shown * 100)}%`
                color: meter.off ? Theme.muted : Theme.text
                font.family: Theme.mono.family
                font.pointSize: Theme.mono.body
                font.features: { "tnum": 1 }
            }
        }
    }
}
