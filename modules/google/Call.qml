// A Meet call (Calendar.qml's calls): true while one is open in a window
// of its own, as google-opens opens a call's link where the machine makes
// an app window of it. It's named for its meeting on Joel's calendar, or
// for its code when it isn't on it, and says when the meeting runs. Its
// actions bring the call's window forward, copy its link, and leave it,
// which closes the window, as Meet's own Leave does. A call in a browser
// tab is the browser's, and isn't seen.
import qs
import QtQuick

Piece {
    id: piece

    readonly property var call: Calendar.calls[0] ?? null
    readonly property var meeting: call?.meeting ?? null
    readonly property string named: !call ? "" : meeting ? meeting.title : call.code
    readonly property string link: call ? `https://meet.google.com/${call.code}` : ""
    readonly property int more: Math.max(0, Calendar.calls.length - 1)

    active: call !== null
    kind: "onAir"
    hue: "person"
    value: named
    title: "In a call"
    hint: `In ${named} on Meet` + (more > 0 ? `, and ${more} more` : "")
    context: meeting ? Calendar.span(meeting) : "on Meet"
    actions: !call ? [] : [{
        glyph: Theme.glyph.windows,
        name: "Show the call",
        run: () => Niri.act({ FocusWindow: { id: piece.call.window.id } })
    }, {
        glyph: Theme.glyph.link,
        name: "Copy its link",
        run: () => Niri.spawn(["wl-copy", piece.link])
    }, {
        glyph: Theme.glyph.hangUp,
        name: "Leave the call",
        run: () => Niri.act({ CloseWindow: { id: piece.call.window.id } })
    }]
}
