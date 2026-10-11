// Every desktop action bound to a key, found by what it does in words, with
// its chord beside it so the chord is learned by use. Return does what the
// chord does; one that opens one of the launcher's own collections, such as
// the shortcuts, steps into it without closing and opening again.
import qs
import QtQuick

Provider {
    // The collection a command opens, when it asks the launcher for one.
    function opens(run) {
        const asks = ["qs", "ipc", "call", "launcher", "collection"];
        return run.length === 6 && asks.every((word, i) => run[i] === word) ? run[5] : "";
    }

    things: Shortcuts.actions.map(action => thing({
        key: `action:${action.does}`,
        verb: opens(action.run) === "" ? "run" : "open",
        name: action.does,
        glyph: action.glyph,
        hue: "action",
        chord: action.chord,
        opens: opens(action.run),
        run: () => Niri.spawn(action.run),
        // Acts on the screen, so it waits for the launcher to be gone.
        later: true
    }))
}
