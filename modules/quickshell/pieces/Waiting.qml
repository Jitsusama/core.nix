// Notices that came and went while Joel wasn't looking (Notices.qml): how
// many, and from whom. It's true while any wait to be seen, and then it
// wants him, unless notices are being held on purpose, when the margin says
// so instead (DoNotDisturb.qml). One still on screen isn't waiting yet.
import qs
import QtQuick

Piece {
    readonly property var missed: Notices.waiting
    readonly property var senders: [...new Set(missed.map(notice => notice.app))]
    // The chord that opens the notices, beside the action that does.
    readonly property var opening: Shortcuts.actions
        .find(action => action.opens === "notices") ?? { chord: [] }

    active: missed.length > 0 && !Quiet.on
    wants: active
    hue: "voice"
    value: `${missed.length} waiting`
    title: "Notices"
    hint: missed.length === 1 ? "A notice came in" : `${missed.length} notices came in`
    context: senders.length === 0 ? ""
        : senders.length === 1 ? `from ${senders[0]}`
        : `from ${senders[0]} and ${senders.length - 1} more`
    actions: [{
        glyph: String.fromCodePoint(0xf009c),
        name: "Show them",
        chord: opening.chord,
        run: shell => shell.launcher.into("notices")
    }, {
        glyph: String.fromCodePoint(0xf0a91),
        name: "Let them go",
        verb: "dismiss",
        run: () => Notices.seen()
    }]
}
