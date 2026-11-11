// Words cut to their room that read on while their row is selected
// (docs/design.md, A long name reads on): after a rest to read how they
// start, they slide at a steady reading pace until their end shows, rest
// there, then glide back to their start, and again for as long as
// `reading` holds, which their row gives twice (Choice.qml). Only the words
// move, inside their own room, so nothing beside them does, and the row
// isn't any wider for it. A row's name and what it says at its right both
// read on through this one piece, so the two can't come to move apart
// (Joel, 2026-10-25: titles "get elipsed really quick"; 2026-10-28: "we
// need to not have different versions of these sort of elements"; VKCF3Z's
// ruling, 2026-10-30: a long value slides exactly as a long name does).
pragma ComponentBehavior: Bound
import QtQuick
import "Measures.js" as Measures
import "Reading.js" as Reading

Item {
    id: room

    property string text
    property int format: Text.PlainText
    property color color: Theme.text
    // The type's level they're set at (Theme.level): a row's name, what it
    // is at its right, or the shell's voice, as the launcher's words to
    // Joel are.
    property var level: Theme.level.name
    // Whether the words may read on now: their row is selected, and it's
    // their turn when the row has more than one cut.
    property bool reading: false
    // How wide a figure's character is, so the pace is in characters.
    property real figure: 0
    // The runs of letters a find matched in the words, washed beneath them
    // (Wash.qml), and the words as they read, without the markup styled
    // words carry, to measure them in.
    property var finds: []
    property string plain: text
    // Whether the words lie on the selection, whose wash is a shade past it.
    property bool washedSelected: false
    // Where it's washed in the words, each run's start, width and ground.
    readonly property var washed: wash.item ? (wash.item as Wash).laid : []
    // Said each time the words are back at their start, so a row with two
    // cut can hand the turn to the other.
    signal through

    implicitWidth: words.implicitWidth
    implicitHeight: words.implicitHeight
    height: words.height
    // So the voice can be set on another face's baseline (anchors.baseline).
    baselineOffset: words.baselineOffset
    clip: scrolls

    // How far past their room the words run, when they do.
    readonly property real over: Math.max(0, words.implicitWidth - width)
    readonly property bool scrolls: reading && over > 0
    // A reading pace of twelve characters a second, held between soft
    // ends (Reading.js). A short rest to see how they start, and a longer
    // one at their end to read that (Joel, 2026-10-29: the slide stuttered,
    // was too slow, and started and stopped abruptly).
    readonly property var slide: Reading.slide(over, 12 * figure)
    readonly property int restFirst: Math.round(500 * Theme.motion.slowdown)
    readonly property int restLast: Math.round(1000 * Theme.motion.slowdown)
    // Whether the words are off their start: from the moment they slide out
    // until they're back. At their start, as in the rest before they read
    // on, they end in an ellipsis as they do unselected, rather than cut
    // through a letter at their room's edge (VKCF3Z, 2026-11-03).
    property bool moving: false
    // Whether, cut at rest, the words give way at a whole word, as what a
    // row says at its right does, rather than through one (VKCF3Z,
    // 2026-11-05: "pick Aubergine, then Custom the\u2026"), and a row's name
    // too (docs/design.md, Set, not styled). Styled words are measured as
    // they read, without their markup (`plain`).
    property bool wholeWords: false
    // How wide the words are laid at rest: their room, or where they give
    // way at a word, only as far as their last whole word and its
    // ellipsis, so the ellipsis Qt draws falls there. A single word too
    // long for the room has nowhere else to give way, so it's cut through.
    // Qt elides what doesn't fit with a pixel to spare, so it's given one,
    // far less than the space or mark after the word.
    readonly property real restWidth: Math.min(width, restInk + (restInk < width ? 1 : 0))
    // How wide the words' ink is at rest, cut at a whole word: as wide as
    // their room where they're whole or cut through.
    readonly property real restInk: {
        if (!wholeWords || over <= 0)
            return width;
        void face.font;
        const wide = Measures.measurer(face);
        const cut = Reading.wholeWords(plain, width, wide);
        return cut === "" ? width : Math.min(width, Math.ceil(wide(cut)));
    }

    // Whether the words end at the room's right edge, as what a row says at
    // its right does: cut short at a whole word, they rest that much in from
    // the left, so their ellipsis falls at the edge every side ends at
    // (VKCF3Z, 2026-11-05), and they slide out and back from there.
    property bool endsRight: false
    // Rested on the ink, not the spare pixel, so the ellipsis ends at the
    // edge.
    readonly property real restAt: endsRight ? Math.max(0, width - restInk) : 0
    onRestAtChanged: if (!room.moving && !slideOut.running && !slideBack.running)
        words.x = room.restAt

    // The words' face, to find where they give way.
    FontMetrics {
        id: face
        font: words.font
    }

    Text {
        id: words
        x: room.restAt
        width: room.moving ? implicitWidth : room.restWidth
        text: room.text
        textFormat: room.format
        elide: room.moving ? Text.ElideNone : Text.ElideRight
        color: room.color
        font.family: room.level.family
        font.italic: room.level.italic
        font.pointSize: room.level.pointSize
        font.weight: room.level.weight
        font.variableAxes: room.level.variableAxes
        font.features: room.level.features

        // Made only while there's a find to wash, beneath the letters.
        Loader {
            id: wash
            anchors.fill: parent
            z: -1
            active: room.finds.length > 0
            sourceComponent: Wash {
                text: room.plain
                runs: room.finds
                font: words.font
                cut: words.truncated
                selected: room.washedSelected
            }
        }
    }

    // Animators, which move the words on the render thread, so the slide
    // keeps every frame while the shell is busy making rows again, as it
    // is each second in Machine (Joel, 2026-10-29: "scrolling long lines
    // feels a bit janky"). An animator says the words' x back only when
    // it stops, so each move runs on its own, the rests between them timed
    // here, rather than looping in one group that would never say where
    // the words rest.
    onScrollsChanged: room.scrolls ? room.read() : room.unread()
    Component.onCompleted: {
        if (room.scrolls)
            room.read();
    }
    // Which move the rest ends in: out to the words' end, or back.
    property bool outward: true
    function read() {
        room.outward = true;
        resting.interval = room.restFirst;
        resting.restart();
    }
    function unread() {
        resting.stop();
        slideOut.stop();
        slideBack.stop();
        words.x = room.restAt;
        room.moving = false;
    }

    Timer {
        id: resting
        onTriggered: {
            if (room.outward)
                room.moving = true;
            (room.outward ? slideOut : slideBack).start();
        }
    }
    XAnimator {
        id: slideOut
        target: words
        from: room.restAt
        to: -room.over
        duration: Math.round(1000 * Theme.motion.slowdown * room.slide.seconds)
        easing.type: Easing.BezierSpline
        easing.bezierCurve: room.slide.curve
        onFinished: {
            if (!room.scrolls)
                return;
            room.outward = false;
            resting.interval = room.restLast;
            resting.restart();
        }
    }
    XAnimator {
        id: slideBack
        target: words
        to: room.restAt
        duration: Theme.motion.medium
        easing.type: Easing.BezierSpline
        easing.bezierCurve: Theme.motion.curve
        onFinished: {
            if (!room.scrolls)
                return;
            room.moving = false;
            room.through();
            // Still its turn, as when it's the row's only cut words.
            if (room.scrolls)
                room.read();
        }
    }
}
