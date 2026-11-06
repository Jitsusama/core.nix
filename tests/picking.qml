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

        // The keys, as a Vim user's (VKCF3Z, 2026-11-06): it opens in
        // normal on the middle box, h j k l move it a step, a count
        // before them as many, H J K L size it from its bottom right, f
        // shows the cells and waits for two letters, a second f making
        // the box from that cell to another, Return takes, - goes back
        // and :q closes. Escape cancels what's begun and otherwise does
        // nothing, and the arrows move as h j k l, though no hint names
        // them.
        const view = { cells, screen: whole, step: 16 };
        const keyed = keys => {
            // With no reader of the keys, each case fails rather than the
            // run stopping on a call to nothing.
            if (typeof Picking.pressed !== "function")
                return { stood: {}, box: null, acts: [], said: "no keys read" };
            let at = { stood: Picking.rested(), box: middle, act: "", said: "" };
            const acts = [];
            for (const key of keys) {
                at = Picking.pressed(at.stood, key, Object.assign({ box: at.box }, view));
                acts.push(at.act);
            }
            return Object.assign(at, { acts });
        };
        const moves = [
            ["h", ["h"], box(84, 100, 200, 200)],
            ["l", ["l"], box(116, 100, 200, 200)],
            ["k", ["k"], box(100, 84, 200, 200)],
            ["j", ["j"], box(100, 116, 200, 200)],
            ["3l", ["3", "l"], box(148, 100, 200, 200)],
            ["12j", ["1", "2", "j"], box(100, 292, 200, 200)],
            ["a count goes with its move", ["3", "l", "l"], box(164, 100, 200, 200)],
            ["L", ["L"], box(100, 100, 216, 200)],
            ["H", ["H"], box(100, 100, 184, 200)],
            ["J", ["J"], box(100, 100, 200, 216)],
            ["K", ["K"], box(100, 100, 200, 184)],
            ["2L", ["2", "L"], box(100, 100, 232, 200)],
            ["an arrow moves as its letter", ["Left", "Down"], box(84, 116, 200, 200)],
            ["Shift and an arrow size as its capital", ["Shift+Right"],
                box(100, 100, 216, 200)],
            ["f and a cell's letters make the box that cell", ["f", "c", "f"],
                box(450, 180, 90, 90)],
            ["a second f makes it from the first cell to the other",
                ["f", "c", "f", "f", "a", "b"], box(90, 0, 450, 270)],
            ["a move after a cell carries on from it, and a new f starts afresh",
                ["f", "a", "a", "l", "f", "a", "b"], box(90, 0, 90, 90)],
            ["Escape cancels a waiting f, keeping the box", ["f", "c", "Escape", "l"],
                box(116, 100, 200, 200)],
            ["Escape cancels a count", ["3", "Escape", "l"], box(116, 100, 200, 200)],
            ["a letter leading to no cell is let be", ["f", "z", "c", "f"],
                box(450, 180, 90, 90)],
            ["Backspace takes back a letter", ["f", "c", "Backspace", "a", "a"],
                box(0, 0, 90, 90)],
            ["the keys move with their list up", ["Space", "?", "l"],
                box(116, 100, 200, 200)],
            ["Escape lets a begun Space go", ["Space", "Escape", "l"],
                box(116, 100, 200, 200)]
        ];
        for (const [what, keys, wanted] of moves)
            expect(what, keyed(keys).box, wanted);
        const acting = [
            ["Return takes", ["Return"], ["take"]],
            ["- goes back", ["-"], ["back"]],
            [":q closes", [":", "q", "Return"], ["", "", "close"]],
            ["Escape at rest does nothing", ["Escape", "Escape"], ["", ""]],
            ["Escape leaves the command line", [":", "q", "Escape", "Return"],
                ["", "", "", "take"]],
            ["Return takes while f waits, the box as it is", ["f", "c", "Return"],
                ["", "", "take"]],
            ["q alone closes nothing", ["q"], [""]]
        ];
        for (const [what, keys, wanted] of acting) {
            asked++;
            const got = keyed(keys).acts;
            if (JSON.stringify(got) !== JSON.stringify(wanted))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(wanted)}`);
        }
        const sayings = [
            ["a command it doesn't know is Vim's E492", [":", "w", "Return"],
                "E492: Not an editor command: w"],
            ["a key meaning nothing says so", ["q"], "not here"],
            ["the command line says what's typed", [":", "q"], ":q"],
            ["f says it waits for a cell", ["f", "c"], "f c"],
            ["a count says itself", ["1", "2"], "12"],
            ["a begun Space says itself", ["Space"], "Space"],
            ["Space then a key meaning nothing says so", ["Space", "x"], "not here"]
        ];
        // Space ? lists the picker's keys, as on every surface; the list
        // stays up as the keys are used, and goes with Space ? again or an
        // Escape with nothing begun.
        const listings = [
            ["Space ? lists the keys", ["Space", "?"], true],
            ["Space ? again puts them away", ["Space", "?", "Space", "?"], false],
            ["the list stays up as the keys are used", ["Space", "?", "l", "f", "c"], true],
            ["Escape with nothing begun puts it away", ["Space", "?", "Escape"], false],
            ["Escape cancels a count first", ["Space", "?", "3", "Escape"], true],
            ["Space alone lists nothing", ["Space"], false]
        ];
        for (const [what, keys, wanted] of listings) {
            asked++;
            const got = keyed(keys).stood.listing === true;
            if (got !== wanted)
                failures.push(`${what}: listing is ${got}, not ${wanted}`);
        }
        // The list names every key the foot does, and Escape nowhere.
        asked++;
        const listed = typeof Picking.keyList === "function" ? Picking.keyList() : [];
        const named = listed.map(line => line.keys);
        // A count is its own line, as the launcher's list says one
        // (VKCF3Z, 2026-11-06).
        for (const key of ["f", "h j k l", "3l", "H J K L", "\u21b5", "-", ":q", "Space ?"]) {
            if (!named.includes(key))
                failures.push(`the key list leaves out ${key}`);
        }
        if (named.some(key => key.includes("Esc")))
            failures.push("the key list names Escape");

        for (const [what, keys, wanted] of sayings) {
            asked++;
            const at = keyed(keys);
            const got = at.said !== "" ? at.said : Picking.typed(at.stood);
            if (got !== wanted)
                failures.push(`${what}: "${got}", not "${wanted}"`);
        }

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
