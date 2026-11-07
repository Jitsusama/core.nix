// A row's glyph in its card's mark column (Columns.js): every mark at one
// optical size beside the type, whatever its shape, and its ink centred on
// the capitals of the line it stands beside. The launcher's rows and a
// notice's draw their glyphs through it, so a glyph can't stand in one
// place on one card and another on the next (VKCF3Z, 2026-11-06). Its
// colour, and whether it's drawn at all, are the row's to say.
import QtQuick
import QtQuick.Shapes
import "MarkSize.js" as MarkSize

Text {
    id: mark

    // The baseline of the Name line it stands beside, in its parent's
    // terms. Not `baseline`, which is every Item's anchor line.
    property real line

    // The glyph's ink and the name's capitals as the faces draw them,
    // measured as they load at many times their size and read back down,
    // since a measure at the size drawn comes in whole pixels, a twelfth of
    // a mark's height.
    readonly property real fine: 8
    TextMetrics {
        id: markInk
        text: mark.text
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize * mark.fine
    }
    // Qt's tight measure of a glyph runs down to the baseline, so a mark
    // standing above it, as the keyboard and people do, measures taller
    // than its ink and centres high (VKCF3Z, 2026-11-05: a pixel at 1.5).
    // Its top is true; its outline gives its height. Measuring the outline
    // rather than Qt's box fixes that cause, where a size or offset tuned
    // per glyph would only hide it until the next mark.
    Shape {
        id: markOutline
        visible: false
        ShapePath {
            PathText {
                text: mark.text
                font.family: Theme.level.figure.family
                font.pointSize: Theme.level.figure.pointSize * mark.fine
            }
        }
    }
    // The ink's height, its top and its middle, at the fine size.
    readonly property real inkTall: markOutline.boundingRect.height > 0
        ? Math.min(markOutline.boundingRect.height, markInk.tightBoundingRect.height)
        : markInk.tightBoundingRect.height
    readonly property real inkMiddle: markInk.tightBoundingRect.y + inkTall / 2
    // Every mark at one optical size beside the type, whatever its shape,
    // and never wider than its cell (MarkSize.js).
    readonly property real markScale: MarkSize.scale(markInk.tightBoundingRect.width / fine,
        inkTall / fine, Theme.level.figure.pointSize, Theme.level.name.pitch)
    // Where the mark's ink centres: on the capitals' middle, the baseline
    // less half their height as drawn on this screen, whose pixels round
    // them: measured at its own scale.
    readonly property real on: line - capitalsDrawn.tightBoundingRect.height / ratio / 2
    readonly property real ratio: Screen.devicePixelRatio > 0 ? Screen.devicePixelRatio : 1
    TextMetrics {
        id: capitalsDrawn
        text: "H"
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize * mark.ratio
    }

    // How far its ink stands above the capitals' top, and as far below their
    // baseline, being centred on them: a row whose first ink is its
    // capitals' starts that much lower, so the frame stands the inset from
    // the mark too.
    readonly property real overhang: Math.max(0,
        (inkTall / fine * markScale - capitalsDrawn.tightBoundingRect.height / ratio) / 2)

    // Its ink's middle on the capitals'.
    y: on - inkMiddle / fine * markScale - baselineOffset
    font.family: Theme.level.figure.family
    font.pointSize: Theme.level.figure.pointSize * markScale
}
