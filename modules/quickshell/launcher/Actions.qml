// Every desktop action bound to a key, found by what it does in words, with
// its chord beside it so the chord is learned by use. Return does what the
// chord does.
import qs
import QtQuick

Provider {
    things: Shortcuts.actions.map(action => thing({
        key: `action:${action.does}`,
        verb: "run",
        name: action.does,
        glyph: action.glyph,
        hue: "action",
        chord: action.chord,
        run: () => Niri.spawn(action.run),
        // Acts on the screen, so it waits for the launcher to be gone.
        later: true
    }))
}
