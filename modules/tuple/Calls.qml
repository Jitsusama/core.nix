// Tuple's calls in the launcher (Tuple.qml), each row there only when it
// can be done: opening Joel's room and copying its link to hand on, or
// joining the call whose link was just copied, while he's in none; leaving
// the call, sharing the screen into it, and muting him in it or letting
// him be heard, while he's in one; bringing forward a ringing call's
// window, where it's answered; and copying his room's link once it's
// known. A ringing call and a copied link are also first at home, since
// opening the launcher then is when they're wanted.
import qs
import QtQuick

Provider {
    id: provider

    function glyph(code) {
        return String.fromCodePoint(code);
    }

    readonly property bool sharing: Object.values(Niri.casts)
        .some(cast => cast.kind === "PipeWire" && cast.is_active)
    // A link copied that isn't Joel's own room, which he opens instead.
    readonly property string joinable: Tuple.calling || Tuple.copied === Tuple.room
        ? "" : Tuple.copied

    readonly property var joining: joinable === "" ? null : thing({
        key: "tuple:join",
        verb: "join",
        name: "Join the Tuple call in the clipboard",
        keywords: "pair call link",
        glyph: glyph(0xf0659),
        hue: "action",
        note: joinable.replace(/^https:\/\//, ""),
        later: true,
        run: () => Tuple.join(provider.joinable)
    })

    readonly property var answering: Tuple.ringing === "" || Tuple.calling ? null : thing({
        key: "tuple:ringing",
        verb: "show",
        name: `Show the Tuple call from ${Tuple.name(Tuple.ringing) || "someone"}`,
        keywords: "pair ringing answer",
        glyph: glyph(0xf03f2),
        hue: "person",
        note: "answer it in Tuple's window",
        run: () => Tuple.show()
    })

    things: (answering ? [answering] : []).concat(Tuple.calling ? [thing({
        key: "tuple:leave",
        verb: "leave",
        name: "Leave the Tuple call",
        keywords: "pair hang up end",
        glyph: glyph(0xf03f5),
        hue: "action",
        later: true,
        run: () => Tuple.leave()
    })].concat(sharing ? [] : [thing({
        key: "tuple:share",
        verb: "share",
        name: "Share the screen in the Tuple call",
        keywords: "pair",
        glyph: glyph(0xf1483),
        hue: "action",
        later: true,
        run: () => Tuple.share()
    })], [thing({
        key: "tuple:mute",
        verb: Tuple.muted === true ? "unmute" : "mute",
        name: Tuple.muted === true ? "Let them hear me again in the Tuple call"
            : "Mute me in the Tuple call",
        keywords: "pair microphone voice",
        glyph: glyph(Tuple.muted === true ? 0xf036c : 0xf036d),
        hue: "action",
        later: true,
        run: () => Tuple.mute(Tuple.muted !== true)
    })]) : [thing({
        key: "tuple:open",
        verb: "open",
        name: "Open my Tuple room",
        keywords: "pair call start new",
        glyph: glyph(0xf081c),
        hue: "action",
        note: "and copy its link to hand on",
        later: true,
        run: () => Tuple.open()
    })].concat(joining ? [joining] : [])).concat(Tuple.room === "" ? [] : [thing({
        key: "tuple:link",
        verb: "copy",
        name: "Copy my Tuple room's link",
        keywords: "pair call url",
        glyph: glyph(0xf0339),
        hue: "action",
        note: Tuple.room.replace(/^https:\/\//, ""),
        run: () => Tuple.copyRoom()
    })])

    function home() {
        return (answering ? [answering] : []).concat(joining ? [joining] : []);
    }
}
