// Text set straight on a picture, lifted off it by the theme's text shadows
// (jitsusama.theme.wallpaper.shadows) rather than by darkening the picture
// behind it, so the picture shows whole and the text still reads. Whatever
// goes inside is drawn once with the nearest shadow, and that again with
// the next, so each shadow falls under the text and the ones before it.
// Something drawn at another size takes shadows of its own, as the margin's
// marks take jitsusama.theme.wallpaper.marks.
//
// The shadows reach past the text, by `reach`, and a window holding lifted
// text leaves that much room around it.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Effects

Item {
    id: root

    default property alias content: body.data

    // Nearest first, as { x, y, blur, opacity }; two are drawn at most.
    property var shadows: Theme.wallpaper.shadows

    readonly property real reach: Math.max(0, ...root.shadows
        .map(s => Math.max(Math.abs(s.x), Math.abs(s.y)) + s.blur))

    implicitWidth: body.childrenRect.width
    implicitHeight: body.childrenRect.height

    component Shade: MultiEffect {
        required property var shadow

        autoPaddingEnabled: true
        shadowEnabled: true
        shadowColor: Theme.shadow
        shadowOpacity: shadow.opacity
        shadowHorizontalOffset: shadow.x
        shadowVerticalOffset: shadow.y
        // MultiEffect's blur is a fraction of the most it may spread.
        blurMax: Math.max(1, shadow.blur)
        shadowBlur: shadow.blur === 0 ? 0 : 1
    }

    // Larger than the text by the reach, so the layer it's drawn into keeps
    // the nearer shadow whole.
    Item {
        id: far

        x: -root.reach
        y: -root.reach
        width: root.width + 2 * root.reach
        height: root.height + 2 * root.reach
        layer.enabled: root.shadows.length > 1
        layer.effect: Shade {
            shadow: root.shadows[1]
        }

        Item {
            id: body

            x: root.reach
            y: root.reach
            width: root.width
            height: root.height
            layer.enabled: root.shadows.length > 0
            layer.effect: Shade {
                shadow: root.shadows[0]
            }
        }
    }
}
