// Dictation listening (Dictation.qml): true while it hears, and in the half
// second it takes to type what it heard, so what's said never goes unseen.
// The chord that started it stops it, let go after a hold or pressed again
// after a tap, and the piece's action does the same.
import qs
import QtQuick

Piece {
    // The chord bound to dictation, beside the action that does what it does.
    // It runs `dictation hold`, or `dictation toggle` on a machine that binds
    // the older action.
    readonly property var toggling: Shortcuts.actions
        .find(action => ["dictation hold", "dictation toggle"].includes(action.run.join(" ")))
        ?? { chord: [] }

    active: Dictation.busy
    kind: "onAir"
    hue: "voice"
    value: Dictation.listening ? "listening" : "typing"
    title: "Dictation"
    hint: Dictation.listening
        ? "Listening; let go of its chord, or press it again, to type it"
        : "Typing what was said"
    actions: Dictation.listening ? [{
        glyph: String.fromCodePoint(0xf04db),
        name: "Stop and type it",
        chord: toggling.chord,
        run: () => Dictation.toggle()
    }] : []
}
