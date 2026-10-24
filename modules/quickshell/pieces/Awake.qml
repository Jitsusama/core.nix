// Whether the machine is kept awake, and whether shutting its lid leaves it
// running too. It's true while it's kept awake, which is a mode, so it never
// holds unseen.
import qs
import QtQuick

Piece {
    active: Caffeinate.on
    value: !Caffeinate.on ? "no" : Caffeinate.lid ? "lid shut too" : "while open"
    title: "Keep Awake"
    hint: Caffeinate.lid ? "Staying awake, the lid shut too" : "Staying awake while the lid is open"
    context: Caffeinate.lid ? "until the power goes" : "the lid still sleeps it"
    actions: [{
        glyph: String.fromCodePoint(0xf04b2),
        name: "Let it sleep",
        // One word for Return, as the launcher's Let it sleep says it.
        verb: "stop",
        run: () => Caffeinate.release()
    }].concat(Caffeinate.lid ? [{
        glyph: String.fromCodePoint(0xf06ca),
        name: "Only while the lid is open",
        run: () => Caffeinate.open()
    }] : Caffeinate.plugged ? [{
        glyph: String.fromCodePoint(0xf0176),
        name: "With the lid shut too",
        run: () => Caffeinate.keep()
    }] : [])
}
