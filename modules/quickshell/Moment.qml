// How long a meter (Meter.qml) stays once shown: a second and a half from
// the first frame niri shows of it, or from the last change once one has
// been, then it's spent, which hides it. Timed from the change, a meter on
// a screen slow to draw it went before it was seen, as the desktop test's
// did on builders drawing a frame in 1.3 to 2 s (17 and 18 November). One
// whose frame is never said shown is spent ten seconds after it last
// began, so a screen that never tells can't keep it up. Plain QtQuick, so
// modules/quickshell/tests/meter-moment.qml times it in Qt's own engine.
import QtQuick

Item {
    id: moment

    // Whether what it times is shown, and how long it lasts once seen and
    // at most, in milliseconds.
    property bool showing: false
    property int lasts: 1500
    property int atMost: 10000
    // Whether a frame of it has been shown since it last showed.
    property bool seen: false

    // Spent, and whether it was ever seen.
    signal spent(bool wasSeen)

    // Begins it again, as what it times changed.
    function again() {
        cap.restart();
        if (seen)
            last.restart();
    }

    // A frame of it was shown.
    function shown() {
        if (showing)
            seen = true;
    }

    onShowingChanged: {
        seen = false;
        last.stop();
        if (showing)
            cap.restart();
        else
            cap.stop();
    }
    onSeenChanged: {
        if (seen)
            last.restart();
    }

    Timer {
        id: last
        interval: moment.lasts
        onTriggered: {
            cap.stop();
            moment.spent(true);
        }
    }

    Timer {
        id: cap
        interval: moment.atMost
        onTriggered: {
            last.stop();
            moment.spent(moment.seen);
        }
    }
}
