// A meeting coming, which wants Joel (Calendar.qml's wanting): true from
// an hour before its start, or its first reminder when that's sooner, to
// five after, while its call isn't yet open in a window of its own, when
// it's on air instead (Call.qml). Now lists it, counting down ("in 42
// min"); the margin wears its mark only from ten minutes before, so an
// hour's notice doesn't crowd it. It's named for the meeting, and its
// first action joins its call, or opens it in Calendar when it has none.
// It's said as a notice of Calendar's (Arrivals.qml) at each of its
// reminders, as Google Calendar keeps them for it, and again as it
// starts, each taking the last one back, unless another notice told him
// the same already; notices held while Joel asked for quiet hold these
// too. Its notice goes, read, once it no longer wants him: its call open,
// or five minutes after its start.
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
    collected: !Calendar.marked
    arrives: Calendar.telling !== "" && !Calendar.toldElsewhere(meeting, Notices.history)
    telling: Calendar.telling
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
