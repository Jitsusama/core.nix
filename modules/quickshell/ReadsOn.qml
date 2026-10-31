// Words cut to their room that read on while their row is selected
// (docs/design.md, A long name reads on): after a rest to read how they
// start, they slide at a steady reading pace until their end shows, rest
// there, then glide back to their start and rest again. Only the words
// move, inside their own room, so nothing beside them does, and the row
// isn't any wider for it. A row's name and what it says at its right both
// read on through this one piece, so the two can't come to move apart
// (Joel, 2026-10-25: titles "get elipsed really quick"; 2026-10-28: "we
// need to not have different versions of these sort of elements"; VKCF3Z's
// ruling, 2026-10-30: a long value slides exactly as a long name does).
pragma ComponentBehavior: Bound
import QtQuick
import "Reading.js" as Reading

Item {
    id: room

    property string text
    property int format: Text.PlainText
    property color color: Theme.text
    property int weight: Font.Normal
    // Whether the words may read on now: their row is selected, and it's
    // their turn when the row has more than one cut.
    property bool reading: false
    // How wide a figure's character is, so the pace is in characters.
    property real figure: 0
    // Said each time the words are back at their start, so a row with two
    // cut can hand the turn to the other.
    signal through

    implicitWidth: words.implicitWidth
    implicitHeight: words.implicitHeight
    height: words.height
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

    Text {
        id: words
        width: room.scrolls ? implicitWidth : room.width
        text: room.text
        textFormat: room.format
        elide: room.scrolls ? Text.ElideNone : Text.ElideRight
        color: room.color
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
        font.weight: room.weight
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
        words.x = 0;
    }

    Timer {
        id: resting
        onTriggered: (room.outward ? slideOut : slideBack).start()
    }
    XAnimator {
        id: slideOut
        target: words
        from: 0
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
        to: 0
        duration: Theme.motion.medium
        easing.type: Easing.BezierSpline
        easing.bezierCurve: Theme.motion.curve
        onFinished: {
            if (!room.scrolls)
                return;
            room.through();
            // Still its turn, as when it's the row's only cut words.
            if (room.scrolls)
                room.read();
        }
    }
}
