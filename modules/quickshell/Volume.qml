// Shows the speaker's volume, or with `input` the microphone's, for a moment
// whenever it changes or mutes, at the bottom of the screen, as a bar in the
// theme. Pipewire says when, so nothing is asked every so often.
import qs
import Quickshell
import Quickshell.Services.Pipewire
import QtQuick

Scope {
    id: root

    property bool input: false
    readonly property PwNode node: input ? Pipewire.defaultAudioSource
        : Pipewire.defaultAudioSink
    // A node's audio arrives a moment after the node, once it's bound.
    readonly property PwNodeAudio audio: node ? node.audio : null
    property bool shown: false
    // The node whose changes it hears, once it's bound: until then a change
    // shows nothing.
    readonly property string heard: node && node.ready && audio ? node.name : ""

    onHeardChanged: {
        if (heard !== "")
            Report.say(`hears ${input ? "microphone" : "volume"} ${heard}`);
    }

    // Pipewire reports a node's volume only once something binds it.
    PwObjectTracker {
        objects: [root.node]
    }

    Connections {
        target: root.audio

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
        name: root.input ? "microphone" : "volume"
        glyph: root.input ? (off ? Theme.glyph.muted : Theme.glyph.microphone)
            : (off ? Theme.glyph.silent : Theme.glyph.volume)
        visible: root.shown
        level: root.audio ? root.audio.volume : 0
        off: root.audio ? root.audio.muted : false
    }
}
