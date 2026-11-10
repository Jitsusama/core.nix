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
    // A glyph is measured once in the shell and read back after
    // (MarkSize.js's known), so a row handed new words measures nothing it
    // has seen; only a glyph not yet known is given to the measures below.
    readonly property string measureKey: [Theme.level.figure.family,
        Theme.level.figure.pointSize * fine, text].join("\t")
    readonly property var known: MarkSize.known(measureKey)
    // Whether the face is there to draw in, asked as the mark is made, so a
    // measure taken in a face standing in for it is never kept. Not the
    // mark's own fontInfo, whose size follows what's measured.
    readonly property bool faceHeld: Qt.fontFamilies().includes(Theme.level.figure.family)
    readonly property string measuring: known === null ? text : ""
    TextMetrics {
        id: markInk
        text: mark.measuring
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
                text: mark.measuring
                font.family: Theme.level.figure.family
                font.pointSize: Theme.level.figure.pointSize * mark.fine
            }
        }
    }
    // Its ink's box and outline's height at the fine size, known or just
    // measured, and kept once the face drawn is the one asked for.
    readonly property var ink: known ?? MarkSize.measured(measureKey, {
        x: markInk.tightBoundingRect.x,
        y: markInk.tightBoundingRect.y,
        width: markInk.tightBoundingRect.width,
        height: markInk.tightBoundingRect.height,
        outline: markOutline.boundingRect.height
    }, faceHeld)
    // The ink's height, its top and its middle, at the fine size.
    readonly property real inkTall: ink.outline > 0 ? Math.min(ink.outline, ink.height)
        : ink.height
    readonly property real inkMiddle: ink.y + inkTall / 2
    // Every mark at one optical size beside the type, whatever its shape,
    // and never wider than its cell (MarkSize.js).
    readonly property real markScale: MarkSize.scale(ink.width / fine,
        inkTall / fine, Theme.level.figure.pointSize, Theme.level.name.pitch)
    // Where its ink's middle stands across, in its parent's terms, which a
    // mode's mark taking its cell centres on, so a / or a : stands where
    // the mark's ink did rather than on its cell's middle, which a mark
    // drawn from the cell's left at its own scale doesn't fill (Y2YBTS,
    // 2026-11-06).
    readonly property real inkCentre: x + (ink.x + ink.width / 2) / fine * markScale
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
