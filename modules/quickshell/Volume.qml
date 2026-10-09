// Shows the speaker's volume for a moment whenever it changes or mutes, at
// the bottom of the screen, as a bar in the theme.
import Quickshell
import Quickshell.Services.Pipewire
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

    Meter {
        name: "volume"
        visible: root.shown
        level: root.sink ? root.sink.audio.volume : 0
        off: root.sink ? root.sink.audio.muted : false
    }
}
