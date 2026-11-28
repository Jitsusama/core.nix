// Draws the theme's triangle and its turned triangle through the one mark
// (Mark.qml), side by side in cells of their own, on lines a quarter of a
// point apart, since where a line falls between pixels moves its ink. Keeps
// a picture of each and says what each is, and where its mark says its ink
// starts and ends down its cell. modules/quickshell/tests/turned-mark.py
// reads them back to the ink, at the scale the shell ran at.
pragma ComponentBehavior: Bound
import Quickshell
import QtQuick

ShellRoot {
    id: root

    readonly property var offsets: [0, 0.25, 0.5, 0.75]
    readonly property var cellsMade: [].concat(...offsets.map(by => [
        { name: `upright-${by}`, glyph: Theme.glyph.shapeTriangle, by: by },
        { name: `turned-${by}`, glyph: Theme.glyph.shapeDownTriangle, by: by }
    ]))
    // How wide and tall each cell is, in logical pixels, which the
    // pictures' own size gives the scale against.
    readonly property int cell: 100
    property int grabbed: 0

    FloatingWindow {
        visible: true
        implicitWidth: root.cell * root.cellsMade.length
        implicitHeight: root.cell
        color: "black"

        Row {
            Repeater {
                id: cells
                model: root.cellsMade

                Rectangle {
                    required property var modelData
                    width: root.cell
                    height: root.cell
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
            cell: root.cell,
            // Where each mark says its ink stands down its cell, its parent,
            // which its half turn doesn't move: what the launcher's reader
            // takes for a mark's ink (Launcher.qml's drawnTexts).
            inks: root.cellsMade.map((made, at) => ({ name: made.name,
                from: cells.itemAt(at).children[0].inkFrom,
                to: cells.itemAt(at).children[0].inkTo })),
        })}`);
        Qt.callLater(() => Qt.exit(0));
    }
}
