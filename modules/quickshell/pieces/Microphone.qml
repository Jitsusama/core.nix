// The microphone: true while the default source is muted, so nothing reaches
// a call. Pipewire says when it changes, so nothing is asked.
import qs
import Quickshell.Services.Pipewire
import QtQuick

Piece {
    id: piece

    readonly property PwNode node: Pipewire.defaultAudioSource
    // A node's audio arrives a moment after the node, once it's bound.
    readonly property PwNodeAudio audio: node ? node.audio : null

    present: node !== null
    active: audio !== null && audio.muted
    value: "muted"
    title: "Microphone"
    hint: "Microphone muted"
    context: node ? node.description || node.name : ""
    actions: [{
        glyph: Words.glyph("verb:on:muted:unmute"),
        name: Words.name("verb:on:muted:unmute"),
        verb: Words.verb("verb:on:muted:unmute"),
        stays: true,
        run: () => {
            if (piece.audio)
                piece.audio.muted = false;
        }
    }]

    // Pipewire reports a node's mute only once something binds it.
    PwObjectTracker {
        objects: [piece.node]
    }
}
