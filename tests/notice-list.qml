// The held notices as a quickfix list, worked out in Qt's own JavaScript
// engine, the one the shell runs it in. The `the-notices-are-a-quickfix-list`
// check copies modules/quickshell/NoticeList.js beside this file and runs
// `qml -platform offscreen notice-list.qml`.
import QtQuick
import "NoticeList.js" as List

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

        holds("the newest is line 1", List.ordered([1, 2, 3], { 1: 10, 2: 30, 3: 20 }),
            [2, 3, 1]);

        holds("a list that fits never scrolls", List.viewTop(0, 5, 6), 0);
        holds("the view keeps three from the cursor, half of eight", List.viewTop(0, 5, 20), 1);
        holds("and three above it going back", List.viewTop(10, 12, 20), 9);
        holds("never past the last", List.viewTop(0, 19, 20), 12);
        holds("a short view keeps half itself", List.viewTop(0, 2, 20, 3), 1);

        holds("the cursor line is its own number", List.number(3, 3, true), "4");
        holds("the rest how far from it", List.number(1, 3, true), "2");
        holds("absolute, each its own", List.number(1, 3, false), "2");

        // Ten lines, in view from the first, from senders a a b b b c ...
        const senders = ["a", "a", "b", "b", "b", "c", "c", "d", "d", "d"];
        const list = { count: 10, top: 0, senders };
        const moving = [
            ["j", "down", 0, 0, 1],
            ["3j", "down", 0, 3, 3],
            ["j stops at the last", "down", 9, 0, 9],
            ["k stops at the first", "up", 0, 0, 0],
            ["gg", "first", 5, 0, 0],
            ["4gg", "first", 0, 4, 3],
            ["G", "last", 0, 0, 9],
            ["2G", "last", 0, 2, 1],
            ["H", "viewTop", 5, 0, 0],
            ["M", "viewMiddle", 0, 0, 3],
            ["L", "viewBottom", 0, 0, 7],
            ["} to the next sender", "runOn", 0, 0, 2],
            ["2} two on", "runOn", 0, 2, 5],
            ["{ to the run's start", "runBack", 3, 0, 2],
            ["{ from a start to the one before", "runBack", 2, 0, 0],
            ["Ctrl+D half a view", "halfDown", 0, 0, 4],
            ["Ctrl+F a view", "viewDown", 0, 0, 8]
        ];
        for (const [what, motion, at, count, want] of moving)
            holds(what, List.moved(motion, at, count, list), want);

        holds("dd takes the cursor line", List.taken("line", 2, 0, list), { from: 2, to: 2 });
        holds("3dd three from it", List.taken("line", 2, 3, list), { from: 2, to: 4 });
        holds("dd past the end stops there", List.taken("line", 8, 5, list), { from: 8, to: 9 });
        holds("dk takes it and the one above", List.taken("up", 2, 0, list), { from: 1, to: 2 });
        holds("dG to the last", List.taken("last", 7, 0, list), { from: 7, to: 9 });
        holds("dip the sender's run", List.taken("inRun", 3, 0, list), { from: 2, to: 4 });

        holds("the selection either way", [List.chosen(2, 5), List.chosen(5, 2)],
            [{ from: 2, to: 5 }, { from: 2, to: 5 }]);
        holds("the ids between", List.between([7, 8, 9, 10], { from: 1, to: 2 }), [8, 9]);

        let history = List.changes();
        history = List.changed(history, { ids: [1, 2], kind: "done" });
        history = List.changed(history, { ids: [3], kind: "forget" });
        holds("away is every change's cards", List.away(history.done), [1, 2, 3]);
        const undone = List.undo(history, 0);
        holds("u takes back the last", undone.moved, [{ ids: [3], kind: "forget" }]);
        holds("leaving the one before", List.away(undone.history.done), [1, 2]);
        const redone = List.redo(undone.history, 0);
        holds("Ctrl+R does it again", List.away(redone.history.done), [1, 2, 3]);
        holds("2u takes back two", List.away(List.undo(history, 2).history.done), []);
        holds("u with nothing done does nothing", List.undo(List.changes(), 0).moved, null);
        holds("a new change forgets what was taken back",
            List.changed(undone.history, { ids: [4], kind: "done" }).undone, []);

        holds("Vim's help notation", List.notation("forget", "forg"), "forg[et]");
        holds("a command with no shorter form", List.notation("open", "open"), "open");

        const entries = [
            { command: "open", short: "ope", name: "Open", key: "\u21b5" },
            { command: "delete", short: "d", name: "Done", key: "dd" },
            { command: "forget", short: "forg", name: "Forget It", key: "" },
            { command: "yank", short: "y", name: "Copy What It Said", key: "yy" }
        ];
        const names = text => List.menu(entries, text).shown.map(one => one.command);
        holds("nothing typed lists them all", names(""), ["open", "delete", "forget", "yank"]);
        holds("narrowed by the command's start", names("fo"), ["forget"]);
        holds("then by the act's words", names("copy"), ["yank"]);
        holds("a range typed first is kept", List.menu(entries, "'<,'>d").prefix, "'<,'>");
        holds("and the rest narrows", names("'<,'>d"), ["delete"]);
        holds("% too", List.menu(entries, "%y").prefix, "%");

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
