// Dictation listening (Dictation.qml): true while it hears, and in the half
// second it takes to type what it heard, so what's said never goes unseen.
// The chord that started it stops it, and the piece's action does the same.
import qs
import QtQuick

Piece {
    // The chord bound to dictation, beside the action that does what it does.
    readonly property var toggling: Shortcuts.actions
        .find(action => action.run.join(" ") === "dictation toggle") ?? { chord: [] }

    active: Dictation.busy
    hue: "voice"
    value: Dictation.listening ? "listening" : "typing"
    title: "Dictation"
    hint: Dictation.listening ? "Listening; its chord again types it" : "Typing what was said"
    actions: Dictation.listening ? [{
        glyph: String.fromCodePoint(0xf04db),
        name: "Stop and type it",
        chord: toggling.chord,
        run: () => Dictation.toggle()
    }] : []
}
