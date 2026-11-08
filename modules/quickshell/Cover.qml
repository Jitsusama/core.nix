// A picture standing in a card's mark column, as music's cover and a
// notice's own image do (docs/design.md, The launcher and held notices are
// one family): two Name pitches square from the capitals of the line it
// starts on, its corners the theme's inner radius, and the raised colour
// until it arrives. Whoever places it gives its top; one piece draws them
// all, so the two can't drift apart.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Effects

Rectangle {
    id: cover

    property string source

    width: 2 * Theme.level.name.pitch
    height: width
    radius: Theme.inner
    color: Theme.raised
    visible: source !== ""

    Image {
        anchors.fill: parent
        source: cover.source
        asynchronous: true
        fillMode: Image.PreserveAspectCrop
        sourceSize.width: 2 * width
        sourceSize.height: 2 * height
        layer.enabled: true
        layer.effect: MultiEffect {
            maskEnabled: true
            maskSource: corners
        }
    }

    Rectangle {
        id: corners
        anchors.fill: parent
        radius: Theme.inner
        visible: false
        layer.enabled: true
    }
}
