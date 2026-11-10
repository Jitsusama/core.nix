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
                rows.splice(step.insert, 0, step.key);
                drawn.push(step.key);
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

        Qt.exit(0);
    }
}
