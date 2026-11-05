// The launcher's field's keys in its two modes, read from their one
// table in Qt's own JavaScript engine, the one the shell runs it in. The
// `the-query-field-takes-vi-modes` check copies
// modules/quickshell/QueryKeys.js, Editing.js and ListedKeys.js beside this
// file and runs `qml -platform offscreen query-keys.qml`.
import QtQuick
import "Editing.js" as Editing
import "ListedKeys.js" as Listed
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

    // The keys typed from a line's opening in insert, on a prompt's words.
    function wordsTyped(keys) {
        let standing = Keys.rest();
        let mode = "insert";
        const acts = [];
        for (const key of keys) {
            const read = Keys.read(standing, key, mode, true);
            standing = read.standing;
            mode = Keys.modeAfter(read.act, mode);
            if (read.act !== "")
                acts.push(read.count > 0 ? `${read.count} ${read.act}` : read.act);
        }
        return { acts, mode };
    }

    // A prompt's words edited in normal, as Vim edits a line, and Ctrl+H in
    // insert, on every line.
    function editing(holds) {
        // [what, the key, ms since it was asked, the answer]
        const answers = [
            ["y answers yes", "y", 400, "yes"],
            ["n answers no", "n", 400, "no"],
            ["Escape answers no", "Escape", 400, "no"],
            ["Ctrl+C answers no", "Ctrl+c", 400, "no"],
            ["any other key does nothing", "Y", 400, ""],
            ["Return does nothing", "Return", 400, ""],
            ["a key on its way as it's asked does nothing", "y", 120, ""]
        ];
        for (const [what, key, waited, want] of answers)
            holds(what, Editing.onceAnswered(key, waited, 300), want);
        // [what, the keys typed from the opening, the acts done]
        const doing = [
            ["w b e move by words", ["Escape", "w", "b", "e"],
                ["normal", "wordNext", "wordBack", "wordEnd"]],
            ["h l and the arrows move by letters", ["Escape", "h", "l", "Left", "Right"],
                ["normal", "letterBack", "letterNext", "letterBack", "letterNext"]],
            ["- and Backspace still step out", ["Escape", "-", "Backspace"],
                ["normal", "out", "out"]],
            ["0 and $ go to the line's ends", ["Escape", "$", "0"],
                ["normal", "lineEnd", "lineStart"]],
            ["x, dw and cw change the words, cw into insert", ["Escape", "x", "d", "w", "c", "w"],
                ["normal", "deleteLetter", "deleteWordOn", "changeWord"]],
            ["cc still clears the line", ["Escape", "c", "c"], ["normal", "change"]],
            ["u undoes", ["Escape", "u"], ["normal", "undo"]],
            ["a count goes before a word's move", ["Escape", "2", "w"], ["normal", "2 wordNext"]],
            ["j still goes down to the rows", ["Escape", "j"], ["normal", "next"]]
        ];
        for (const [what, keys, want] of doing)
            holds(`on a prompt's words, ${what}`, wordsTyped(keys).acts, want);
        holds("cw leaves the line in insert", wordsTyped(["Escape", "c", "w"]).mode, "insert");
        holds("on a find, u brings back what was cleared", typed(["Escape", "u"]).acts,
            ["normal", "restore"]);
        holds("and w and x do nothing", typed(["Escape", "w", "x"]).acts, ["normal"]);
        holds("Ctrl+H deletes back in insert", typed(["Ctrl+h"]).acts, ["deleteBack"]);
        for (const act of ["wordNext", "wordBack", "wordEnd"])
            holds(`${act} repeats while held`, Keys.repeats(act), true);
        holds("x is once a press", Keys.repeats("deleteLetter"), false);

        // [what, act, words, caret, count, { text, at } after]
        const edits = [
            ["h goes a letter back", "letterBack", "notes.md", 5, 0, 4],
            ["h at the start stays", "letterBack", "notes.md", 0, 0, 0],
            ["l goes a letter on", "letterNext", "notes.md", 4, 0, 5],
            ["l on the last letter stays", "letterNext", "notes.md", 7, 0, 7],
            ["3l goes three on", "letterNext", "notes.md", 0, 3, 3],
            ["w goes past a word to the next run", "wordNext", "notes.md", 0, 0, 5],
            ["w goes past the dot to the extension", "wordNext", "notes.md", 5, 0, 6],
            ["w goes past spaces", "wordNext", "my notes", 0, 0, 3],
            ["w on the last word rests on the last letter", "wordNext", "notes", 1, 0, 4],
            ["2w goes two words on", "wordNext", "my big notes", 0, 2, 7],
            ["b goes to the word's start", "wordBack", "notes.md", 3, 0, 0],
            ["b at a word's start goes to the one before", "wordBack", "notes.md", 6, 0, 5],
            ["e goes to the word's end", "wordEnd", "notes.md", 0, 0, 4],
            ["e at a word's end goes to the next one's", "wordEnd", "my notes", 1, 0, 7],
            ["0 goes to the start", "lineStart", "notes.md", 5, 0, 0],
            ["$ goes to the last letter", "lineEnd", "notes.md", 0, 0, 7]
        ];
        for (const [what, act, text, at, count, want] of edits)
            holds(what, Editing.edited(act, text, at, count), { text, at: want });
        // [what, act, words, caret, count, words after, caret after]
        const changes = [
            ["x takes the letter under the caret", "deleteLetter", "notes.md", 5, 0, "notesmd", 5],
            ["3x takes three", "deleteLetter", "notes.md", 0, 3, "es.md", 0],
            ["x on the last letter rests on the one before", "deleteLetter", "ab", 1, 0, "a", 0],
            ["dw takes the word and the space after it", "deleteWordOn", "my notes", 0, 0, "notes",
                0],
            ["dw on the last word takes it to the end", "deleteWordOn", "my notes", 3, 0, "my ", 2],
            ["cw takes the word but not the space after it", "changeWord", "my notes", 0, 0,
                " notes", 0],
            ["cw takes the name before the extension", "changeWord", "notes.md", 0, 0, ".md", 0],
            ["c2w takes two words and the space between", "changeWord", "my big notes", 0, 2,
                " notes", 0]
        ];
        for (const [what, act, text, at, count, after, caret] of changes)
            holds(what, Editing.edited(act, text, at, count), { text: after, at: caret });
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
            ["normal's Backspace and - step out, as his oil's do", ["Escape", "Backspace", "-"],
                ["normal", "out", "out"]],
            ["q and ZZ close", ["Escape", "q", "Z", "Z"], ["normal", "close", "close"]],
            ["a Z with anything but Z after it lets go", ["Escape", "Z", "j", "Z", "Z"],
                ["normal", "close"]],
            [": in normal opens the command line", ["Escape", ":"], ["normal", "command"]],
            ["a colon in insert is the field's to type", [":", "q", "Return"], ["run"]],
            ["normal's rows keep the arrows", ["Escape", "Down", "Up"],
                ["normal", "next", "previous"]],
            ["normal's Tab and Space Tab go to the place used last",
                ["Escape", "Tab", "Space", "Tab"], ["normal", "lastPlace", "lastPlace"]],
            ["Space ? and F1 show every key here", ["Escape", "Space", "?", "F1"],
                ["normal", "keys", "keys"]],
            ["Space w closes, as his leader closes a split", ["Escape", "Space", "w"],
                ["normal", "close"]],
            ["Space alone begins, and a key it doesn't lead lets it go",
                ["Escape", "Space", "j", "j"], ["normal", "next"]],
            ["Escape cancels a Space waiting, then steps back",
                ["Escape", "Space", "Escape", "Escape"], ["normal", "back"]],
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
            ["gx opens the row's file in its own program", ["Escape", "g", "x"],
                ["normal", "openOwn"]],
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
                ["normal", "next"]],
            ["a count before a colon goes with it", ["Escape", "3", ":"],
                ["normal", "3 command"]]
        ];
        for (const [what, keys, want] of counting)
            holds(what, typed(keys).counted, want);
        holds("a digit in insert is the field's to type", typed(["3"]).ours, false);
        holds("and counts nothing", typed(["3", "Escape", "j"]).counted, ["normal", "next"]);
        holds("a count typed is the line's", typed(["Escape", "4"]).ours, true);
        holds("and stands until its move", typed(["Escape", "4", "2"]).standing,
            { count: "42", typed: [] });
        holds("a count before a colon is the range Vim's 3: gives",
            [Keys.counted(0), Keys.counted(1), Keys.counted(3)], ["", ".", ".,.+2"]);
        holds("and the keys stand at rest once it opens",
            typed(["Escape", "3", ":"]).standing, { count: "", typed: [] });
        holds("insert's Space is the field's to type", typed(["Space"]).ours, false);
        // The rows have no search of their own: the find line is the list's
        // one way to find (VKCF3Z's vi-paradigm spec, The Find Line).
        holds("/ ? n N search nothing in normal", typed(["Escape", "/", "?", "n", "N"]).acts,
            ["normal"]);
        holds("and leave it in normal", typed(["Escape", "/", "?"]).mode, "normal");
        holds("a Space waiting its timeoutlen lets go", Keys.waited({ count: "", typed: ["Space"] }),
            { count: "", typed: [] });
        // Space ? lists the keys by what each line says, its keys as the
        // desktop draws them, Vim's own where an act has them, and its arrow.
        const lines = { rows: "rows", screen: "screen", inOut: "in and out", run: "run",
            lastPlace: "last", keys: "keys", close: "close", drawer: "drawer", ends: "ends",
            nextRow: "next row", caret: "caret", word: "word" };
        const modes = { normal: "Normal", insert: "Insert" };
        const list = Listed.keyList(Keys.table, lines, modes);
        const keyed = value => (list.find(line => line.value === value) ?? {}).label;
        holds("the list's rows, the arrows after Vim's", keyed("rows"),
            "j  k  \u2193  \u2191");
        holds("the list's ends, a sequence as Vim writes it", keyed("ends"), "gg  G");
        holds("the list's screens, Ctrl's before the named keys", keyed("screen"),
            "Ctrl+F  Ctrl+B");
        holds("the list's in and out, - among them", keyed("in and out"),
            "l  h  -  \u2192  \u2190");
        holds("the list's Return, once", keyed("run"), "\u21b5");
        holds("the list's place used last", keyed("last"), "Tab  Space Tab");
        holds("the list's own keys", keyed("keys"), "Space ?  F1");
        holds("the list's closing, :q among them and q not", keyed("close"), "ZZ  :q");
        holds("the list's drawer", keyed("drawer"), "\u21e7+\u2193  \u21e7+\u2191");
        // Each mode's section, by its heading: the keys each line names, as
        // a list, two spaces apart, so \u21e7+\u2193 is never read as \u2193.
        const sections = {};
        let heading = "";
        list.forEach(line => {
            if (line.heading !== undefined) {
                heading = line.heading;
                sections[heading] = [];
            } else {
                sections[heading] = sections[heading].concat(line.label.split("  "));
            }
        });
        holds("the list's sections, normal's where it's opened first",
            list.filter(line => line.heading !== undefined).map(line => line.heading),
            ["Normal", "Insert"]);
        holds("insert's next row, Ctrl+N with the arrow", keyed("next row"), "Ctrl+N  \u2193");
        holds("insert's caret", keyed("caret"), "\u2190  \u2192");
        holds("insert's word deleted", keyed("word"), "Ctrl+W");
        holds("normal's keys aren't listed in insert, where they type",
            ["j", "k", "gg", "ZZ", "Tab"].filter(key => sections.Insert.includes(key)),
            []);
        holds("insert's keys aren't listed in normal, where they do nothing",
            ["Ctrl+W", "Ctrl+N", "Ctrl+Y", "Home"].filter(key => sections.Normal.includes(key)),
            []);
        const facts = list.filter(line => line.heading === undefined);
        holds("every line the list gives has its keys",
            facts.filter(line => line.label === "").length, 0);
        // Joel presses the arrows, so every act one does in a mode lists it
        // in that mode's section. A prompt's words keep their own keys,
        // which the launcher's list doesn't name.
        const arrows = ["Up", "Down", "Left", "Right"];
        holds("every act an arrow does lists its arrow", Keys.table
            .filter(one => one.mode !== "words")
            .map(one => ({ in: one.mode === "both" ? ["Normal", "Insert"] : [modes[one.mode]],
                arrows: one.keys.filter(key => arrows.includes(key)) }))
            .filter(one => one.arrows.some(key => one.in.some(mode =>
                !sections[mode].includes(Listed.face(key)))))
            .map(one => one.arrows), []);
        // The narrowest plate's names sit whole at 16 figures; 14 leaves a
        // glyph's room.
        holds("every line's keys are short enough to sit whole",
            facts.filter(line => line.label.length > 14).map(line => line.label), []);
        holds("a g waiting its timeoutlen lets go, keeping the count",
            Keys.waited({ count: "3", typed: ["g"] }), { count: "3", typed: [] });
        for (const act of ["next", "previous", "first", "last", "viewTop", "viewMiddle",
            "viewBottom", "halfDown", "halfUp", "pageDown", "pageUp"])
            holds(`${act} repeats while held`, Keys.repeats(act), true);
        for (const act of ["run", "into", "out", "back", "close", "lastPlace", "keys",
            "insert", "change", "restore", ""])
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

        editing(holds);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
