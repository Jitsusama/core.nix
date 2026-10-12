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

        // The arrows keep the half they point at, until it's too small.
        expect("left", Picking.halved(whole, "left"), box(0, 0, 720, 900));
        expect("right", Picking.halved(whole, "right"), box(720, 0, 720, 900));
        expect("up", Picking.halved(whole, "up"), box(0, 0, 1440, 450));
        expect("down", Picking.halved(whole, "down"), box(0, 450, 1440, 450));
        expect("right of an odd width", Picking.halved(box(10, 0, 101, 50), "right"),
            box(61, 0, 50, 50));
        expect("down at the least", Picking.halved(box(0, 0, 50, 16), "down"), box(0, 0, 50, 16));
        expect("left near the least", Picking.halved(box(0, 0, 20, 50), "left"),
            box(0, 0, 16, 50));

        // Shift moves it by its size, and stops at the screen's edge.
        expect("shift right", Picking.moved(box(0, 0, 720, 450), "right", screen),
            box(720, 0, 720, 450));
        expect("shift right past the edge", Picking.moved(box(900, 0, 400, 100), "right", screen),
            box(1040, 0, 400, 100));
        expect("shift up at the top", Picking.moved(box(0, 0, 720, 450), "up", screen),
            box(0, 0, 720, 450));
        expect("shift down", Picking.moved(box(0, 100, 720, 300), "down", screen),
            box(0, 400, 720, 300));

        // Ctrl pushes an edge out, to the screen's edge and no further.
        expect("ctrl left", Picking.pushed(box(100, 100, 200, 200), "left", 16, screen),
            box(84, 100, 216, 200));
        expect("ctrl left at the edge", Picking.pushed(box(8, 100, 200, 200), "left", 16, screen),
            box(0, 100, 208, 200));
        expect("ctrl right", Picking.pushed(box(100, 100, 200, 200), "right", 16, screen),
            box(100, 100, 216, 200));
        expect("ctrl down at the edge",
            Picking.pushed(box(100, 800, 200, 90), "down", 16, screen), box(100, 800, 200, 100));

        // Ctrl and Shift pull the edge the arrow faces, the way it points.
        expect("ctrl shift left", Picking.pulled(box(100, 100, 200, 200), "left", 16),
            box(100, 100, 184, 200));
        expect("ctrl shift right", Picking.pulled(box(100, 100, 200, 200), "right", 16),
            box(116, 100, 184, 200));
        expect("ctrl shift up", Picking.pulled(box(100, 100, 200, 200), "up", 16),
            box(100, 100, 200, 184));
        expect("ctrl shift down at the least", Picking.pulled(box(100, 100, 200, 20), "down", 16),
            box(100, 104, 200, 16));

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
