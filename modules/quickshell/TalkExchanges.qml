// The talks Joel has had with whoever hears him, newest first, as TalkLine.js's exchange()
// and spoken() say each: Super+S's line draws them under its field, and so does the launcher
// under the field of any act asking his words for an agent (Joel at 19:30 on 2026-11-13:
// "they need to feel more like the void"), so a talk reads one way wherever it's said.
//
// Each on the one text column and every line on the pitch, so nothing wraps back past where
// it starts; a talk an empty line from the next (docs/design.md: an empty line between groups
// of lines). In each, his words muted, when he said them or who answered at its right; then
// every answer in the text's ink; and while someone has it, "waiting on" them in the voice,
// breathing (WCJQSG [cf8639ab], Joel at 07:46 on 2026-11-13). The newest has room for eight
// lines of answer and the others three, so the two before it stay in view (the designer's
// choice).
pragma ComponentBehavior: Bound
import qs
import QtQuick

Column {
    id: root

    // What each talk shows, newest first.
    required property var exchanges
    // Where the text's column starts, from this column's left.
    property real textAt: 0
    // Whether a talk still waiting breathes: only while it's on screen.
    property bool breathing: true

    spacing: Theme.level.figure.pitch

    Repeater {
        model: root.exchanges

        Item {
            id: told

            required property var modelData
            required property int index
            readonly property real pitch: Theme.level.words.pitch
            readonly property int saidLines: line.lineCount
            readonly property int answerLines: answers.visible ? answers.lineCount : 0

            width: root.width
            height: (saidLines + answerLines + (stands.visible ? 1 : 0)) * pitch
            // The one text column every answer holds, up to where a time stands, whoever
            // answered, so no answer is pushed in by a long name (Joel at 17:49 on
            // 2026-11-13, [0adc518f]).
            readonly property real column: width - clock.width - Theme.space.rung.groups
                - root.textAt

            TextMetrics {
                id: clock
                font: side.font
                text: "00:00"
            }

            Text {
                id: line
                x: root.textAt
                y: told.pitch - baselineOffset
                width: side.x - Theme.space.rung.groups - x
                text: told.modelData.line
                textFormat: Text.PlainText
                color: Theme.muted
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
                lineHeight: told.pitch
                lineHeightMode: Text.FixedHeight
                font.family: Theme.level.words.family
                font.pointSize: Theme.level.words.pointSize
                font.variableAxes: Theme.level.words.variableAxes
            }

            Text {
                id: side
                x: told.width - width
                anchors.baseline: line.baseline
                text: told.modelData.side
                textFormat: Text.PlainText
                color: Theme.muted
                font.family: Theme.level.note.family
                font.pointSize: Theme.level.note.pointSize
                font.features: { "tnum": 1 }
            }

            Text {
                id: answers
                visible: told.modelData.answers.length > 0
                x: root.textAt
                y: (told.saidLines + 1) * told.pitch - baselineOffset
                width: told.column
                text: told.modelData.answers.join("\n")
                textFormat: Text.PlainText
                color: Theme.text
                wrapMode: Text.Wrap
                maximumLineCount: told.index === 0 ? 8 : 3
                elide: Text.ElideRight
                lineHeight: told.pitch
                lineHeightMode: Text.FixedHeight
                font.family: Theme.level.words.family
                font.pointSize: Theme.level.words.pointSize
                font.variableAxes: Theme.level.words.variableAxes
                // Its first line alone gives way to who answered, as his words give way to
                // when he said them.
                onLineLaidOut: laid => {
                    if (laid.number === 0 && by.visible)
                        laid.width = Math.min(laid.width,
                            by.x - Theme.space.rung.groups - answers.x);
                }
            }

            // Who answered, at the right of its first line in the time's own face, its right
            // edge on the time's, as when he said it stands at his words' right, so each side
            // means one thing; a long name gives way at a third of the line.
            Text {
                id: by
                visible: answers.visible && text !== ""
                x: told.width - width
                width: Math.min(implicitWidth, told.width / 3)
                anchors.baseline: answers.baseline
                text: told.modelData.by
                textFormat: Text.PlainText
                color: Theme.muted
                elide: Text.ElideRight
                font: side.font
                onXChanged: answers.forceLayout()
            }

            Text {
                id: stands
                readonly property bool waits: told.modelData.waiting !== ""
                visible: text !== ""
                x: root.textAt
                y: (told.saidLines + told.answerLines + 1) * told.pitch - baselineOffset
                width: told.width - x
                text: waits ? told.modelData.waiting : told.modelData.stands
                textFormat: Text.PlainText
                color: Theme.muted
                elide: Text.ElideRight
                font.family: Theme.level.voice.family
                font.italic: Theme.level.voice.italic
                font.pointSize: Theme.level.voice.pointSize
                font.variableAxes: Theme.level.voice.variableAxes

                // In and out over twice the caret's fade, on a soft curve rather than its
                // linear one, so it reads as breath rather than a blink, deeply, and for as
                // long as it waits, so a line still being worked on reads as alive: Joel's
                // word over motion that ends (8GYNJ6 [cdffbffc], 17:46 on 2026-11-13).
                // Answered or put away it rests whole.
                SequentialAnimation on opacity {
                    running: root.breathing && stands.waits
                    loops: Animation.Infinite
                    onStopped: stands.opacity = 1
                    NumberAnimation {
                        to: 0.2
                        duration: 2 * Options.caret.fade
                        easing.type: Easing.InOutSine
                    }
                    NumberAnimation {
                        to: 1
                        duration: 2 * Options.caret.fade
                        easing.type: Easing.InOutSine
                    }
                }
            }
        }
    }
}
