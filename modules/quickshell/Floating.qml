// A surface floating above the windows: the launcher, a prompt, a notice or
// a meter (docs/design.md, Shape and Kinds). It's the deepest colour, cloudy
// enough to hint at what's beneath, which niri blurs; lifted by a light
// shadow; rounded as a whole thing; and framed by a hairline whose colour
// says where keys go: the accent when it takes them, the edge when it
// doesn't, the alert when it needs Joel. Its window gives it `area`, for
// niri's blur and for where the pointer reaches.
pragma ComponentBehavior: Bound
import Quickshell
import QtQuick
import QtQuick.Effects

Item {
    id: floating

    // Whether the surface holds the keyboard, as the launcher and prompts do.
    property bool keys: false
    // Whether it needs Joel: a failed password, a critical notice.
    property bool urgent: false
    // Whether its foot is square, as where it stands on another surface
    // along a line: the launcher's drawer over its card on a narrow screen,
    // its sides on the card's and its foot across the card's rows, where a
    // rounded corner would leave a notch the card shows through.
    property bool squareFoot: false
    // Whether its top is square, as an edge in a stack of notices is: it
    // starts under the bottom hairline of the card in front, so its sides run
    // straight down from there, as a whole card's would, rather than curving
    // in where they show.
    property bool flatTop: false
    default property alias content: body.data

    // How far its shadows reach beyond it, which its window leaves room for.
    readonly property int reach: Theme.floating.reach
    // Its bottom corners' radius: the whole thing's, or square at a
    // square foot.
    readonly property int footRadius: squareFoot ? 0 : Theme.radius
    // Its top corners' radius: the whole thing's, or square at a flat top.
    readonly property int topRadius: flatTop ? 0 : Theme.radius
    readonly property Region area: Region {
        item: panel
        radius: Theme.radius
        topLeftRadius: floating.topRadius
        topRightRadius: floating.topRadius
        bottomLeftRadius: floating.footRadius
        bottomRightRadius: floating.footRadius
    }
    // The frame its window gathers: its own area, which follows it only as
    // it moves itself, or one its window places where it stands, which
    // follows what holds it too, as a stack of notices does.
    property Region framed: area

    implicitHeight: body.implicitHeight + 2 * Theme.border

    // Its shadows, as far as they reach. At a square foot they're cut
    // there, and each runs a reach past it, so neither a shadow's round
    // corner nor its fading edge shows under the cloudy panel, and none
    // falls on what it stands on.
    Item {
        x: -floating.reach
        y: -floating.reach
        width: floating.width + 2 * floating.reach
        height: floating.height + (floating.squareFoot ? 1 : 2) * floating.reach
        clip: floating.squareFoot

        Repeater {
            model: Theme.floating.shadows

            RectangularShadow {
                required property var modelData

                x: floating.reach
                y: floating.reach
                width: floating.width
                height: floating.height + (floating.squareFoot ? floating.reach : 0)
                offset.y: modelData.offset
                blur: modelData.blur
                radius: Theme.radius
                color: Qt.alpha(Theme.shadow, modelData.opacity)
            }
        }
    }

    Rectangle {
        id: panel
        anchors.fill: parent
        color: Qt.alpha(Theme.deepest, Theme.floating.opacity)
        radius: Theme.radius
        bottomLeftRadius: floating.footRadius
        bottomRightRadius: floating.footRadius
        topLeftRadius: floating.topRadius
        topRightRadius: floating.topRadius
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
