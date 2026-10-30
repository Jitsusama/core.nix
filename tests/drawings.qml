// The margin's marks as Qt draws them, in its own engine. The
// `the-margins-marks-are-drawn-level` check writes Drawings.js beside this
// file, with every mark an overview piece of the graphical account shows
// (`marks`) and the theme's drawing of each mark (`drawn`, empty for none),
// copies Devices.js beside it for the kinds a device is drawn as, then runs
// `qml -platform offscreen drawings.qml`.
import QtQuick
import QtQuick.Shapes
import "Devices.js" as Devices
import "Drawings.js" as Drawings

Item {
    id: root

    // The grid every drawing is made on (jitsusama.theme.marks).
    readonly property int grid: 12

    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // A drawing, with no stroke at all, since a stroke's width, even one not
    // drawn, is taken off the bounds.
    Component {
        id: drawing

        Shape {
            property alias path: svg.path

            ShapePath {
                strokeWidth: 0

                PathSvg {
                    id: svg
                }
            }
        }
    }

    // A drawing's bounds as Qt finds them.
    function bounds(path) {
        return (drawing.createObject(root, { path }) as Shape).boundingRect;
    }

    function run() {
        const failures = [];
        const near = (a, b) => Math.abs(a - b) < 1e-6;

        // Every mark the margin can show has a drawing, since a font's
        // glyph at its size blurs, and would stand beside the drawings at a
        // height of its own.
        for (const mark of Drawings.marks) {
            if (!Drawings.drawn[mark])
                failures.push(`${mark} has no drawing`);
        }

        // So has every kind a device running low is drawn as in its place
        // (pieces/Devices.qml), which no overview piece names.
        for (const kind of Devices.kinds()) {
            if (!Drawings.drawn[kind])
                failures.push(`${kind}, a device's kind, has no drawing`);
        }

        // Each touches the grid's top and bottom, so all stand the same
        // height in a row, and is centred across it, so the room either side
        // of it in its cell is the same.
        for (const [mark, path] of Object.entries(Drawings.drawn)) {
            if (path === "")
                continue;
            const box = bounds(path);
            if (!near(box.top, 0) || !near(box.bottom, root.grid))
                failures.push(`${mark} stands from ${box.top} to ${box.bottom},`
                    + ` not 0 to ${root.grid}`);
            if (!near(box.left + box.right, root.grid) || box.left < -1e-6)
                failures.push(`${mark} spans ${box.left} to ${box.right}, off the grid's middle`);
        }

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
