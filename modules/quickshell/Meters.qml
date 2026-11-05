pragma Singleton

// What the meters at the bottom centre stand above: dictation's frame,
// while it shows (modules/dictation/Speaking.qml), and the space between
// frames that belong together. Overlays that share a place stack, never
// overlap (docs/design.md, The Ladder), and dictation stays put while it
// listens, so a meter, there only for a moment, is the one that moves.
// Nothing is under it while dictation isn't installed or shows nothing.
import Quickshell
import QtQuick

Singleton {
    // How far above its own place a meter stands, in logical pixels.
    property real under: 0
}
