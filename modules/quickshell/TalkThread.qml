// A line of Super+S's opened as its thread, in the list's place: his words whole, muted, when
// he said them at their right; then everything the void wrote of it in the order written, an
// empty line apart, answers in the text's ink and whole, the rest (who has it, who let it go)
// in the voice, each with who wrote it and when at its right; and under each, while za has
// opened the fold, what the void calls it and the messages it is, in the note face (Joel at
// 17:48 on 2026-11-13, 8GYNJ6 [851e94b2]: "i need a way to drill into them ... they lack the
// nerdiness of other things"). It stands no taller than `room`, in whole lines, and scrolls a
// line at a time by the wheel and by j and k, as the list does (TalkSpoken.qml).
pragma ComponentBehavior: Bound
import qs
import QtQuick

Item {
    id: thread

    // The thread as TalkThread.js's threadOf makes it, or null while none is open.
    required property var shown
    // Where the text's column starts, the field's own.
    required property real textAt
    // The room between one entry and the next, an empty line.
    required property real gap
    // Whether the fold is open.
    property bool nerdy: false
    // How tall it may stand.
    property real room: Infinity

    readonly property real pitch: Theme.level.words.pitch
    readonly property real whole: lines.height
    readonly property int scrolled: Math.round(view.contentY / pitch)
    height: Math.min(whole, Math.max(pitch, Math.floor(room / pitch) * pitch))

    // His words first, said as a step of his own is.
    readonly property var entries: shown === null ? [] : [{ text: shown.line, answer: false,
        mine: true, by: "", side: shown.side, nerd: shown.nerd }].concat(shown.steps)

    // `count` lines down, or up where it's less than nothing, as far as there is.
    function scroll(count) {
        const furthest = Math.max(0, whole - height);
        view.contentY = Math.max(0, Math.min(furthest, (scrolled + count) * pitch));
    }

    onShownChanged: {
        if (shown === null)
            view.contentY = 0;
    }

    // Clipped a descent under the last baseline, as the list is.
    Flickable {
        id: view
        width: thread.width
        height: thread.height + face.descent
        contentWidth: width
        contentHeight: thread.whole + face.descent
        clip: true
        interactive: contentHeight > height
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick
        onMovementEnded: thread.scroll(0)

        FontMetrics {
            id: face
            font.family: Theme.level.words.family
            font.pointSize: Theme.level.words.pointSize
        }

        Column {
            id: lines
            width: view.width
            spacing: thread.gap

            Repeater {
                model: thread.entries

                Item {
                    id: entry

                    required property var modelData
                    readonly property real pitch: Theme.level.words.pitch
                    // His words and the answers in the words' face, the rest in the voice.
                    readonly property bool voiced: !modelData.answer && !modelData.mine
                    readonly property bool folded: nerd.visible

                    width: thread.width
                    height: (said.lineCount + (folded ? 1 : 0)) * pitch

                    Text {
                        id: said
                        x: thread.textAt
                        y: entry.pitch - baselineOffset
                        width: entry.width - x
                        text: entry.modelData.text
                        textFormat: Text.PlainText
                        color: entry.modelData.answer ? Theme.text : Theme.muted
                        wrapMode: Text.Wrap
                        lineHeight: entry.pitch
                        lineHeightMode: Text.FixedHeight
                        font.family: entry.voiced ? Theme.level.voice.family
                            : Theme.level.words.family
                        font.italic: entry.voiced && Theme.level.voice.italic
                        font.pointSize: entry.voiced ? Theme.level.voice.pointSize
                            : Theme.level.words.pointSize
                        font.variableAxes: entry.voiced ? Theme.level.voice.variableAxes
                            : Theme.level.words.variableAxes
                        // Its first line alone gives way to who and when, as the list's
                        // answers give way to who answered.
                        onLineLaidOut: laid => {
                            if (laid.number === 0)
                                laid.width = Math.min(laid.width,
                                    side.x - Theme.space.rung.groups - said.x);
                        }
                    }

                    // Who wrote it and when, at the right of its first line in the note face,
                    // its right edge on the frame's text edge; a long name gives way at a
                    // third of the line.
                    Text {
                        id: side
                        x: entry.width - width
                        width: Math.min(implicitWidth, entry.width / 3)
                        anchors.baseline: said.baseline
                        text: entry.modelData.by === "" ? entry.modelData.side
                            : `${entry.modelData.by} \u00b7 ${entry.modelData.side}`
                        textFormat: Text.PlainText
                        color: Theme.muted
                        elide: Text.ElideLeft
                        font.family: Theme.level.note.family
                        font.pointSize: Theme.level.note.pointSize
                        font.features: { "tnum": 1 }
                        onXChanged: said.forceLayout()
                    }

                    // The nerdy bits, a line under it while the fold is open.
                    Text {
                        id: nerd
                        visible: thread.nerdy && text !== ""
                        x: thread.textAt
                        y: (said.lineCount + 1) * entry.pitch - baselineOffset
                        width: entry.width - x
                        text: entry.modelData.nerd
                        textFormat: Text.PlainText
                        color: Theme.muted
                        elide: Text.ElideRight
                        font.family: Theme.level.note.family
                        font.pointSize: Theme.level.note.pointSize
                        font.features: { "tnum": 1 }
                    }
                }
            }
        }
    }
}
