// The launcher's field's keys in its two modes, read from their one
// table in Qt's own JavaScript engine, the one the shell runs it in. The
// `the-query-field-takes-vi-modes` check copies
// modules/quickshell/QueryKeys.js beside this file and runs
// `qml -platform offscreen query-keys.qml`.
import QtQuick
import "QueryKeys.js" as Keys

QtObject {
    // What failed, said as the test ends, or as it throws.
    property var failures: []

    // A test that throws ends as a failure, rather than leaving qml running,
    // saying what had failed before it.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            for (const failure of failures)
                console.warn(`FAIL ${failure}`);
            console.warn(`the test threw: ${error}`);
            Qt.exit(1);
        }
    }

    // What the keys do, typed one after another from the line's opening, in
    // insert: each act done, the mode it's left in, and whether the last key
    // was the line's rather than the field's to type.
    // Each act done is said with the count typed before it, as "3 next",
    // in `counted`.
    function typed(keys) {
        let standing = Keys.rest();
        let mode = "insert";
        const acts = [];
        const counted = [];
        let ours = false;
        for (const key of keys) {
            const read = Keys.read(standing, key, mode);
            standing = read.standing;
            ours = read.ours;
            mode = Keys.modeAfter(read.act, mode);
            if (read.act !== "") {
                acts.push(read.act);
                counted.push(read.count > 0 ? `${read.count} ${read.act}` : read.act);
            }
        }
        return { acts, counted, ours, mode, standing };
    }

    function run() {
        function holds(what, got, want) {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        }

        // [what, the keys typed from the opening, the acts done]
        const doing = [
            ["insert's arrows, Home and End move the caret", ["Left", "Right", "Home", "End"],
                ["caretLeft", "caretRight", "caretStart", "caretEnd"]],
            ["insert's Backspace, Ctrl+W and Ctrl+U delete", ["Backspace", "Ctrl+w", "Ctrl+u"],
                ["deleteBack", "deleteWord", "deleteToStart"]],
            ["insert's next row, as his completion menu's", ["Down", "Tab", "Ctrl+n"],
                ["next", "next", "next"]],
            ["insert's row before", ["Up", "Shift+Tab", "Ctrl+p"],
                ["previous", "previous", "previous"]],
            ["Ctrl+Y completes", ["Ctrl+y"], ["complete"]],
            ["Return does the row in insert, and in normal", ["Return", "Escape", "Return"],
                ["run", "normal", "run"]],
            ["Escape gives normal, and then steps back", ["Escape", "Escape"], ["normal", "back"]],
            ["normal's arrows and h l step out and in", ["Escape", "Left", "h", "Right", "l"],
                ["normal", "out", "out", "into", "into"]],
            ["normal's Backspace steps out", ["Escape", "Backspace"], ["normal", "out"]],
            ["normal's rows keep the arrows and Tab", ["Escape", "Down", "Tab", "Up", "Shift+Tab"],
                ["normal", "next", "next", "previous", "previous"]],
            ["normal's j and k go down and up", ["Escape", "j", "k"],
                ["normal", "next", "previous"]],
            ["gg and G go to the first and the last", ["Escape", "g", "g", "G"],
                ["normal", "first", "last"]],
            ["H M L go to the view's top, middle and bottom", ["Escape", "H", "M", "L"],
                ["normal", "viewTop", "viewMiddle", "viewBottom"]],
            ["Ctrl+D and Ctrl+U go half a screen", ["Escape", "Ctrl+d", "Ctrl+u"],
                ["normal", "halfDown", "halfUp"]],
            ["Ctrl+F and Ctrl+B go a screen, as Page Down and Up",
                ["Escape", "Ctrl+f", "Ctrl+b", "PageDown", "PageUp"],
                ["normal", "pageDown", "pageUp", "pageDown", "pageUp"]],
            ["insert's Ctrl+U still deletes to the start", ["Ctrl+u"], ["deleteToStart"]],
            ["a g with anything but g after it lets go", ["Escape", "g", "j", "g", "g"],
                ["normal", "first"]],
            ["u brings the query back", ["Escape", "u"], ["normal", "restore"]],
            ["i a I A go into insert", ["Escape", "i", "Escape", "a", "Escape", "I", "Escape", "A"],
                ["normal", "insert", "normal", "append", "normal", "insertStart", "normal",
                    "appendEnd"]],
            ["cc and S clear the line into insert", ["Escape", "c", "c", "Escape", "S"],
                ["normal", "change", "normal", "change"]],
            ["]b and [b go to the place beside, the next and the one before",
                ["Escape", "]", "b", "[", "b"], ["normal", "nextFile", "previousFile"]],
            ["Escape cancels a c waiting for its second", ["Escape", "c", "Escape", "c"],
                ["normal"]],
            ["a key that ends nothing lets c go", ["Escape", "c", "x", "c"], ["normal"]],
            ["Shift pressed for a capital changes nothing", ["Escape", "c", "", "c"],
                ["normal", "change"]],
            ["the arrows with Shift scroll the drawer in either mode",
                ["Shift+Down", "Escape", "Shift+Up"], ["drawerDown", "normal", "drawerUp"]],
            ["Page Up and Down page the rows in either mode",
                ["PageDown", "Escape", "PageUp"], ["pageDown", "normal", "pageUp"]],
            ["Return with Shift or Ctrl is Return", ["Escape", "Shift+Return", "Ctrl+Return"],
                ["normal", "run", "run"]]
        ];
        for (const [what, keys, want] of doing)
            holds(what, typed(keys).acts, want);

        // [what, the keys typed from the opening, each act with its count]
        const counting = [
            ["a count moves that many", ["Escape", "3", "j", "2", "k"],
                ["normal", "3 next", "2 previous"]],
            ["a count of several digits, 0 among them", ["Escape", "1", "0", "j"],
                ["normal", "10 next"]],
            ["a count before G and gg names the row", ["Escape", "1", "2", "G", "4", "g", "g"],
                ["normal", "12 last", "4 first"]],
            ["a count before H, L and the screens",
                ["Escape", "2", "H", "3", "L", "2", "Ctrl+d", "2", "Ctrl+f"],
                ["normal", "2 viewTop", "3 viewBottom", "2 halfDown", "2 pageDown"]],
            ["0 alone is no count, and does nothing", ["Escape", "0", "j"], ["normal", "next"]],
            ["a count goes with its move, not the next", ["Escape", "3", "j", "j"],
                ["normal", "3 next", "next"]],
            ["Escape only cancels a count, then steps back", ["Escape", "5", "Escape", "j", "Escape"],
                ["normal", "next", "back"]],
            ["Escape only cancels a g waiting, then steps back", ["Escape", "g", "Escape", "Escape"],
                ["normal", "back"]],
            ["a key that ends nothing lets the count go", ["Escape", "3", "x", "j"],
                ["normal", "next"]]
        ];
        for (const [what, keys, want] of counting)
            holds(what, typed(keys).counted, want);
        holds("a digit in insert is the field's to type", typed(["3"]).ours, false);
        holds("and counts nothing", typed(["3", "Escape", "j"]).counted, ["normal", "next"]);
        holds("a count typed is the line's", typed(["Escape", "4"]).ours, true);
        holds("and stands until its move", typed(["Escape", "4", "2"]).standing,
            { count: "42", typed: [] });
        holds("a g waiting its timeoutlen lets go, keeping the count",
            Keys.waited({ count: "3", typed: ["g"] }), { count: "3", typed: [] });
        for (const act of ["next", "previous", "first", "last", "viewTop", "viewMiddle",
            "viewBottom", "halfDown", "halfUp", "pageDown", "pageUp"])
            holds(`${act} repeats while held`, Keys.repeats(act), true);
        for (const act of ["run", "into", "out", "back", "insert", "change", "restore", ""])
            holds(`${act || "a count"} is once a press`, Keys.repeats(act), false);

        // Twenty rows, the 5th to the 12th wholly in view, a screen of 8,
        // the 7th selected: [what, the act, the count, where it goes].
        const place = { at: 6, highest: 0, last: 19, top: 4, bottom: 11, screen: 8 };
        const moves = [
            ["j", "next", 0, 7],
            ["3j", "next", 3, 9],
            ["a count past the last, to the last", "next", 30, 19],
            ["k", "previous", 0, 5],
            ["a count past the first, to the first", "previous", 30, 0],
            ["gg", "first", 0, 0],
            ["5gg", "first", 5, 4],
            ["G", "last", 0, 19],
            ["12G", "last", 12, 11],
            ["a row past the last, the last", "last", 99, 19],
            ["H", "viewTop", 0, 4],
            ["3H, the third from the top", "viewTop", 3, 6],
            ["a count past the view, its bottom", "viewTop", 20, 11],
            ["M", "viewMiddle", 0, 7],
            ["L", "viewBottom", 0, 11],
            ["2L, the second from the bottom", "viewBottom", 2, 10],
            ["Ctrl+D, half the rows in view", "halfDown", 0, 10],
            ["Ctrl+U", "halfUp", 0, 2],
            ["2 Ctrl+D, two rows", "halfDown", 2, 8],
            ["Ctrl+F", "pageDown", 0, 14],
            ["2 Ctrl+F, to the last", "pageDown", 2, 19],
            ["Ctrl+B, to the first", "pageUp", 0, 0]
        ];
        for (const [what, act, count, want] of moves)
            holds(`${what} goes to the row`, Keys.moved(act, count, place), want);
        holds("k goes up to an answer's headline",
            Keys.moved("previous", 0, Object.assign({}, place, { at: 0, highest: -1 })), -1);
        holds("gg goes to an answer's headline",
            Keys.moved("first", 0, Object.assign({}, place, { highest: -1 })), -1);
        holds("j with no rows stays where it is",
            Keys.moved("next", 0, { at: 0, highest: 0, last: -1, top: 0, bottom: 0, screen: 8 }),
            0);
        holds("M in a view of three rows, the second",
            Keys.moved("viewMiddle", 0, Object.assign({}, place, { top: 0, bottom: 2, last: 2 })), 1);

        holds("the line opens in insert", typed([]).mode, "insert");
        holds("a letter in insert is the field's to type", typed(["x"]).ours, false);
        holds("a letter in insert types, never acts", typed(["i", "a", "c", "c", "u"]).acts, []);
        holds("a ] in insert is the field's to type", typed(["]"]).ours, false);
        holds("a letter in normal is the line's, typing nothing", typed(["Escape", "x"]).ours,
            true);
        holds("Escape leaves insert for normal", typed(["Escape"]).mode, "normal");
        holds("cc leaves normal for insert", typed(["Escape", "c", "c"]).mode, "insert");
        holds("a c begun is the line's", typed(["Escape", "c"]).ours, true);
        holds("nothing begun is left after an act", typed(["Escape", "c", "c"]).standing,
            Keys.rest());
        holds("the field says Ctrl+Y for completing", Keys.keyOf("complete", "insert"), "Ctrl+y");
        holds("and the left arrow for clearing", Keys.keyOf("out", "normal"), "Left");

        // [what, the text, the caret, where Ctrl+W deletes back to]
        const words = [
            ["a word", "foo bar", 7, 4],
            ["a word and the spaces after it", "foo bar  ", 9, 4],
            ["a word's part before the caret", "foo bar", 6, 4],
            ["a run of what isn't a word's", "foo.bar", 4, 3],
            ["a word after a stop", "foo.bar", 7, 4],
            ["the line's first word", "foo", 3, 0],
            ["nothing before the caret", "foo", 0, 0],
            ["an accented word whole", "caf\u00e9 cr\u00e8me", 10, 5]
        ];
        for (const [what, text, at, want] of words)
            holds(`Ctrl+W takes ${what}`, Keys.wordStart(text, at), want);

        // [what, the act, the text, the caret, where it goes]
        const carets = [
            ["leaving insert, one left", "normal", "sound", 5, 4],
            ["leaving insert at the start, staying", "normal", "sound", 0, 0],
            ["i, at it", "insert", "sound", 2, 2],
            ["a, after it", "append", "sound", 4, 5],
            ["a on an empty line, staying", "append", "", 0, 0],
            ["I, at the first letter", "insertStart", "  sound", 5, 2],
            ["A, at the end", "appendEnd", "sound", 1, 5]
        ];
        for (const [what, act, text, at, want] of carets)
            holds(`the caret ${what}`, Keys.caretAfter(act, text, at), want);
        holds("normal's caret rests on the last letter", Keys.resting("sound", 5), 4);
        holds("and on an empty line at its start", Keys.resting("", 0), 0);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
