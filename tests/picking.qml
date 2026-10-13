// What the region picker's keys must do to its box, run in Qt's own
// JavaScript engine, the one the shell runs them in. The
// `the-picker-picks-what-the-keys-say` check copies Picking.js beside this
// file and runs `qml -platform offscreen picking.qml`. Each case names the
// keys, the box they start from, and the box that has to come back.
import QtQuick
import "Picking.js" as Picking

QtObject {
    readonly property var screen: ({ x: 1440, y: 0, width: 1440, height: 900 })
    readonly property var whole: ({ x: 0, y: 0, width: 1440, height: 900 })

    function box(x, y, width, height) {
        return { x, y, width, height };
    }

    function said(box) {
        return box === null ? "nothing" : `${box.x},${box.y} ${box.width}x${box.height}`;
    }

    Component.onCompleted: {
        const failures = [];
        let asked = 0;
        const expect = (what, got, wanted) => {
            asked++;
            if (said(got) !== said(wanted))
                failures.push(`${what} gave ${said(got)}, not ${said(wanted)}`);
        };

        // It starts as the middle quarter.
        expect("the start", Picking.start(whole), box(360, 225, 720, 450));
        expect("the start of an odd screen", Picking.start({ width: 1281, height: 801 }),
            box(320, 200, 641, 401));

        // The arrows move it a step, and stop at the screen's edge.
        const middle = box(100, 100, 200, 200);
        expect("left", Picking.moved(middle, "left", 16, screen), box(84, 100, 200, 200));
        expect("right", Picking.moved(middle, "right", 16, screen), box(116, 100, 200, 200));
        expect("up", Picking.moved(middle, "up", 16, screen), box(100, 84, 200, 200));
        expect("down", Picking.moved(middle, "down", 16, screen), box(100, 116, 200, 200));
        expect("left at the edge", Picking.moved(box(8, 0, 200, 200), "left", 16, screen),
            box(0, 0, 200, 200));
        expect("right at the edge", Picking.moved(box(1230, 0, 200, 200), "right", 16, screen),
            box(1240, 0, 200, 200));
        expect("down at the edge", Picking.moved(box(0, 690, 200, 200), "down", 16, screen),
            box(0, 700, 200, 200));

        // Shift sizes it from its bottom right corner, within the screen and
        // no smaller than the least.
        expect("shift right", Picking.sized(middle, "right", 16, screen), box(100, 100, 216, 200));
        expect("shift left", Picking.sized(middle, "left", 16, screen), box(100, 100, 184, 200));
        expect("shift down", Picking.sized(middle, "down", 16, screen), box(100, 100, 200, 216));
        expect("shift up", Picking.sized(middle, "up", 16, screen), box(100, 100, 200, 184));
        expect("shift right at the edge",
            Picking.sized(box(1300, 0, 130, 50), "right", 16, screen), box(1300, 0, 140, 50));
        expect("shift up at the least", Picking.sized(box(0, 0, 50, 20), "up", 16, screen),
            box(0, 0, 50, 16));

        // Letters name cells, row then column, alphabetically.
        const cells = Picking.grid(whole);
        asked++;
        if (cells.columns !== 16 || cells.rows !== 10)
            failures.push(`the grid is ${cells.columns} by ${cells.rows}, not 16 by 10`);
        expect("aa", Picking.cell(cells, "aa"), box(0, 0, 90, 90));
        expect("cf", Picking.cell(cells, "cf"), box(450, 180, 90, 90));
        expect("jp", Picking.cell(cells, "jp"), box(1350, 810, 90, 90));
        expect("one letter", Picking.cell(cells, "c"), null);
        expect("a row past the last", Picking.cell(cells, "ka"), null);
        expect("a column past the last", Picking.cell(cells, "aq"), null);
        for (const [typed, fits] of [["", true], ["j", true], ["k", false], ["jp", true],
                ["jq", false]]) {
            asked++;
            if (Picking.fits(cells, typed) !== fits)
                failures.push(`"${typed}" ${fits ? "didn't fit" : "fitted"}`);
        }
        expect("cf to ab", Picking.spanned(Picking.cell(cells, "cf"), Picking.cell(cells, "ab")),
            box(90, 0, 450, 270));

        // The desktop's coordinates start where the screen does.
        asked++;
        if (Picking.geometry(box(10, 20, 300, 200), screen) !== "1450,20 300x200")
            failures.push(`the geometry is ${Picking.geometry(box(10, 20, 300, 200), screen)}`);

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
