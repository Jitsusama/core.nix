// The time as the lock screen and the greeter set it on the picture: the
// clock at Display, at the size it stands alone on the wallpaper at, and
// the day under it at Words, in text, since nothing else on screen says it,
// both lifted off the picture by the theme's text shadows. The two are one
// group (docs/design.md, Space, The Ladder), so the day's capitals stand
// apart under the clock's baseline.
pragma ComponentBehavior: Bound
import QtQuick

Lifted {
    id: time

    property date date
    // The day's baseline, from the top, for what stands under it to measure
    // from.
    readonly property real dayBaseline: day.y + day.baselineOffset

    Item {
        implicitWidth: Math.max(clock.implicitWidth, day.implicitWidth)
        implicitHeight: day.y + day.implicitHeight

        FontMetrics {
            id: wordsFace
            font.family: Theme.level.words.family
            font.pointSize: Theme.level.words.pointSize
            font.variableAxes: Theme.level.words.variableAxes
        }

        Text {
            id: clock
            anchors.horizontalCenter: parent.horizontalCenter
            text: Qt.formatDateTime(time.date, "HH:mm")
            color: Theme.level.display.colour
            font.family: Theme.level.display.family
            font.pointSize: Theme.level.display.wallpaper.pointSize
            font.weight: Theme.level.display.weight
            font.letterSpacing: Theme.level.display.wallpaper.letterSpacing
            font.features: Theme.level.display.features
        }

        Text {
            id: day
            anchors.horizontalCenter: parent.horizontalCenter
            y: clock.baselineOffset + Theme.space.rung.apart
                + wordsFace.tightBoundingRect("H").height - baselineOffset
            text: Qt.formatDateTime(time.date, "dddd d MMMM")
            color: Theme.text
            font.family: Theme.level.words.family
            font.pointSize: Theme.level.words.pointSize
            font.weight: Theme.level.words.weight
            font.variableAxes: Theme.level.words.variableAxes
        }
    }
}
