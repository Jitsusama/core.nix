// A surface floating above the windows: the launcher, a prompt, a notice or
// a meter (docs/design.md, Shape and Kinds). It's the deepest colour, cloudy
// enough to hint at what's beneath, which niri blurs; lifted by a light
// shadow; rounded as a whole thing; and framed by a hairline whose colour
// says where keys go: the accent when it takes them, the edge when it
// doesn't, the alert when it needs Joel. Its window gives it `area`, for
// niri's blur and for where the pointer reaches.
import Quickshell
import QtQuick
import QtQuick.Effects

Item {
    id: floating

    // Whether the surface holds the keyboard, as the launcher and prompts do.
    property bool keys: false
    // Whether it needs Joel: a failed password, a critical notice.
    property bool urgent: false
    default property alias content: body.data

    // How far its shadows reach beyond it, which its window leaves room for.
    readonly property int reach: Theme.floating.reach
    readonly property Region area: Region {
        item: panel
        radius: Theme.radius
    }

    implicitHeight: body.implicitHeight + 2 * Theme.border

    Repeater {
        model: Theme.floating.shadows

        RectangularShadow {
            required property var modelData

            anchors.fill: panel
            offset.y: modelData.offset
            blur: modelData.blur
            radius: Theme.radius
            color: Qt.alpha(Theme.shadow, modelData.opacity)
        }
    }

    Rectangle {
        id: panel
        anchors.fill: parent
        color: Qt.alpha(Theme.deepest, Theme.floating.opacity)
        radius: Theme.radius
        border.width: Theme.border
        border.color: floating.urgent ? Theme.alert : floating.keys ? Theme.accent : Theme.edge

        Item {
            id: body
            x: Theme.border
            y: Theme.border
            width: parent.width - 2 * Theme.border
            implicitHeight: childrenRect.height
        }
    }
}
