// Every desktop action bound to a key, found by what it does in words or the
// others its bind lists ("kitty" for a terminal, "fullscreen"), with its
// chord beside it so the chord is learned by use. Return does what the
// chord does; one that opens one of the launcher's own collections, such as
// the shortcuts, steps into it without closing and opening again.
import qs
import QtQuick

Provider {
    what: "shortcut"
    things: Shortcuts.actions.map(action => thing({
        key: `action:${action.does}`,
        // Return says the act's own verb, the first word of what it does
        // ("move", "focus"), as the foot's every other word is one.
        verb: action.opens === "" ? action.does.split(" ")[0].toLowerCase() : "open",
        name: action.does,
        keywords: action.also,
        glyph: action.glyph,
        hue: "action",
        chord: action.chord,
        opens: action.opens,
        run: () => Niri.spawn(action.run),
        // Acts on the screen, so it waits for the launcher to be gone.
        later: true
    }))
}
