// A Tuple call (Tuple.qml): true while Joel's in one, saying who with and
// for how long, and while one rings, when it wants him. Its panel brings a
// ringing call's window forward, since that's where it's answered; in a
// call it leaves, shares the screen while nothing's shared (the sharing
// piece stops a share), mutes Joel in Tuple or lets him be heard again,
// and copies his room's link. Muting is Tuple's own, so the others see
// it, and the margin says so beside the mark. The clock ticks only during
// a call, so it costs nothing otherwise.
import qs
import Quickshell
import QtQuick
import "../Tuple.js" as Log

Piece {
    id: piece

    readonly property int minutes: Tuple.calling
        ? Math.max(0, Math.floor((clock.date - Tuple.since) / 60000)) : 0
    readonly property string length: Log.length(minutes)
    readonly property bool sharing: Object.values(Niri.casts)
        .some(cast => cast.kind === "PipeWire" && cast.is_active)
    readonly property bool rings: Tuple.ringing !== "" && !Tuple.calling
    readonly property string caller: Tuple.name(Tuple.ringing) || "Someone"
    // First names, for the panel's one short line.
    function first(who) {
        return Tuple.name(who).split(" ")[0];
    }
    // Whose screen is shown in the call, by the first sharing it.
    readonly property var showing: Tuple.call.showing.map(who => first(who) || "Someone")

    active: Tuple.calling || rings
    wants: rings
    hue: "person"
    value: rings ? `${caller} is calling` : length
    title: rings ? "Ringing" : "In a call"
    badge: Tuple.calling && Tuple.muted === true ? "muted" : ""
    hint: rings ? `${caller} is calling on Tuple`
        : `In a Tuple call ${Tuple.company} for ${length}`
            + (Tuple.muted === true ? ", muted" : "")
    context: rings ? "Answer in Tuple's window"
        : showing.length > 0 ? `${showing[0]} is sharing`
        : Log.company(Tuple.others.map(who => first(who)))
    actions: rings ? [{
        glyph: String.fromCodePoint(0xf03f2),
        name: "Show the call",
        run: () => Tuple.show()
    }] : [{
        glyph: String.fromCodePoint(0xf03f5),
        name: "Leave the call",
        run: () => Tuple.leave()
    }].concat(sharing ? [] : [{
        glyph: String.fromCodePoint(0xf1483),
        name: "Share the screen",
        run: () => Tuple.share()
    }], [{
        glyph: String.fromCodePoint(Tuple.muted === true ? 0xf036c : 0xf036d),
        name: Tuple.muted === true ? "Let them hear me again" : "Mute me in the call",
        run: () => Tuple.mute(Tuple.muted !== true)
    }], Tuple.room === "" ? [] : [{
        glyph: String.fromCodePoint(0xf0339),
        name: "Copy my room's link",
        run: () => Tuple.copyRoom()
    }])

    SystemClock {
        id: clock
        enabled: Tuple.calling
        precision: SystemClock.Minutes
    }
}
