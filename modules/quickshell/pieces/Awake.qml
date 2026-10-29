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
        glyph: Theme.glyph.suspend,
        name: "Let It Sleep",
        // One word for Return, as the launcher's Let It Sleep says it.
        verb: "stop",
        stays: true,
        run: () => Caffeinate.release()
    }].concat(Caffeinate.lid ? [{
        glyph: Words.glyph("verb:on:awake:open"),
        name: Words.name("verb:on:awake:open"),
        verb: Words.verb("verb:on:awake:open"),
        stays: true,
        run: () => Caffeinate.open()
    }] : Caffeinate.plugged ? [{
        glyph: Words.glyph("verb:on:awake:shut"),
        name: Words.name("verb:on:awake:shut"),
        verb: Words.verb("verb:on:awake:shut"),
        stays: true,
        run: () => Caffeinate.keep()
    }] : [])
}
