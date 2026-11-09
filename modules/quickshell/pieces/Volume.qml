// The speakers' volume, or muted. It's true while they're muted or there are
// none, which is when sound would surprise by its absence.
import Quickshell.Services.Pipewire
import QtQuick

Piece {
    id: piece

    readonly property PwNode sink: Pipewire.defaultAudioSink

    // The speakers' own drawer, as Sound draws their row (launcher/Now.qml's
    // borrowed): none while Sound's rows draw none.
    drawnBy: sink ? { place: "sound", key: `sound:${sink.name}` } : null

    active: !sink || sink.audio.muted
    value: !sink ? "none" : sink.audio.muted ? "muted" : Math.round(sink.audio.volume * 100) + "%"
    sign: active ? "silent" : settings.sign ?? ""
    // Silence is the glyph alone.
    glance: active ? "" : value

    // Pipewire reports a node's volume only once something binds it.
    PwObjectTracker {
        objects: [piece.sink]
    }
}
