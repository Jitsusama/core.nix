// Keeping the last minute of the screen and its sound (Recorder.qml): true
// while it's on, so something that records the screen is never unseen. It's
// a mode Joel set and might forget, so it's muted as Keep Awake is, and
// never on air, since nothing leaves the machine or reaches a file until he
// saves it. Paused for a call, it still shows, saying so (VKCF3Z,
// 2026-10-29).
import qs
import QtQuick

Piece {
    active: Recorder.keeping
    value: Recorder.keepingHeld ? "paused" : Recorder.saving ? "saving" : "yes"
    title: Words.name("capture:keep")
    hint: Recorder.keepingHeld ? "Keeping the last minute, paused during the call"
        : "Keeping the last minute of the screen and its sound"
    context: Recorder.keepingHeld ? "paused during the call" : "the screen and its sound"
    actions: (Recorder.keepingHeld ? [] : [{
        glyph: Words.glyph("capture:save-minute"),
        name: Words.name("capture:save-minute"),
        verb: Words.verb("capture:save-minute"),
        run: () => Recorder.saveLastMinute()
    }]).concat([{
        glyph: Theme.glyph.stopped,
        name: "Stop Keeping It",
        verb: "stop",
        stays: true,
        run: () => Recorder.keepLastMinute(false)
    }])
}
