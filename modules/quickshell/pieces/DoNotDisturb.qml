// Do not disturb (Quiet.qml): until when notices are held, and how many
// have been. It's a mode, so it's true the whole time it's on, whether Joel
// turned it on or the screen being shared did.
import qs
import QtQuick

Piece {
    readonly property string ends: Quiet.kept && Quiet.until > 0
        ? Qt.formatDateTime(new Date(Quiet.until), "HH:mm") : ""
    // The chord that opens the notices, beside the action that does.
    readonly property var opening: Shortcuts.actions
        .find(action => action.opens === "notices") ?? { chord: [] }
    readonly property int held: Quiet.held.length

    active: Quiet.on
    // On only because the screen is shared, the share's mark says it.
    collected: !Quiet.kept
    hue: "voice"
    value: Quiet.lasts
    title: "Do Not Disturb"
    hint: `Quiet ${Quiet.lasts}` + (held === 0 ? "" : `, ${held} held`)
    context: held === 0 ? "nothing held yet" : held === 1 ? "1 notice held" : `${held} notices held`
    actions: [{
        glyph: String.fromCodePoint(0xf009f),
        name: Quiet.kept ? "End it now" : "Let notices show",
        run: () => Quiet.end()
    }].concat(ends !== "" ? [{
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
