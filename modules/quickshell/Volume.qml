// Shows the speaker's volume for a moment whenever it changes or mutes, at
// the bottom of the screen, as a bar in the theme.
import Quickshell
import Quickshell.Services.Pipewire
import Quickshell.Wayland
import QtQuick

Scope {
    id: root

    readonly property PwNode sink: Pipewire.defaultAudioSink
    property bool shown: false

    // Pipewire reports a node's volume only once something binds it.
    PwObjectTracker {
        objects: [root.sink]
    }

    Connections {
        target: root.sink ? root.sink.audio : null

        function onVolumesChanged() {
            root.shown = true;
            hide.restart();
        }

        function onMutedChanged() {
            root.shown = true;
            hide.restart();
        }
    }

    Timer {
        id: hide
        interval: 1500
        onTriggered: root.shown = false
    }

    PanelWindow {
        visible: root.shown
        anchors.bottom: true
        margins.bottom: Theme.gap * 6
        implicitWidth: 320
        implicitHeight: Theme.gap * 4
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "volume"

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
                width: (parent.width - Theme.gap * 2) * Math.min(root.sink ? root.sink.audio.volume : 0, 1)
                color: root.sink && root.sink.audio.muted ? Theme.muted : Theme.accent
                radius: Theme.radius
            }
        }
    }
}
