// How a pixel mark (the empty places' cat) is laid on a screen, run in Qt's
// own JavaScript engine, the one the shell runs it in. The
// `the-cat-is-drawn-in-whole-pixels` check copies modules/quickshell/Pixels.js
// beside this file and runs `qml -platform offscreen pixels.qml`.
import QtQuick
import "Pixels.js" as Pixels

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    function run() {
        const failures = [];
        const check = (what, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };

        check("each row's stretch of ink is one run, to the row's end",
            Pixels.runs(["#..##", ".###.", "....#"]),
            [{ x: 0, y: 0, across: 1 }, { x: 3, y: 0, across: 2 }, { x: 1, y: 1, across: 3 },
                { x: 4, y: 2, across: 1 }]);
        check("and a row with none is none", Pixels.runs(["....", ""]), []);

        // A mark's 15 logical pixels over 8 cells: 2 of the screen's pixels a
        // cell at 1, 3 at 1.5 and 4 at 2, so 16 logical pixels at each.
        check("a cell is a whole number of the screen's pixels at each scale",
            [1, 1.5, 2].map(ratio => Pixels.cell(15, 8, ratio)), [2, 3, 4]);
        check("never none", Pixels.cell(1, 8, 1), 1);

        // At 1.5 a logical pixel is a pixel and a half, so one at 11 starts
        // halfway through the screen's 16th pixel.
        const whole = (at, ratio) => {
            const pixels = (at + Pixels.snap(at, ratio)) * ratio;
            return Math.abs(pixels - Math.round(pixels)) < 1e-9;
        };
        check("a drawing moves to start on one of the screen's pixels",
            [[11, 1.5], [7.25, 1.5], [3.9, 2], [10, 1.5]].map(([at, ratio]) => whole(at, ratio)),
            [true, true, true, true]);
        check("by less than one, and not at all where it already does",
            [Math.abs(Pixels.snap(11, 1.5)) < 1, Pixels.snap(10, 1.5), Pixels.snap(10.5, 2)],
            [true, 0, 0]);

        if (failures.length > 0) {
            failures.forEach(failure => console.warn(`FAIL ${failure}`));
            Qt.exit(1);
        } else {
            Qt.exit(0);
        }
    }
}
