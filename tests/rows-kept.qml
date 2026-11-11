// How the launcher's list keeps its rows, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `a-row-kept-is-never-drawn-again`
// check copies modules/quickshell/launcher/RowsKept.js and tests/lib/Expect.js
// beside this file and runs it offscreen. Each case does the steps to a list
// as Launcher.qml's keep does them to its ListModel, and holds that they end
// on the rows listed now, drawing afresh only those it didn't hold.
import QtQuick
import "Expect.js" as Expect
import "RowsKept.js" as RowsKept

QtObject {
    // A case that doesn't hold throws, naming it, and ends as a failure,
    // rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // The rows the steps leave, and those they drew afresh, as ListModel's
    // remove, move and insert would.
    function done(was, now) {
        const rows = was.slice();
        const drawn = [];
        for (const step of RowsKept.steps(was, now)) {
            if (step.remove !== undefined) {
                rows.splice(step.remove, step.count);
            } else if (step.move !== undefined) {
                rows.splice(step.to, 0, ...rows.splice(step.move, 1));
            } else {
                rows.splice(step.insert, 0, ...step.keys);
                drawn.push(...step.keys);
            }
        }
        return { rows: rows.join(" "), drawn: drawn.join(" ") };
    }

    function run() {
        // [what, the rows held, the rows listed now, what's drawn afresh]
        const cases = [
            ["the same rows take no step", "a b c", "a b c", ""],
            ["a letter narrowing keeps the rows between", "a b c d e", "b d", ""],
            ["rows moving keep what they drew", "a b c d", "d b a c", ""],
            ["only a row new to the list is drawn", "a b c", "c x a", "x"],
            ["a first list draws every row", "", "a b", "a b"],
            ["and nothing listed leaves nothing", "a b c", "", ""],
            ["a key listed twice is two rows", "a b a", "a a", ""],
            ["and a second of it is drawn", "a b", "b a a", "a"],
            ["a list wholly new draws each row", "a b", "c d", "c d"]
        ];
        for (const [what, was, now, drawn] of cases) {
            const words = said => said.split(" ").filter(word => word !== "");
            Expect.same(what, done(words(was), words(now)), { rows: now, drawn });
        }
        Expect.same("the same rows take no step at all", RowsKept.steps(["a", "b"], ["a", "b"]),
            []);
        // [what, the rows held, the rows listed now, the steps]
        const moves = [
            ["a row falling to the end is one move", "a b c d e", "b c d e a",
                [{ move: 0, to: 4 }]],
            ["a row rising to the top is one move", "a b c d e", "e a b c d",
                [{ move: 4, to: 0 }]],
            ["a row falling partway is one move", "a b c d e", "b c d a e",
                [{ move: 0, to: 3 }]],
            ["rows falling past each other move once each", "a b c d e", "c d e b a",
                [{ move: 0, to: 4 }, { move: 0, to: 3 }]]
        ];
        for (const [what, was, now, steps] of moves) {
            Expect.same(what, RowsKept.steps(was.split(" "), now.split(" ")), steps);
        }
        const many = "a b c d e f g h".split(" ");
        const swapped = ["h", ...many.slice(1, 7), "a"];
        Expect.same("two rows swapping ends move no row between",
            RowsKept.steps(many, swapped).length, 2);
        Expect.same("and end where they're listed", done(many, swapped),
            { rows: swapped.join(" "), drawn: "" });
        Expect.same("rows new to the list go in as one step a run",
            RowsKept.steps("a b".split(" "), "x y a z b w v".split(" ")),
            [{ insert: 0, keys: ["x", "y"] }, { insert: 3, keys: ["z"] },
                { insert: 5, keys: ["w", "v"] }]);
        // A step into Emoji lists 1,924 rows the list didn't hold, which
        // went in one step each, each looked for among those held first.
        const emoji = Array.from({ length: 1924 }, (_, at) => `emoji:${at}`);
        Expect.same("a first list of many is one step", RowsKept.steps([], emoji),
            [{ insert: 0, keys: emoji }]);
        // "f" in Emoji ranks 838 of its rows, best first, which moved 550 of
        // them one at a time, each move reading the rows in view again.
        const narrowed = emoji.filter((_, at) => at % 16 < 7);
        const ranked = narrowed.map((key, at) => [key, (at * 7919) % narrowed.length])
            .sort((one, other) => one[1] - other[1])
            .map(([key]) => key);
        Expect.same("a reorder moving more rows than the card shows is listed afresh",
            RowsKept.steps(emoji, ranked, 8),
            [{ remove: 0, count: emoji.length }, { insert: 0, keys: ranked }]);
        Expect.same("and one moving no more keeps its rows",
            RowsKept.steps("a b c d e".split(" "), "e a b c d".split(" "), 1),
            [{ move: 4, to: 0 }]);
        Expect.same("and narrowing moves none, however many go",
            RowsKept.steps(emoji, narrowed, 8).filter(step =>
                step.move !== undefined || step.insert !== undefined), []);
        // Rows all put in again stand from the list's start, so the launcher
        // puts the selection back in view; moved or narrowed, they stay put.
        Expect.same("rows listed afresh are said to be",
            RowsKept.whole(emoji, RowsKept.steps(emoji, ranked, 8)), true);
        Expect.same("and rows moved are not",
            RowsKept.whole("a b c d e".split(" "),
                RowsKept.steps("a b c d e".split(" "), "e a b c d".split(" "), 1)), false);
        Expect.same("nor rows narrowed",
            RowsKept.whole(emoji, RowsKept.steps(emoji, narrowed, 8)), false);
        Expect.same("nor a first list", RowsKept.whole([], RowsKept.steps([], emoji)), false);

        // Lists drawn from a seeded sequence (Park and Miller's), the same
        // every run, each ending where it's listed and drawing only the
        // rows it didn't hold.
        let seed = 7;
        const next = most => {
            seed = (seed * 48271) % 2147483647;
            return seed % most;
        };
        const keys = "a b c d e f g h i j".split(" ");
        const pickedFrom = () => Array.from({ length: next(9) }, () => keys[next(keys.length)]);
        for (let each = 0; each < 500; each += 1) {
            const was = pickedFrom();
            const now = pickedFrom();
            const ended = done(was, now);
            const held = was.slice();
            const fresh = now.filter(key => {
                const at = held.indexOf(key);
                if (at >= 0) {
                    held.splice(at, 1);
                }
                return at < 0;
            });
            Expect.same(`${was.join("")} to ${now.join("")} ends where it's listed`,
                ended.rows, now.join(" "));
            Expect.same(`${was.join("")} to ${now.join("")} draws only rows it didn't hold`,
                ended.drawn.split(" ").filter(key => key !== "").sort(), fresh.sort());
        }

        Qt.exit(0);
    }
}
