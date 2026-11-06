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

        // The view scrolled as Vim's is, the cursor kept inside it by
        // scrolloff (Joel, 2026-11-05: "I can't scroll up and down in a
        // notices list now ... that is very vi like"): Ctrl+E and Ctrl+Y a
        // line, the wheel three, zz zt zb about the cursor. Twenty lines,
        // eight shown, three kept from the edge.
        const twenty = { count: 20, top: 0, shown: 8 };
        const scrolling = [
            ["Ctrl+E scrolls a line, the cursor kept off the top", "lineDown", 0, 0,
                twenty, { top: 1, cursor: 4 }],
            ["3 Ctrl+E three, the cursor where it was", "lineDown", 6, 3, twenty,
                { top: 3, cursor: 6 }],
            ["never past the last", "lineDown", 0, 30, twenty, { top: 12, cursor: 15 }],
            ["Ctrl+Y back a line, the cursor kept off the foot", "lineUp", 19, 0,
                { count: 20, top: 12, shown: 8 }, { top: 11, cursor: 15 }],
            ["and none at the top", "lineUp", 2, 0, twenty, { top: 0, cursor: 2 }],
            ["zz puts the cursor in the middle", "viewCentre", 9, 0, twenty,
                { top: 6, cursor: 9 }],
            ["zt at the top, short of scrolloff", "viewAtTop", 10, 0, twenty,
                { top: 7, cursor: 10 }],
            ["zb at the foot", "viewAtBottom", 10, 0, twenty, { top: 6, cursor: 10 }],
            ["a count names the line", "viewAtTop", 2, 11, twenty, { top: 7, cursor: 10 }],
            ["a list that fits never scrolls", "lineDown", 1, 0,
                { count: 5, top: 0, shown: 8 }, { top: 0, cursor: 1 }]
        ];
        for (const [what, motion, at, count, list, want] of scrolling)
            holds(what, List.scrolled(motion, at, count, list), want);

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

        // A card's words on a short screen: the whole lines that fit, ending
        // at a whole word with an ellipsis (VKCF3Z, 2026-11-05), a run's
        // width here its length.
        const measure = run => run.length;
        holds("words broken at their spaces", List.laid(["alpha", "beta", "gamma", "delta"],
            12, measure), 2);
        holds("a word too wide alone takes its lines", List.laid(["abcdefghijklmnopqrstuvwxy"],
            10, measure), 3);
        holds("words that fit stay as they are", List.wholeWords("alpha beta", 1, 12, measure),
            null);
        holds("else they end at a whole word",
            List.wholeWords("alpha beta gamma delta", 1, 12, measure), "alpha beta\u2026");
        holds("over the lines they have",
            List.wholeWords("alpha beta gamma delta epsilon", 2, 12, measure),
            "alpha beta gamma delta\u2026");
        holds("and a first word too wide is kept for Qt to cut",
            List.wholeWords("abcdefghijklmno pq", 1, 10, measure), "abcdefghijklmno\u2026");

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
