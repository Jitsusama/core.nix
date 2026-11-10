// A Meet call (Calendar.qml's calls): true while one is open in a window
// of its own, as google-opens opens a call's link where the machine makes
// an app window of it. A call in a browser tab is the browser's, and isn't
// seen.
//
// In Now it's the call itself (VKCF3Z, 2026-11-08): named for its meeting
// on Joel's calendar, or for its code when it isn't on it; its side how it
// stands first, whether the microphone lets him be heard and how long it
// has gone on, "muted · 23 min"; after its name who's in it, giving way
// by whole names, and its note when its meeting runs. Return steps in to
// the Microphone, a switch that keeps its name as it flips; People, how
// many are in it on its side and their names after its own, each a step in
// since when they're in or when they left, as Meet says it to the google
// program (src/meet.rs) while the launcher is open; then going to the
// call's window, and Leave It, which closes the window as Meet's own Leave
// does and, as it ends the call, Return only writes `:leave` for (the key
// language's leave-call); and copying its link last. Its plate says the rest (Meet.js's
// details).
//
// The microphone is the machine's default source, as the Microphone
// piece's is: Meet's own mute, in its page, the shell can't see, so it's
// left out. Who's in the call waits on a sign-in granting Meet's scope to
// read it, and People says so until then.
import qs
import Quickshell
import Quickshell.Services.Pipewire
import QtQuick
// A piece is placed in pieces/, Meet.js beside the shell's root, as
// Calendar.qml reads it.
import "../Meet.js" as Said

Piece {
    id: piece

    readonly property var call: Calendar.calls[0] ?? null
    readonly property string code: call?.code ?? ""
    readonly property var meeting: call?.meeting ?? null
    readonly property string named: !call ? "" : meeting ? meeting.title : call.code
    readonly property string link: call ? `https://meet.google.com/${call.code}` : ""
    readonly property int more: Math.max(0, Calendar.calls.length - 1)
    // What Meet last said of it, or null before it has.
    readonly property var heard: Said.heard(Google.calls, code)
    // When its window opened, which is when Joel joined, for a call whose
    // conference Meet hasn't said the start of.
    property double opened: 0
    onCodeChanged: opened = code === "" ? 0 : Date.now()
    // A piece made while a call is already up hears no change of code.
    Component.onCompleted: opened = code === "" ? 0 : Date.now()
    readonly property double since: Said.began(heard, opened)

    readonly property PwNode node: Pipewire.defaultAudioSource
    readonly property PwNodeAudio audio: node ? node.audio : null
    readonly property bool muted: audio !== null && audio.muted

    readonly property var said: Words.of("meet:call").said ?? {}

    active: call !== null
    kind: "onAir"
    hue: "person"
    title: named
    value: call && since > 0 ? Said.side(muted, since, clock.date.getTime(), said) : ""
    parts: Said.inIt(heard)
    hint: `In ${named} on Meet` + (more > 0 ? `, and ${more} more` : "")
    context: meeting ? Calendar.span(meeting) : ""
    drawer: !call ? null : () => Said.details({
        named: piece.named,
        code: piece.code,
        heard: piece.heard,
        since: piece.since,
        meeting: piece.meeting
    }, Object.assign({}, piece.said, {
            people: Words.name("meet:call:people"),
            started: Words.name("meet:call:started"),
            meeting: Words.name("meet:call:meeting"),
            code: Words.name("meet:call:code")
        }))
    // The microphone and who's in the call, each a row of the launcher's,
    // before the acts.
    rows: !call ? null : (thing, listed) => [thing({
        key: "meet:call:microphone",
        name: Words.name("meet:call:microphone"),
        side: piece.muted ? piece.said.muted : piece.said.live,
        verb: piece.muted ? piece.said.unmute : piece.said.mute,
        glyph: piece.muted ? Theme.glyph.muted : Theme.glyph.microphone,
        hue: "action",
        stays: true,
        run: () => {
            if (piece.audio)
                piece.audio.muted = !piece.audio.muted;
        }
    }), thing({
        key: "meet:call:people",
        name: Words.name("meet:call:people"),
        side: Said.counted(piece.heard, piece.said),
        count: listed(Said.inIt(piece.heard), Words.name("meet:call:people"),
            Said.counted(piece.heard, piece.said)),
        glyph: Theme.glyph.people,
        hue: "person",
        verbs: () => Said.people(piece.heard).map(person => thing({
            key: `meet:call:person:${person.name}:${person.joined}`,
            name: person.name,
            side: Said.personSide(person, piece.said),
            glyph: Theme.glyph.person,
            hue: "person",
            plain: true,
            gone: Said.gone(person)
        }))
    })]
    actions: !call ? [] : [{
        glyph: Theme.glyph.windows,
        name: Words.name("meet:call:show"),
        verb: Words.verb("meet:call:show"),
        run: () => Niri.act({ FocusWindow: { id: piece.call.window.id } })
    }, {
        glyph: Theme.glyph.hangUp,
        name: Words.name("meet:call:leave"),
        verb: Words.verb("meet:call:leave"),
        run: () => Niri.act({ CloseWindow: { id: piece.call.window.id } })
    }, {
        // Copy is last, as it always is (docs/design.md). The clipboard's
        // history puts the link on the clipboard over the socket it holds
        // open (Pasting.qml), so nothing is started; without one kept,
        // wl-copy takes it, since a piece outside the launcher holds no
        // keys for Qt's clipboard.
        glyph: Theme.glyph.link,
        name: Words.name("meet:call:link"),
        verb: Words.verb("meet:call:link"),
        run: () => Pasting.copy(JSON.stringify(piece.link),
            () => Niri.spawn(["wl-copy", piece.link]))
    }]

    // Pipewire reports a node's mute only once something binds it.
    PwObjectTracker {
        objects: [piece.node]
    }

    SystemClock {
        id: clock
        enabled: piece.call !== null
        precision: SystemClock.Minutes
    }
}
