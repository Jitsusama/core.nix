// What Volume.qml and Brightness.qml show for a moment at the bottom of the
// screen while the level changes. It's the meter kind (docs/design.md, Shape
// and Kinds): the edge frame, since it never takes the keys; the machine's
// glyph; a bar, square, as it's part of no thing of its own, a stroke four
// hairlines thick, kin to the frame's border rather than a space; and the
// level in figures, which hold still as it changes. The bar goes muted
// while what it measures is off.
import Quickshell
import Quickshell.Wayland
import QtQuick

PanelWindow {
    id: meter

    Counted { name: "meter" }

    // Names the surface to niri, which is how the desktop test finds it.
    required property string name
    property string glyph
    property real level: 0
    property bool off: false

    // Wide enough for the bar to read as a scale at a glance.
    readonly property int length: 320
    readonly property real shown: Math.max(0, Math.min(level, 1))
    // What the figure says, a share unless a meter gives its own, such as
    // a warmth in kelvin, and the widest it gets.
    property string reading: ""
    property string widestReading: "100%"
    readonly property string figure: reading !== "" ? reading : `${Math.round(shown * 100)}%`

    // The figure it shows, said to the desktop test while reports are on.
    function tell() {
        if (Options.reports && meter.visible)
            Report.say(`level ${meter.name} ${meter.figure}`);
    }

    onFigureChanged: tell()
    onVisibleChanged: tell()

    anchors.bottom: true
    implicitWidth: length + 2 * Theme.floating.reach
    // A floating frame's distance from the screen's edge, below it, and
    // above dictation's frame while that shows, so the two stack rather
    // than overlap (docs/design.md, The Ladder).
    implicitHeight: card.height + Theme.space.rung.screen + Meters.under + Theme.floating.reach
    exclusionMode: ExclusionMode.Ignore
    color: "transparent"
    mask: card.area
    BackgroundEffect.blurRegion: card.area
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: meter.name

    Reported {
        name: meter.name
    }

    // The line's face, to measure its ink by: the glyph's, the figure's and
    // a capital's, since the room round the line is measured to the ink.
    // Each is a TextMetrics, whose bounds follow the face, where a call on
    // FontMetrics measures once, before the face has loaded, and a binding
    // never asks again.
    FontMetrics {
        id: ink
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
        font.features: { "tnum": 1 }
    }
    TextMetrics {
        id: capitalInk
        font: ink.font
        text: "H"
    }
    TextMetrics {
        id: markInk
        font: ink.font
        text: meter.glyph
    }
    TextMetrics {
        id: widestInk
        font: ink.font
        text: meter.widestReading
    }

    Floating {
        id: card
        x: Theme.floating.reach
        y: Theme.floating.reach
        width: meter.length

        // One line, the frame's inset from its ink on every side: above the
        // capitals, below the baseline, before the glyph and after the
        // widest figure. Dictation's frame is the same (Speaking.qml).
        Item {
            id: line
            readonly property rect capital: capitalInk.tightBoundingRect
            readonly property rect mark: markInk.tightBoundingRect
            readonly property rect widest: widestInk.tightBoundingRect
            // Where the line's baseline rests: its capitals centred in it,
            // so what rounding its height leaves is shared above and below.
            readonly property real rests: (height + capital.height) / 2

            width: parent.width
            // A whole number of pixels tall, so the frame's bottom is set.
            height: 2 * Theme.space.rung.inset + Math.round(capital.height)

            Text {
                id: mark
                x: Theme.space.rung.inset - line.mark.x
                y: line.rests - baselineOffset
                text: meter.glyph
                color: Theme.machine
                font: ink.font
            }

            // The bar, a mark from the glyph it's the level of and its
            // runs apart from the figure, centred on the capitals, on whole
            // pixels so its edges are drawn sharp.
            Rectangle {
                x: Math.round(mark.x + line.mark.x + line.mark.width + Theme.space.rung.mark)
                width: Math.round(value.x + line.widest.x - Theme.space.rung.apart) - x
                y: Math.round(line.rests - line.capital.height / 2 - height / 2)
                height: 4 * Theme.border
                color: Theme.edge

                Rectangle {
                    width: parent.width * meter.shown
                    height: parent.height
                    color: meter.off ? Theme.muted : Theme.text
                }
            }

            // Set from the left of the room the widest figure takes, so a
            // shorter one leaves its room after it, at the line's far end,
            // and the bar's gap to it never grows.
            Text {
                id: value
                x: line.width - Theme.space.rung.inset - line.widest.x - line.widest.width
                y: line.rests - baselineOffset
                text: meter.figure
                color: meter.off ? Theme.muted : Theme.text
                font: ink.font
            }
        }
    }
}
