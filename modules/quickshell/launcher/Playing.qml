pragma Singleton
// What programs are playing, for each program's own controls in the
// launcher (Applications.qml): every media player that speaks MPRIS, every
// stream a program has open to play sound through, and every one it has
// open to record from a microphone, as Sound.qml finds them. Kept apart
// from the program's rows so a check can stand in for them all
// (tests/program-controls.qml). Mpris and the sound server say when
// anything changes, so nothing here is asked or polled.
import Quickshell
import Quickshell.Services.Mpris
import Quickshell.Services.Pipewire
import QtQuick
import "../Music.js" as Players

Singleton {
    id: playing

    // playerctld speaks for whichever player it last saw, so it'd be one
    // player twice.
    readonly property var players: Players.own(Mpris.players.values)

    // A playback stream carries the sound server's sink flag.
    readonly property var streams: Pipewire.nodes.values
        .filter(node => node.isStream && node.isSink && (node.type & PwNodeType.Audio))

    // A recording stream doesn't. The peaks the shell reads to draw a
    // microphone's level, in the launcher and in dictation, are a recording
    // stream of the shell's own (PwNodePeakMonitor), and a level meter's
    // is a monitor: neither is a program recording, so they're left out.
    readonly property var recorders: Pipewire.nodes.values
        .filter(node => node.isStream && !node.isSink && (node.type & PwNodeType.Audio)
            && node.name !== "quickshell-peak-monitor")

    // A stream's volume and properties are heard only once something binds
    // it, and its properties say which program it is.
    PwObjectTracker {
        objects: playing.streams.concat(playing.recorders)
    }
}
