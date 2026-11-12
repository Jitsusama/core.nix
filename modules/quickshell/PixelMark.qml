// A pixel drawing of the theme's (jitsusama.theme.marks' `pixels`) where a
// row's glyph would stand: the empty places' cat, on the launcher's line
// saying nothing. Each cell is a whole number of the screen's pixels and
// the drawing starts on one, so it's as sharp at 1.5 as at 2 (Pixels.js);
// it's still, so it costs no frame past the one its row draws in, and with
// no rows it's nothing at all.
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Window
import "MarkSize.js" as MarkSize
import "Pixels.js" as Pixels

Item {
    id: drawing

    // Its rows, top to bottom, each a string with # for ink.
    property var rows: []
    property color hue
    // About how tall it stands: as tall as a mark's ink beside the type.
    readonly property real size: MarkSize.size(Theme.level.figure.pointSize)

    // The window's own ratio, which a fractional scale sets, rather than
    // the screen's, as Hairline.qml reads it: the screen says a whole one.
    readonly property real ratio: Window.window?.devicePixelRatio
        ?? (Screen.devicePixelRatio > 0 ? Screen.devicePixelRatio : 1)
    readonly property int cells: rows.length
    readonly property real cell: Pixels.cell(size, Math.max(1, cells), ratio) / ratio
    // How far its cells are moved to start on the screen's own pixels,
    // found once it's shown, since where its card lies is known only then.
    property point snapped: Qt.point(0, 0)

    width: cells * cell
    height: cells * cell
    onVisibleChanged: Qt.callLater(drawing.snap)
    Component.onCompleted: Qt.callLater(drawing.snap)
    // And again once the card has settled, since it may still be gliding
    // in as the line shows.
    Timer {
        interval: 400
        running: drawing.visible && drawing.cells > 0
        onTriggered: drawing.snap()
    }

    function snap() {
        if (!visible || cells === 0)
            return;
        const at = mapToItem(null, 0, 0);
        snapped = Qt.point(Pixels.snap(at.x, ratio), Pixels.snap(at.y, ratio));
    }

    // Where its first cell starts, in the screen's pixels, for the
    // launcher's report.
    function drawnAt() {
        const at = mapToItem(null, snapped.x, snapped.y);
        return [at.x * ratio, at.y * ratio];
    }

    Repeater {
        model: Pixels.runs(drawing.rows)

        Rectangle {
            required property var modelData

            x: drawing.snapped.x + modelData.x * drawing.cell
            y: drawing.snapped.y + modelData.y * drawing.cell
            width: modelData.across * drawing.cell
            height: drawing.cell
            color: drawing.hue
        }
    }
}
