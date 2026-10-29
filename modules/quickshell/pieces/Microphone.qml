// The microphone: true while the default source is muted, so nothing reaches
// a call. Pipewire says when it changes, so nothing is asked.
//
// In Now it's the switch itself, as every Muted is (docs/design.md: "A
// switch is one row that keeps its name as it flips"), rather than an
// Unmute It act: named for whose, since Now has no crumbs to say it, and
// saying yes. Return flips it and the launcher stays, the row marked gone
// till Now is opened again, as any row in a list that changes is.
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
    value: "yes"
    // What a glance says, the switch's yes alone saying nothing there.
    glance: "muted"
    title: "Microphone Muted"
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
