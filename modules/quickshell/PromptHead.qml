// A prompt's head (docs/design.md, Shape and Kinds, the prompt kind): who
// asks and for what, at Words, then what it's for under it. It keeps the
// frame's inset to the ink (Space, The Ladder): its first capital's top
// under the frame's top and each line's first letter from the frame's
// left. With no rows under it to set its band, the hairline below falls
// where its lines read centred (Bands.js's holding; VKCF3Z, 2026-11-05),
// a little more than the inset under the last baseline. Its lines sit at the Words
// pitch, what it's for under what's asked as lines of one saying, so the
// head reads as one thing. Polkit's, the askpass's and the greeter's are
// one head.
pragma ComponentBehavior: Bound
import QtQuick
import "Bands.js" as Bands

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
    // A rich Text set at a fixed line height puts the room the pitch adds
    // to its natural line above each line, not below as a plain one does,
    // while its baselineOffset still says where the natural line's
    // baseline is; so its baselines fall this much lower than it says.
    readonly property real richDrop: pitch - natural.implicitHeight
    readonly property real askedBaseline: askedText.baselineOffset + richDrop
    // What's asked's first baseline, from the head's top: its capitals the
    // inset under it.
    readonly property real askedFirst: askedText.y + askedBaseline
    // Each Text's last baseline, from the head's top.
    readonly property real askedLast: askedText.y + askedBaseline
        + (lines(askedText) - 1) * pitch
    readonly property real reasonLast: reasonText.y + reasonText.baselineOffset
        + (lines(reasonText) - 1) * pitch

    // Its foot on a whole device pixel, counted from the frame's outer
    // edge, since it sits under the frame's border, so the rule under it
    // falls as every other does (Hairline.qml).
    implicitHeight: rule.whole(Bands.holding(askedFirst,
        reasonText.visible ? reasonLast : askedLast, wordsCaps.tightBoundingRect.height,
        wordsSmall.tightBoundingRect.height) + Theme.border) - Theme.border

    Hairline {
        id: rule
        visible: false
    }

    // How tall Words' capitals and small letters stand: TextMetrics, since
    // a FontMetrics' call in a binding is asked once, before the face has
    // loaded, and reads small ever after (HVD6O3, 2026-11-05).
    TextMetrics {
        id: wordsCaps
        text: "H"
        font.family: Theme.level.words.family
        font.pointSize: Theme.level.words.pointSize
        font.variableAxes: Theme.level.words.variableAxes
    }
    TextMetrics {
        id: wordsSmall
        text: "x"
        font: wordsCaps.font
    }
    // How far each line's first letter stands in from its box, so the ink
    // lands on the inset.
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
        y: head.inset + wordsCaps.tightBoundingRect.height - head.askedBaseline
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

    // One rich line at its natural height, to tell how far the pitch moves
    // a rich line's baseline.
    Text {
        id: natural
        visible: false
        textFormat: Text.RichText
        text: "H"
        font: askedText.font
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
