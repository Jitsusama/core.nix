// What goes with Super+S's line once it's sent, under its typing: a row each, muted, as it's
// context rather than anything to act on (TalkLine.qml). Each has its mark on the lead's
// column, in its kind's hue where it has one, its words on the text column, and at the right
// its note, which gives way first, so a row is one line (brief, slice C). Marks: a window's,
// the web's, an agent's, text's lines and the clipboard's; a browser's tab and a pi window's
// session wear the program's hue as the window does, and his own words, the selection and
// the clipboard, the muted text's, as the launcher's copies wear text's (Copies.qml), the
// designer's choice. TalkLine.js's shownRows says which rows there are.
pragma ComponentBehavior: Bound
import qs
import QtQuick

Column {
    id: attached

    // The rows, as TalkLine.js's shownRows gives them: {kind, mark, hue, text, note}.
    property var rows: []
    // Where the line's text column starts, from the frame's inset, so a row's words stand
    // under what's typed.
    property real textAt: 0

    Repeater {
        model: attached.rows

        Item {
            id: attachment

            required property var modelData
            readonly property real pitch: Theme.level.words.pitch
            // The note's room once the words have theirs, and whether there's enough of it
            // for the note to be read.
            readonly property real noteRoom: width - Theme.space.rung.groups
                - (attached.textAt + whatText.implicitWidth)
            readonly property bool noted: attachment.modelData.note !== ""
                && noteRoom >= Math.min(noteText.implicitWidth, 6 * Theme.space.beat)

            width: attached.width
            height: pitch

            Mark {
                x: 0
                line: whatText.y + whatText.baselineOffset
                text: Theme.glyph[attachment.modelData.mark] ?? ""
                color: attachment.modelData.hue === "program" ? Theme.program : Theme.muted
            }

            Text {
                id: whatText
                x: attached.textAt
                y: attachment.pitch - baselineOffset
                width: (attachment.noted ? noteText.x - Theme.space.rung.groups
                    : attachment.width) - x
                text: attachment.modelData.text
                textFormat: Text.PlainText
                color: Theme.muted
                elide: Text.ElideRight
                font.family: Theme.level.words.family
                font.pointSize: Theme.level.words.pointSize
                font.variableAxes: Theme.level.words.variableAxes
            }

            Text {
                id: noteText
                visible: attachment.noted
                width: Math.min(implicitWidth, attachment.noteRoom)
                x: attachment.width - width
                anchors.baseline: whatText.baseline
                text: attachment.modelData.note
                textFormat: Text.PlainText
                color: Theme.muted
                elide: Text.ElideMiddle
                font.family: Theme.level.note.family
                font.pointSize: Theme.level.note.pointSize
            }
        }
    }
}
