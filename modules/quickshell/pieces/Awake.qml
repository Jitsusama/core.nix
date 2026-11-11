// Whether the machine is kept awake, and whether shutting its lid leaves it
// running too. It's true while it's kept awake, which is a mode, so it never
// holds unseen.
import qs
import QtQuick

Piece {
    readonly property var said: Words.of("session:awake").said ?? {}

    // Keep Awake's own drawer, as the session's row draws it
    // (launcher/Now.qml's borrowed): none while that row draws none.
    drawnBy: ({ place: "session", key: "session:awake" })
    active: KeepAwake.on
    value: !KeepAwake.on ? "no" : KeepAwake.lidShutToo ? "lid shut too" : "while open"
    title: "Keep Awake"
    hint: KeepAwake.lidShutToo ? "Staying awake, the lid shut too"
        : "Staying awake while the lid is open"
    context: KeepAwake.lidShutToo ? said.lidShut : said.lidSleeps
    // The lid's acts only where holding it changes something: where a shut
    // lid never suspends, neither would do anything.
    actions: [{
        glyph: Theme.glyph.suspend,
        name: "Let It Sleep",
        // One word for Return, as the launcher's Let It Sleep says it.
        verb: "turn off",
        stays: true,
        run: () => KeepAwake.release()
    }].concat(!KeepAwake.lidHolds ? [] : KeepAwake.lid ? [{
        glyph: Words.glyph("verb:on:awake:open"),
        name: Words.name("verb:on:awake:open"),
        verb: Words.verb("verb:on:awake:open"),
        stays: true,
        run: () => KeepAwake.open()
    }] : [{
        glyph: Words.glyph("verb:on:awake:shut"),
        name: Words.name("verb:on:awake:shut"),
        verb: Words.verb("verb:on:awake:shut"),
        stays: true,
        run: () => KeepAwake.keep()
    }])
}
