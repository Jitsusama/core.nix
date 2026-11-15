// How the margin cat is petted with the mouse (MarginCat.js), in Qt's own
// JavaScript engine, the one the shell runs it in, with the theme's own
// drawings of it sitting and leaning. The
// `the-margin-cat-sits-above-its-column` check copies modules/margin-cat's
// scripts beside this file, with those drawings as MarginCatDrawings.js, and
// runs `qml -platform offscreen margin-cat-pets.qml`.
import QtQuick
import "MarginCat.js" as MarginCat
import "MarginCatDrawings.js" as Drawings

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
        const expect = (name, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${name}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };
        strokes(expect);
        leans(expect);
        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }

    // Petted with the mouse (VKCF3Z, b8fc852e): a pointer that moved onto
    // it as it sits, never one it walked or sat down under, nor one passing
    // over, which leaves before the pet's 300 ms.
    function strokes(expect) {
        const hand = { x: 405, y: 7 };
        expect("a pointer moved onto it as it sits strokes it",
            [MarginCat.strokes(null, hand, 5000), MarginCat.strokes({ x: 404, y: 7 }, hand, 5000)],
            [true, true]);
        expect("never one it came to under, still where it was",
            [MarginCat.strokes(null, hand, -1), MarginCat.strokes(null, hand, 50),
                MarginCat.strokes({ x: 405, y: 7 }, hand, 5000)],
            [false, false, false]);
        expect("resting on it 300 ms, and leaning 600 ms after it goes",
            [MarginCat.PETS, MarginCat.PETTED], [300, 600]);
    }

    // Its lean toward the hand (VKCF3Z, 10bacc58): only its head moves, so
    // it never hops away from the hand.
    function leans(expect) {
        expect("it leans toward the pointer's side, a lean left drawn from a cell out",
            [MarginCat.leansTo(3, 16), MarginCat.leansOut(-1),
                MarginCat.leansTo(12, 16), MarginCat.leansOut(1)],
            [-1, 1, 1, 0]);
        // Each drawing's cells as they land on the screen, from a cell left
        // of where it sits.
        const padded = line => line.padEnd(10, ".");
        const screen = (drawing, out) => drawing.map(row => padded(".".repeat(1 - out) + row));
        const sitting = screen(Drawings.poses.nothingHere, 0);
        const toLeft = screen(MarginCat.leaning(-1, Drawings.poses), MarginCat.leansOut(-1));
        const toRight = screen(MarginCat.leaning(1, Drawings.poses), MarginCat.leansOut(1));
        const moved = (row, way) => way > 0 ? `.${row.slice(0, -1)}` : `${row.slice(1)}.`;
        expect("leaning either way, its base is on sitting's cells, and leaning left its tail too",
            [toLeft[7], toRight[7], toLeft.slice(5)], [sitting[7], sitting[7], sitting.slice(5)]);
        expect("and only its head a cell toward the hand, eyes shut",
            [toLeft[0], toRight[0], toLeft[1], toRight[1], /#\.#/.test(toLeft[2] + toRight[2])],
            [moved(sitting[0], -1), moved(sitting[0], 1), moved(sitting[1], -1),
                moved(sitting[1], 1), false]);
    }
}
