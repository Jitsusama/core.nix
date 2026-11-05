// A prompt's head (docs/design.md, Shape and Kinds, the prompt kind): who
// asks and for what, at Words, then what it's for under it. It keeps the
// frame's inset to the ink (Space, The Ladder): its first capital's top
// under the frame's top, each line's first letter from the frame's left,
// and its last baseline over the hairline below. Its lines sit at the Words
// pitch, what it's for under what's asked as lines of one saying, so the
// head reads as one thing. Polkit's, the askpass's and the greeter's are
// one head.
pragma ComponentBehavior: Bound
import QtQuick

Item {
    id: head

    // What's asked, as rich text in text's colour, and the letter it opens
    // on with the face that letter is set in, so its ink, not its box,
    // stands on the column.
    property string asked
    property string askedOpens
    property string askedOpensIn: Theme.level.words.family
    // Whether what's asked keeps to one line, cut at its end, as the
    // greeter's greeting does.
    property bool oneLine: false
    // What it's for, plain, at its level and colour; none when it's empty.
    property string reason
    property var reasonLevel: Theme.level.words
    property color reasonColour: reasonLevel.colour

    readonly property int inset: Theme.space.rung.inset
    readonly property int pitch: Theme.level.words.pitch

    // The lines a Text sets at the pitch. A rich Text's first line comes a
    // pixel under the pitch and its lineCount counts paragraphs, so its
    // lines are counted by its height.
    function lines(text: Text): int {
        return Math.max(1, Math.round(text.implicitHeight / head.pitch));
    }
    // Each Text's last baseline, from the head's top.
    readonly property real askedLast: askedText.y + askedText.baselineOffset
        + (lines(askedText) - 1) * pitch
    readonly property real reasonLast: reasonText.y + reasonText.baselineOffset
        + (lines(reasonText) - 1) * pitch

    // Its foot on a whole device pixel, counted from the frame's outer
    // edge, since it sits under the frame's border, so the rule under it
    // falls as every other does (Hairline.qml).
    implicitHeight: rule.whole((reasonText.visible ? reasonLast : askedLast) + inset
        + Theme.border) - Theme.border

    Hairline {
        id: rule
        visible: false
    }

    // How tall Words' capitals stand, and how far each line's first letter
    // stands in from its box, so the ink lands on the inset.
    FontMetrics {
        id: wordsFace
        font.family: Theme.level.words.family
        font.pointSize: Theme.level.words.pointSize
        font.variableAxes: Theme.level.words.variableAxes
    }
    FontMetrics {
        id: askedFace
        font.family: head.askedOpensIn
        font.pointSize: Theme.level.words.pointSize
    }
    FontMetrics {
        id: reasonFace
        font.family: head.reasonLevel.family
        font.pointSize: head.reasonLevel.pointSize
    }

    Text {
        id: askedText
        x: head.inset - askedFace.tightBoundingRect(head.askedOpens).x
        y: head.inset + wordsFace.tightBoundingRect("H").height - baselineOffset
        width: head.width - x - head.inset
        textFormat: Text.RichText
        text: head.asked
        color: Theme.text
        wrapMode: head.oneLine ? Text.NoWrap : Text.Wrap
        elide: head.oneLine ? Text.ElideRight : Text.ElideNone
        lineHeight: head.pitch
        lineHeightMode: Text.FixedHeight
        // The line takes the face's weight rather than its axis, since a
        // strong span can't set the axis.
        font.family: Theme.level.words.family
        font.pointSize: Theme.level.words.pointSize
        font.weight: Theme.level.words.spans.weight
        font.variableAxes: Theme.level.words.spans.variableAxes
    }

    Text {
        id: reasonText
        visible: head.reason !== ""
        x: head.inset - reasonFace.tightBoundingRect(head.reason.charAt(0)).x
        y: head.askedLast + head.pitch - baselineOffset
        width: head.width - x - head.inset
        text: head.reason
        color: head.reasonColour
        wrapMode: Text.Wrap
        lineHeight: head.pitch
        lineHeightMode: Text.FixedHeight
        font.family: head.reasonLevel.family
        font.pointSize: head.reasonLevel.pointSize
        font.weight: head.reasonLevel.weight
        font.variableAxes: head.reasonLevel.variableAxes
        font.features: head.reasonLevel.features
    }
}
