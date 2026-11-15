// Draws the theme's triangle and its turned triangle through the one mark
// (Mark.qml), side by side in cells of their own, on lines a quarter of a
// point apart, since where a line falls between pixels moves its ink. Keeps
// a picture of each and says what each is. modules/quickshell/tests/
// turned-mark.py reads them back to the ink, at the scale the shell ran at.
import Quickshell
import QtQuick

ShellRoot {
    id: root

    readonly property var offsets: [0, 0.25, 0.5, 0.75]
    readonly property var cellsMade: [].concat(...offsets.map(by => [
        { name: `upright-${by}`, glyph: Theme.glyph.shapeTriangle, by: by },
        { name: `turned-${by}`, glyph: Theme.glyph.shapeDownTriangle, by: by }
    ]))
    property int grabbed: 0

    FloatingWindow {
        visible: true
        implicitWidth: 100 * root.cellsMade.length
        implicitHeight: 100
        color: "black"

        Row {
            Repeater {
                id: cells
                model: root.cellsMade

                Rectangle {
                    required property var modelData
                    width: 100
                    height: 100
                    color: "black"

                    Mark {
                        x: 30
                        line: 60 + parent.modelData.by
                        text: parent.modelData.glyph
                        color: "white"
                    }
                }
            }
        }
    }

    // Once every cell is drawn, keep a picture of each and say what they are.
    Timer {
        interval: 500
        running: true
        onTriggered: {
            for (let at = 0; at < cells.count; at += 1) {
                const name = root.cellsMade[at].name;
                cells.itemAt(at).grabToImage(picture => {
                    if (!picture.saveToFile(`${Quickshell.shellDir}/${name}.png`)) {
                        console.warn(`FAIL couldn't keep the picture of ${name}`);
                        Qt.exit(1);
                        return;
                    }
                    root.grabbed += 1;
                    if (root.grabbed === cells.count)
                        root.say();
                });
            }
        }
    }

    function say() {
        console.log(`MEANT ${JSON.stringify({
            offsets: root.offsets,
            turned: Theme.turned[Theme.glyph.shapeDownTriangle] === true,
            upright: Theme.turned[Theme.glyph.shapeTriangle] === true,
        })}`);
        Qt.callLater(() => Qt.exit(0));
    }
}
