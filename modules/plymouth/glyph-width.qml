// Says how far Prompt.qml's glyph leads what's typed, its glyphWidth, as Qt
// works it out, for the splash to start what's typed where the lock's field
// does. Qt rounds a face to whole pixels and counts the ink a glyph throws
// past its advance, so pango's own measure of it differs.
// usage: qml glyph-width.qml -- <glyph> <family> <points> <room after it>
import QtQuick
import QtQuick.Window

Window {
    id: root

    readonly property var given: Qt.application.arguments.slice(-4)
    // An int, as Prompt.qml's is, which drops what's past the whole pixel.
    property int glyphWidth: glyph.implicitWidth + Number(root.given[3])

    Text {
        id: glyph

        text: root.given[0]
        font.family: root.given[1]
        font.pointSize: Number(root.given[2])
    }

    Component.onCompleted: {
        console.log(root.glyphWidth);
        Qt.quit();
    }
}
