// The screen, a region, a window or the camera being recorded (Recorder.qml):
// true while niri says such a screencast streams or the camera's viewfinder
// says it records, and for how long. The clock
// ticks only while it's true, so it costs nothing otherwise. Recorder.qml
// starts and stops it.
import qs
import Quickshell
import QtQuick

Piece {
    id: piece

    // When it was first seen, since niri doesn't say when it began.
    property real began: 0
    readonly property int seconds: active ? Math.max(0, (clock.date - began) / 1000) : 0
    readonly property string length:
        `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`

    active: Recorder.recording
    kind: "onAir"
    onActiveChanged: began = active ? Date.now() : 0
    value: length
    title: "Recording"
    hint: `Recording ${Recorder.what}, ${length}`
    context: Recorder.what
    actions: Recorder.stoppable ? [{
        glyph: Theme.glyph.recording,
        name: "Stop Recording",
        stays: true,
        run: () => Recorder.stop()
    }] : []

    SystemClock {
        id: clock
        enabled: piece.active
        precision: SystemClock.Seconds
    }
}
