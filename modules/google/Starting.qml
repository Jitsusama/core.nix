// A meeting about to start, which wants Joel (Calendar.qml's wanting):
// true from two minutes before its start to five after, while its call
// isn't yet open in a window of its own, when it's on air instead
// (Call.qml). It's named for the meeting and says when it starts, and its
// first action joins its call, or opens it in Calendar when it has none.
// It's said once as a notice of Calendar's (Arrivals.qml), a minute before
// it starts, which is when it `arrives`, unless another notice told him of
// it already; notices held while Joel asked for quiet hold it too. Its
// notice goes, read, once it no longer wants him: its call open, or five
// minutes after its start.
import qs
import QtQuick

Piece {
    id: piece

    readonly property var meeting: Calendar.wanting
    readonly property string named: !meeting ? ""
        : meeting.title || Words.name("calendar:meeting", "untitled")

    active: meeting !== null
    kind: "wants"
    wants: true
    arrives: Calendar.heralded && !Calendar.toldElsewhere(meeting, Notices.history)
    withdraws: true
    app: "Calendar"
    hue: "person"
    title: named
    value: meeting ? Calendar.when(meeting) : ""
    hint: meeting ? `${named} ${Calendar.when(meeting)}` : ""
    context: !meeting ? "" : meeting.place ? `${Calendar.span(meeting)}, ${meeting.place}`
        : Calendar.span(meeting)
    actions: !meeting ? [] : meeting.link ? [{
        glyph: Words.glyph("verb:calendar:join"),
        name: Words.name("verb:calendar:join"),
        verb: Words.verb("verb:calendar:join"),
        run: () => Calendar.join(piece.meeting)
    }, {
        glyph: Words.glyph("verb:calendar:open"),
        name: Words.name("verb:calendar:open"),
        verb: Words.verb("verb:calendar:open"),
        run: () => Calendar.open(piece.meeting)
    }] : [{
        glyph: Words.glyph("verb:calendar:open"),
        name: Words.name("verb:calendar:open"),
        verb: Words.verb("verb:calendar:open"),
        run: () => Calendar.open(piece.meeting)
    }]
}
