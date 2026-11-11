// How the launcher keeps the rows a daemon serves ready in their order,
// run in Qt's own JavaScript engine, the one the shell runs it in. The
// `the-launcher-places-each-row-it-is-served` check copies
// launcher/Placed.js and tests/lib/Expect.js beside this file and runs
// `qml -platform offscreen placed.qml` over rows made up here, as wifid
// serves them.
import QtQuick
import "launcher/Placed.js" as Placed
import "Expect.js" as Expect

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`${error}`);
        }
    }

    function run() {
        const held = {
            a: { order: 1024 },
            b: { order: 2048 },
            c: { order: 3072 }
        };
        let keys = Placed.placed([], held, ["c", "a", "b"], []);
        Expect.same("a whole answer is placed by order, whatever order it came in",
            keys, ["a", "b", "c"]);
        held.c = { order: 1536 };
        const before = keys;
        keys = Placed.placed(keys, held, ["c"], []);
        Expect.same("a row moved midway between two goes between them", keys, ["a", "c", "b"]);
        Expect.same("and the list before is left as it was", before, ["a", "b", "c"]);
        held.d = { order: 0 };
        delete held.b;
        keys = Placed.placed(keys, held, ["d"], ["b"]);
        Expect.same("a new row takes its place and a row gone is let go", keys,
            ["d", "a", "c"]);
        keys = Placed.placed(keys, held, [], ["z"]);
        Expect.same("a key never held changes nothing", keys, ["d", "a", "c"]);
        held.a = { order: 4096 };
        held.c = { order: 8192 };
        held.d = { order: 2048 };
        keys = Placed.placed(keys, held, ["a", "c", "d"], []);
        Expect.same("every row renumbered at once is placed once", keys, ["d", "a", "c"]);
    }
}
