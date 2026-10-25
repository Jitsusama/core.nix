// A Tuple call (Tuple.qml): true while Joel's in one, saying who with and
// for how long, and while one rings, when it wants him. Its line below
// says what matters most in the call now: that it's connecting, whose
// screen is shown, whose camera is on, and otherwise who's in it. Its
// actions bring a ringing call's window forward, since that's where it's
// answered; in a call they leave, share the screen while nothing's shared
// (the sharing piece stops a share), mute Joel in Tuple or let him be heard
// again, and copy his room's link. Muting is Tuple's own, so the others
// see it, and how the call stands says so. The clock ticks only during
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
    // First names, for its one short line.
    function first(who) {
        return Tuple.name(who).split(" ")[0];
    }
    // Whose screen is shown in the call, by the first sharing it.
    readonly property var showing: Tuple.showing.map(who => first(who) || "Someone")
    readonly property var watching: Tuple.watching.map(who => first(who) || "Someone")

    active: Tuple.calling || rings
    wants: rings
    kind: rings ? "wants" : "onAir"
    now: rings
    // Who's calling, joining or leaving is said as a notice of Tuple's.
    app: "Tuple"
    hue: "person"
    value: rings ? `${caller} is calling`
        : length + (Tuple.muted === true ? ", muted" : "")
    title: rings ? "Ringing" : "In a call"
    hint: rings ? `${caller} is calling on Tuple`
        : `In a Tuple call ${Tuple.company} for ${length}`
            + (Tuple.muted === true ? ", muted" : "")
    context: rings ? "Answer in Tuple's window"
        : Tuple.connecting ? "Connecting"
        : showing.length > 0 ? `${showing[0]} is sharing`
        : Tuple.sending ? "Sharing your screen"
        : watching.length > 0 ? `${Log.company(watching).replace(/^with /, "")}, on camera`
        : Log.company(Tuple.others.map(who => first(who)))
    actions: rings ? [{
        glyph: Theme.glyph.call,
        name: "Show the call",
        run: () => Tuple.show()
    }] : [{
        glyph: Theme.glyph.hangUp,
        name: "Leave the call",
        run: () => Tuple.leave()
    }].concat(sharing ? [] : [{
        glyph: Theme.glyph.sharing,
        name: "Share the screen",
        run: () => Tuple.share()
    }], [{
        glyph: Tuple.muted === true ? Theme.glyph.microphone : Theme.glyph.muted,
        name: Tuple.muted === true ? "Let them hear me again" : "Mute me in the call",
        run: () => Tuple.mute(Tuple.muted !== true)
    }], Tuple.room === "" ? [] : [{
        glyph: Theme.glyph.link,
        name: "Copy my room's link",
        run: () => Tuple.copyRoom()
    }])

    SystemClock {
        id: clock
        enabled: Tuple.calling
        precision: SystemClock.Minutes
    }
}
