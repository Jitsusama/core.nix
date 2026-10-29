// The screen, a region, a window or the camera being recorded (Recorder.qml):
// true while niri says such a screencast streams or the camera's viewfinder
// says it records, and for how long. The clock
// ticks only while it's true, so it costs nothing otherwise. Recorder.qml
// starts and stops it.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

Piece {
    id: piece

    // When it began, as the kernel says its recorder's process started,
    // since niri doesn't say: a recording already running when the shell
    // starts, as after a switch, says how long it has really run (Joel,
    // 2026-10-28: "i never want it to assume anything"). When it first
    // showed, only where the process can't be read.
    readonly property real began: started.ticks > 0 && boot.at > 0
        ? (boot.at + started.ticks / ticksPerSecond) * 1000 : seen
    // Linux counts a process's start in ticks of a hundredth of a second
    // to whatever reads /proc, on every architecture.
    readonly property int ticksPerSecond: 100
    property real seen: 0
    // The process recording: the camera's, else the one behind a cast.
    readonly property var recorder: Recorder.camera?.pid ?? Recorder.casts
        .map(cast => Recorder.recorderOf(cast)).find(pid => pid !== null) ?? null
    readonly property int seconds: active ? Math.max(0, (clock.date - began) / 1000) : 0
    readonly property string length:
        `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`

    active: Recorder.recording
    kind: "onAir"
    onActiveChanged: seen = active ? Date.now() : 0
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

    // When the recorder's process started, in ticks since the machine
    // booted: the 22nd of its stat's fields, counted past its name, which
    // is in brackets and may hold spaces.
    FileView {
        id: started
        property real ticks: 0
        path: piece.recorder === null ? "" : `/proc/${piece.recorder}/stat`
        printErrors: false
        onPathChanged: ticks = 0
        onLoaded: {
            const fields = text().slice(text().lastIndexOf(")") + 2).split(" ");
            ticks = Number(fields[19]) || 0;
        }
        onLoadFailed: ticks = 0
    }

    // When the machine booted, in seconds since 1970.
    FileView {
        id: boot
        property real at: 0
        path: piece.recorder === null ? "" : "/proc/stat"
        printErrors: false
        onLoaded: at = Number(/^btime (\d+)$/m.exec(text())?.[1] ?? 0)
        onLoadFailed: at = 0
    }

    SystemClock {
        id: clock
        enabled: piece.active
        precision: SystemClock.Seconds
    }
}
