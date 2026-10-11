// Do not disturb (Quiet.qml): until when notices are held, and how many
// have been. It's a mode, so it's true the whole time it's on.
import qs
import QtQuick

Piece {
    readonly property string ends: Quiet.until > 0
        ? Qt.formatDateTime(new Date(Quiet.until), "HH:mm") : ""
    // The chord that opens the notices, beside the action that does.
    readonly property var opening: Shortcuts.actions
        .find(action => action.opens === "notices") ?? { chord: [] }
    readonly property int held: Quiet.held.length

    active: Quiet.on
    hue: "voice"
    value: ends === "" ? "until it's ended" : `until ${ends}`
    title: "Do Not Disturb"
    badge: ends
    hint: (ends === "" ? "Quiet until it's ended" : `Quiet until ${ends}`)
        + (held === 0 ? "" : `, ${held} held`)
    context: held === 0 ? "nothing held yet" : held === 1 ? "1 notice held" : `${held} notices held`
    actions: [{
        glyph: String.fromCodePoint(0xf009f),
        name: "End it now",
        run: () => Quiet.end()
    }].concat(Quiet.until > 0 ? [{
        glyph: String.fromCodePoint(0xf1862),
        name: "An hour more",
        run: () => Quiet.more(60)
    }] : [], held > 0 ? [{
        glyph: String.fromCodePoint(0xf009c),
        name: "Show what's held",
        chord: opening.chord,
        run: shell => shell.launcher.into("notices")
    }] : [])
}
