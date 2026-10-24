// The order of the notices' stack, run in Qt's own JavaScript engine, the
// one the shell runs it in. The `the-notices-keep-their-order` check copies
// modules/quickshell/Stack.js beside this file and runs
// `qml -platform offscreen stack.qml`.
import QtQuick
import "Stack.js" as Stack

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
        function holds(what, got, want) {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        }

        // [what, the order, the live ids, the stack]
        const showing = [
            ["the stack keeps its order", [3, 1, 2], [1, 2, 3], [3, 1, 2]],
            ["one gone leaves the stack", [3, 1, 2], [1, 3], [3, 1]],
            ["one never heard arrive joins the back", [2], [1, 2], [2, 1]],
            ["nothing live is an empty stack", [1, 2], [], []]
        ];
        for (const [what, order, live, want] of showing)
            holds(what, Stack.shown(order, live), want);

        holds("a plain one waits behind the one before", Stack.arrived([1, 2], 3, false), [1, 2, 3]);
        holds("a critical one goes to the front", Stack.arrived([1, 2], 3, true), [3, 1, 2]);
        holds("the first to arrive is the stack", Stack.arrived([], 1, false), [1]);

        holds("turning brings the next to the front", Stack.turned([1, 2, 3], 1), [2, 3, 1]);
        holds("turning back brings the last", Stack.turned([1, 2, 3], -1), [3, 1, 2]);
        holds("a stack of one doesn't turn", Stack.turned([1], 1), [1]);
        holds("an empty stack doesn't turn", Stack.turned([], -1), []);

        holds("one brought forward leads the rest", Stack.forward([1, 2, 3], 3), [3, 1, 2]);
        holds("the front brought forward stays", Stack.forward([1, 2, 3], 1), [1, 2, 3]);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
