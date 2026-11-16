// How the margin cat is petted, and picked up, with the mouse
// (MarginCat.js), in Qt's own
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
        swats(expect);
        held(expect);
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

    // A swat (VKCF3Z, 73c59942): a pointer crossing it on its way elsewhere,
    // gone before a pet would lean it in, has it swat after it now and then,
    // never so often it nags, and never as it dozes.
    function swats(expect) {
        const rolls = Array.from({ length: 100 }, (_, i) => i / 100);
        expect("a pointer crossing it swats about one time in three",
            rolls.filter(roll => MarginCat.swats(true, 30000, false, roll)).length, 34);
        expect("never one that stopped on it, never twice within 20 s, never asleep",
            [MarginCat.swats(false, 30000, false, 0), MarginCat.swats(true, 19999, false, 0),
                MarginCat.swats(true, 20000, false, 0), MarginCat.swats(true, 30000, true, 0)],
            [false, false, true, false]);
        // On the screen a swat ran 83 ms past its hold, from the paw's
        // first frame to the sit, at 60 Hz and at 120 (VKCF3Z, 38d40d08: 283
        // ms with a 200 ms hold), and the paw must be back within 250.
        expect("its paw is back down within 250 ms of showing", MarginCat.SWATS + 83 <= 250,
            true);
        // Each drawing's cells as they land on the screen, from three cells
        // left of where it sits.
        const padded = line => line.padEnd(12, ".");
        const screen = (drawing, out) => drawing.map(row => padded(".".repeat(3 - out) + row));
        const sitting = screen(Drawings.poses.nothingHere, 0);
        const toLeft = screen(MarginCat.swatting(-1, Drawings.poses), MarginCat.swatsOut(-1));
        const toRight = screen(MarginCat.swatting(1, Drawings.poses), MarginCat.swatsOut(1));
        expect("swatting either way, its base and tail are on sitting's cells",
            [toLeft.slice(5), toRight.slice(5)], [sitting.slice(5), sitting.slice(5)]);
        // Its paw is what's lit that sitting doesn't light, by columns.
        const paw = drawing => drawing.slice(0, 5).reduce((lit, row, r) => lit.concat(row
            .split("").map((cell, c) => cell === "#" && sitting[r][c] !== "#" ? c : -1)
            .filter(c => c >= 0)), []);
        const head = sitting.slice(0, 5).map(row => [row.indexOf("#"), row.lastIndexOf("#")]);
        const left = Math.min(...head.map(ends => ends[0]).filter(c => c >= 0));
        const right = Math.max(...head.map(ends => ends[1]));
        expect("a paw raised toward the side it left by, beside its head, the rest unmoved",
            [paw(toLeft).length > 0 && paw(toLeft).every(c => c < left),
                paw(toRight).length > 0 && paw(toRight).every(c => c > right),
                toLeft.slice(0, 5).map(row => row.replace(/^.{3}/, "...")),
                toRight.slice(0, 5).map(row => row.slice(0, 8))],
            [true, true, sitting.slice(0, 5), sitting.slice(0, 5).map(row => row.slice(0, 8))]);
        expect("either way, the same paw",
            toLeft.slice(0, 5).map(row => row.slice(0, 3).split("").reverse().join("")),
            toRight.slice(0, 5).map(row => row.slice(8, 11)));
    }

    // Picked up by Joel's hand (VKCF3Z, 9ead6710; Joel, 2026-11-12: "i want
    // to be able to also use my mouse to engage with my animals."): it hangs
    // by its scruff under the pointer, wholly on the screen; and it's over
    // the windows while held and as it falls, then as the ledge it landed
    // on says, then by its seat.
    function held(expect) {
        const at = { cell: 1, wide: 8, tall: 8, across: 1000, height: 600 };
        expect("held by the scruff, it hangs under the pointer, wholly on the screen",
            [MarginCat.held({ x: 100, y: 50 }, at), MarginCat.held({ x: 1, y: 0 }, at),
                MarginCat.held({ x: 999, y: 599 }, at)],
            [{ x: 96.5, y: 49.5 }, { x: 0, y: 0 }, { x: 992, y: 592 }]);
        const tiled = { floats: false };
        const floating = { floats: true };
        expect("over the windows held and falling, then by its landing, then its seat",
            [MarginCat.inFront({ holds: true, seat: tiled }),
                MarginCat.inFront({ falls: true, landed: tiled }),
                MarginCat.inFront({ landed: tiled, seat: floating }),
                MarginCat.inFront({ landed: floating, seat: tiled }),
                MarginCat.inFront({ landed: null, seat: floating }),
                MarginCat.inFront({ seat: floating, leaving: true }),
                MarginCat.inFront({ seat: tiled, leaving: true, aboard: true })],
            [true, true, false, true, true, false, true]);
        expect("its scruffed drawing hangs whole, as wide as it sits",
            [Drawings.poses.scruffed?.length, Drawings.poses.scruffed?.every(row =>
                row.length === 8)],
            [8, true]);
        // The GPU rig, 2026-11-16: wanting only its own tile while held, it
        // knew no column under Joel's hand and fell past a lower top.
        expect("every tile is wanted while it's held, as while it travels, and only then",
            [MarginCat.travels({ holding: true }), MarginCat.travels({ going: true }),
                MarginCat.travels({ going: true, handsBack: true }),
                MarginCat.travels({ moving: true }), MarginCat.travels({ resting: true }),
                MarginCat.travels({})],
            [true, true, false, true, true, false]);
    }
}
