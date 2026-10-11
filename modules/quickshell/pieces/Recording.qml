// The screen being recorded straight from niri, as wf-recorder does: true
// while niri says such a screencast streams, and for how long. The clock
// ticks only while it's true, so it costs nothing otherwise. A recorder is
// stopped as Ctrl+C would, so it finishes the file.
import qs
import Quickshell
import QtQuick

Piece {
    id: piece

    readonly property var casts: Object.values(Niri.casts)
        .filter(cast => cast.kind === "WlrScreencopy" && cast.is_active)
    // When it was first seen, since niri doesn't say when it began.
    property real began: 0
    readonly property int seconds: active ? Math.max(0, (clock.date - began) / 1000) : 0
    readonly property string length:
        `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`

    active: casts.length > 0
    onActiveChanged: began = active ? Date.now() : 0
    value: length
    title: "Recording"
    badge: length
    hint: `Recording the screen, ${length}`
    context: casts.every(cast => cast.target.Output) ? "the whole screen" : "one window"
    actions: casts.some(cast => cast.pid !== null) ? [{
        glyph: String.fromCodePoint(0xf04db),
        name: "Stop recording",
        run: () => {
            for (const cast of casts)
                if (cast.pid !== null)
                    Niri.spawn(["kill", "-INT", String(cast.pid)]);
        }
    }] : []

    SystemClock {
        id: clock
        enabled: piece.active
        precision: SystemClock.Seconds
    }
}
