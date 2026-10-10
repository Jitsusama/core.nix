// The speakers' volume, or muted. It's true while they're muted or there are
// none, which is when sound would surprise by its absence.
import Quickshell.Services.Pipewire
import QtQuick

Piece {
    readonly property PwNode sink: Pipewire.defaultAudioSink

    active: !sink || sink.audio.muted
    value: !sink ? "none" : sink.audio.muted ? "muted" : Math.round(sink.audio.volume * 100) + "%"

    // Pipewire reports a node's volume only once something binds it.
    PwObjectTracker {
        objects: [sink]
    }
}
