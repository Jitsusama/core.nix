// The launcher's field's keys in its two modes, read from their one
// table in Qt's own JavaScript engine, the one the shell runs it in. The
// `the-query-field-takes-vi-modes` check copies
// modules/quickshell/QueryKeys.js beside this file and runs
// `qml -platform offscreen query-keys.qml`.
import QtQuick
import "QueryKeys.js" as Keys

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

    // What the keys do, typed one after another from the line's opening, in
    // insert: each act done, the mode it's left in, and whether the last key
    // was the line's rather than the field's to type.
    function typed(keys) {
        let standing = Keys.rest();
        let mode = "insert";
        const acts = [];
        let ours = false;
        for (const key of keys) {
            const read = Keys.read(standing, key, mode);
            standing = read.standing;
            ours = read.ours;
            mode = Keys.modeAfter(read.act, mode);
            if (read.act !== "")
                acts.push(read.act);
        }
        return { acts, ours, mode, standing };
    }

    function run() {
        const failures = [];
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
            ["u brings the query back", ["Escape", "u"], ["normal", "restore"]],
            ["i a I A go into insert", ["Escape", "i", "Escape", "a", "Escape", "I", "Escape", "A"],
                ["normal", "insert", "normal", "append", "normal", "insertStart", "normal",
                    "appendEnd"]],
            ["cc and S clear the line into insert", ["Escape", "c", "c", "Escape", "S"],
                ["normal", "change", "normal", "change"]],
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

        holds("the line opens in insert", typed([]).mode, "insert");
        holds("a letter in insert is the field's to type", typed(["x"]).ours, false);
        holds("a letter in insert types, never acts", typed(["i", "a", "c", "c", "u"]).acts, []);
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
