pragma Singleton
// What programs are playing, for each program's own controls in the
// launcher (Applications.qml): every media player that speaks MPRIS, and
// every stream a program has open to play sound through, as Sound.qml
// finds them. Kept apart from the program's rows so a check can stand in
// for both (tests/program-controls.qml). Mpris and PipeWire say when
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

    // A playback stream carries PipeWire's sink flag.
    readonly property var streams: Pipewire.nodes.values
        .filter(node => node.isStream && node.isSink && (node.type & PwNodeType.Audio))

    // A stream's volume and properties are heard only once something binds
    // it, and its properties say which program it is.
    PwObjectTracker {
        objects: playing.streams
    }
}
