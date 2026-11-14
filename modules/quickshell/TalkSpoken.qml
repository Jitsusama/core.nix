// Super+S's lines under its typing, newest first, each on the one text column and every line
// on the pitch, so nothing wraps back past where it starts; a line an empty line from the next
// (docs/design.md: an empty line between groups of lines). In each, his words muted, when he
// said them or who answered at its right; then every answer in the text's ink; and while
// someone has it, "waiting on" them in the voice, breathing (WCJQSG [cf8639ab], Joel at 07:46
// on 2026-11-13). The newest has room for eight lines of answer and the others three, so the
// two before it stay in view (the designer's choice). They stand no taller than `room`, in
// whole lines, and scroll there a line at a time, by the wheel and by j and k in normal, so
// the card never runs off the screen (Joel at 23:39 on 2026-11-13, [ff6c6197]: "it also clips
// past the bottom of the screen ... i still can't really scroll through them"). In normal, j
// and k pick a line instead, on the selection's ground, and the list scrolls to keep it in
// view, so Return can open it as its thread (TalkThread.qml).
pragma ComponentBehavior: Bound
import qs
import QtQuick

Item {
    id: spoken

    // How each line stands, as TalkLine.js's exchange says it.
    required property var exchanges
    // Where the text's column starts, the field's own.
    required property real textAt
    // The room between one line and the next, an empty line.
    required property real gap
    // Whether the line is up, while which a line still waiting breathes; it comes up on the
    // newest.
    required property bool up
    // How tall it may stand.
    property real room: Infinity
    // The line j and k picked, which stands on the selection's ground, or -1 for none.
    property int picked: -1
    // How far left of it the frame's inside edge is, which the ground reaches out to.
    property real reach: 0

    readonly property real pitch: Theme.level.words.pitch
    // How tall every line stands, shown or not.
    readonly property real whole: lines.height
    // How far down it's scrolled, in whole lines.
    readonly property int scrolled: Math.round(view.contentY / pitch)
    // Its last baseline, where it ends, as each line's last baseline is.
    height: Math.min(whole, Math.max(pitch, Math.floor(room / pitch) * pitch))

    // `count` lines down, or up where it's less than nothing, as far as there is.
    function scroll(count) {
        const furthest = Math.max(0, whole - height);
        view.contentY = Math.max(0, Math.min(furthest, (scrolled + count) * pitch));
    }

    // Scrolled the fewest whole lines that bring the picked line in, all of it where there's
    // room, else its top.
    function reveal() {
        const told = shownLines.itemAt(picked);
        if (told === null)
            return;
        const top = Math.floor(told.y / pitch) * pitch;
        const below = Math.ceil((told.y + told.height - height) / pitch) * pitch;
        view.contentY = Math.max(0, Math.min(top, Math.max(view.contentY, below)));
    }

    onPickedChanged: reveal()
    onUpChanged: {
        if (up)
            view.contentY = 0;
    }

    // Clipped a descent under the last baseline, so the last line's tails show and no part
    // of the next line's capitals does; let go anywhere, it comes to rest on a whole line.
    // As wide as the frame's inside, so a ground can reach its edges, the lines the reach in.
    Flickable {
        id: view
        x: -spoken.reach
        width: spoken.width + 2 * spoken.reach
        height: spoken.height + face.descent
        contentWidth: width
        contentHeight: spoken.whole + face.descent
        clip: true
        interactive: contentHeight > height
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick
        onMovementEnded: spoken.scroll(0)

        FontMetrics {
            id: face
            font.family: Theme.level.words.family
            font.pointSize: Theme.level.words.pointSize
        }

        Column {
            id: lines
            x: spoken.reach
            width: spoken.width
            spacing: spoken.gap

            Repeater {
                id: shownLines
                model: spoken.exchanges

                Item {
                    id: told

                    required property var modelData
                    required property int index
                    readonly property real pitch: Theme.level.words.pitch
                    readonly property int saidLines: line.lineCount
                    readonly property int answerLines: answers.visible
                        ? answers.lineCount : 0

                    width: spoken.width
                    height: (saidLines + answerLines + (stands.visible ? 1 : 0)) * pitch
                    // The one text column every answer holds, up to where a time
                    // stands, whoever answered, so no answer is pushed in by a long
                    // name (Joel at 17:49 on 2026-11-13, [0adc518f]).
                    readonly property real column: width - clock.width
                        - Theme.space.rung.groups - spoken.textAt

                    // Its band, as much over its first line's ink as under its last, which
                    // keeps it inside its own lines' room: at rest the first stands whole
                    // under the view's top, and none meets the next line's ink.
                    Ground {
                        visible: told.index === spoken.picked
                        ink: "selection"
                        from: spoken.reach
                        width: told.width + 2 * spoken.reach
                        height: told.height + face.descent + told.pitch - face.ascent
                    }

                    TextMetrics {
                        id: clock
                        font: side.font
                        text: "00:00"
                    }

                    Text {
                        id: line
                        x: spoken.textAt
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
                        x: spoken.textAt
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
                        // Its first line alone gives way to who answered, as his
                        // words give way to when he said them.
                        onLineLaidOut: laid => {
                            if (laid.number === 0 && by.visible)
                                laid.width = Math.min(laid.width,
                                    by.x - Theme.space.rung.groups - answers.x);
                        }
                    }

                    // Who answered, at the right of its first line in the time's own
                    // face, its right edge on the time's, as when he said it stands at
                    // his words' right, so each side means one thing; a long name
                    // gives way at a third of the line.
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
                        x: spoken.textAt
                        y: (told.saidLines + told.answerLines + 1) * told.pitch
                            - baselineOffset
                        width: told.width - x
                        text: waits ? told.modelData.waiting : told.modelData.stands
                        textFormat: Text.PlainText
                        color: Theme.muted
                        elide: Text.ElideRight
                        font.family: Theme.level.voice.family
                        font.italic: Theme.level.voice.italic
                        font.pointSize: Theme.level.voice.pointSize
                        font.variableAxes: Theme.level.voice.variableAxes

                        // In and out over twice the caret's fade, on a soft curve
                        // rather than its linear one, so it reads as breath rather
                        // than a blink, deeply, and for as long as it waits, so a
                        // line still being worked on reads as alive: Joel's word
                        // over motion that ends (8GYNJ6 [cdffbffc], 17:46 on
                        // 2026-11-13). Answered or put away it rests whole.
                        SequentialAnimation on opacity {
                            running: spoken.up && stands.waits
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
    }
}
