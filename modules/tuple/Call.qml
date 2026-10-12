// A Tuple call (Tuple.qml): true while Joel's in one, and for how long. Its
// panel leaves the call, shares the screen into it while nothing's shared
// (the sharing piece stops a share), mutes or unmutes the microphone, which
// Tuple hears through, and copies Joel's room's link. The clock ticks only
// during a call, so it costs nothing otherwise.
import qs
import Quickshell
import Quickshell.Services.Pipewire
import QtQuick
import "../Tuple.js" as Log

Piece {
    id: piece

    readonly property int minutes: active
        ? Math.max(0, Math.floor((clock.date - Tuple.since) / 60000)) : 0
    readonly property string length: Log.length(minutes)
    readonly property bool sharing: Object.values(Niri.casts)
        .some(cast => cast.kind === "PipeWire" && cast.is_active)

    readonly property PwNode microphone: Pipewire.defaultAudioSource
    readonly property PwNodeAudio audio: microphone ? microphone.audio : null

    active: Tuple.calling
    value: length
    title: "In a call"
    hint: `In a Tuple call for ${length}`
    context: "on Tuple"
    actions: [{
        glyph: String.fromCodePoint(0xf03f5),
        name: "Leave the call",
        run: () => Tuple.leave()
    }].concat(sharing ? [] : [{
        glyph: String.fromCodePoint(0xf1483),
        name: "Share the screen",
        run: () => Tuple.share()
    }], audio === null ? [] : [{
        glyph: String.fromCodePoint(audio.muted ? 0xf036c : 0xf036d),
        name: audio.muted ? "Unmute the microphone" : "Mute the microphone",
        run: () => {
            if (piece.audio)
                piece.audio.muted = !piece.audio.muted;
        }
    }], Tuple.room === "" ? [] : [{
        glyph: String.fromCodePoint(0xf0339),
        name: "Copy my room's link",
        run: () => Tuple.copyRoom()
    }])

    SystemClock {
        id: clock
        enabled: piece.active
        precision: SystemClock.Minutes
    }

    // Pipewire reports a node's mute only once something binds it.
    PwObjectTracker {
        objects: [piece.microphone]
    }
}
