// The launcher's field's keys in its two modes, read from their one
// table in Qt's own JavaScript engine, the one the shell runs it in. The
// `the-query-field-takes-vi-modes` check copies
// modules/quickshell/QueryKeys.js beside this file and runs
// `qml -platform offscreen query-keys.qml`.
import QtQuick
import "QueryKeys.js" as Keys
import "Search.js" as Search

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

    // A search typed on the line: what the keys typed after the line's
    // opening leave it as, { pattern, at, ends }, each key's letters being
    // the key itself where it's one character, or a space for Space.
    function searchTyped(keys) {
        let standing = Keys.rest();
        let mode = "insert";
        let line = { pattern: "", at: 0, ends: "" };
        for (const key of keys) {
            const read = Keys.read(standing, key, mode);
            standing = read.standing;
            if (mode === "search")
                line = Keys.typedInSearch(line, read.act,
                    key === "Space" ? " " : key.length === 1 ? key : "");
            mode = Keys.modeAfter(read.act, mode);
        }
        return line;
    }

    // / ? n N, the search's line, and where Search.js lands a search.
    function searching(holds) {
        // [what, the keys typed from the opening, the acts done]
        const doing = [
            ["/ and ? begin a search, and Return runs it",
                ["Escape", "/", "f", "Return", "?", "Return"],
                ["normal", "searchForward", "find", "searchBack", "find"]],
            ["Escape cancels a search, and only it, then steps back",
                ["Escape", "/", "Escape", "Escape"], ["normal", "searchForward", "cancel", "back"]],
            ["Ctrl+C cancels a search too", ["Escape", "?", "Ctrl+c"],
                ["normal", "searchBack", "cancel"]],
            ["a search's keys edit it as insert's edit the line",
                ["Escape", "/", "Left", "Right", "Home", "End", "Backspace", "Ctrl+w", "Ctrl+u"],
                ["normal", "searchForward", "caretLeft", "caretRight", "caretStart", "caretEnd",
                    "deleteBack", "deleteWord", "deleteToStart"]],
            ["n and N go on to the next match, either way", ["Escape", "n", "N"],
                ["normal", "searchNext", "searchPrevious"]],
            ["a letter typed in a search is the search's, never a key's",
                ["Escape", "/", "n", "j", "g", "g", "i", "q", "3"], ["normal", "searchForward"]],
            ["the rows' and drawer's keys do nothing in a search",
                ["Escape", "/", "PageDown", "Shift+Down", "Down", "Tab"],
                ["normal", "searchForward"]],
            ["/ and ? in insert type", ["/", "?", "n"], []]
        ];
        for (const [what, keys, want] of doing)
            holds(what, typed(keys).acts, want);
        holds("a count goes before n and N", typed(["Escape", "3", "n", "2", "N"]).counted,
            ["normal", "3 searchNext", "2 searchPrevious"]);
        holds("and before / and ?", typed(["Escape", "2", "/", "Return", "4", "?"]).counted,
            ["normal", "2 searchForward", "find", "4 searchBack"]);
        holds("/ leaves the line taking a search", typed(["Escape", "/"]).mode, "search");
        holds("Return leaves it in normal", typed(["Escape", "/", "Return"]).mode, "normal");
        holds("and Escape", typed(["Escape", "?", "Escape"]).mode, "normal");
        holds("a letter in a search is the line's, not the field's",
            typed(["Escape", "/", "x"]).ours, true);
        holds("a key with no name in a search, as Space, is left to the line",
            typed(["Escape", "/", ""]).ours, false);
        for (const act of ["searchNext", "searchPrevious"])
            holds(`${act} repeats while held`, Keys.repeats(act), true);
        for (const act of ["searchForward", "searchBack", "find", "cancel"])
            holds(`${act} is once a press`, Keys.repeats(act), false);

        // [what, the keys typed from the opening, the line they leave]
        const lines = [
            ["letters and Space type", ["Escape", "/", "f", "Space", "X"],
                { pattern: "f X", at: 3, ends: "" }],
            ["a letter goes in at the caret", ["Escape", "/", "a", "c", "Left", "b"],
                { pattern: "abc", at: 2, ends: "" }],
            ["Backspace takes the letter before the caret",
                ["Escape", "/", "a", "b", "c", "Left", "Backspace"],
                { pattern: "ac", at: 1, ends: "" }],
            ["Backspace at the start takes nothing", ["Escape", "/", "a", "Home", "Backspace"],
                { pattern: "a", at: 0, ends: "" }],
            ["Backspace with nothing left cancels", ["Escape", "/", "a", "Backspace", "Backspace"],
                { pattern: "", at: 0, ends: "cancelled" }],
            ["Ctrl+W takes the word before the caret",
                ["Escape", "/", "f", "o", "Space", "b", "a", "Ctrl+w"],
                { pattern: "fo ", at: 3, ends: "" }],
            ["Ctrl+U takes all before the caret", ["Escape", "/", "a", "b", "Left", "Ctrl+u"],
                { pattern: "b", at: 0, ends: "" }],
            ["End goes to the end", ["Escape", "/", "a", "b", "Home", "Right", "End"],
                { pattern: "ab", at: 2, ends: "" }],
            ["Return finds", ["Escape", "/", "a", "Return"], { pattern: "a", at: 1, ends: "found" }],
            ["Escape cancels", ["Escape", "/", "a", "Escape"],
                { pattern: "a", at: 1, ends: "cancelled" }]
        ];
        for (const [what, keys, want] of lines)
            holds(`in a search, ${what}`, searchTyped(keys), want);
        holds("a control character types nothing",
            Keys.typedInSearch({ pattern: "a", at: 1 }, "", "\u0001"),
            { pattern: "a", at: 1, ends: "" });

        // Rows by their words, as the launcher says them to Search.js.
        const rows = Search.searchable(["Firefox web browser", "Files", "Kitty terminal",
            "Fire alarm", "Spotify music", "firewall"]);
        holds("lowercase finds rows either case", Search.holding(rows, "fire"), [0, 3, 5]);
        holds("a capital finds only itself", Search.holding(rows, "Fire"), [0, 3]);
        holds("words past the name are found", Search.holding(rows, "music"), [4]);
        holds("a space is a letter like any", Search.holding(rows, "y t"), [2]);
        holds("what's typed is no pattern", Search.holding(Search.searchable(["a.b", "axb"]), "a.b"),
            [0]);
        holds("nothing sought finds no rows", Search.holding(rows, ""), []);

        // The rows holding a match, by their places: [what, from, forward,
        // count, where it lands].
        const holding = [2, 5, 8];
        const landings = [
            ["/ goes to the next row holding it", 2, true, 0, { at: 5, wrapped: false }],
            ["from a row between, the next", 3, true, 0, { at: 5, wrapped: false }],
            ["from the last, round to the first", 8, true, 0, { at: 2, wrapped: true }],
            ["past the last, round to the first", 9, true, 0, { at: 2, wrapped: true }],
            ["? goes to the one before", 5, false, 0, { at: 2, wrapped: false }],
            ["from the first, round to the last", 2, false, 0, { at: 8, wrapped: true }],
            ["3n goes on three", 2, true, 3, { at: 2, wrapped: true }],
            ["2n without going round", 0, true, 2, { at: 5, wrapped: false }],
            ["2N round", 5, false, 2, { at: 8, wrapped: true }],
            ["a count past every match goes round again", 2, true, 7, { at: 5, wrapped: true }],
            ["from an answer's headline, the first", -1, true, 0, { at: 2, wrapped: false }]
        ];
        for (const [what, from, forward, count, want] of landings)
            holds(what, Search.landing(holding, from, forward, count), want);
        holds("the one row holding it is found from itself, round",
            Search.landing([4], 4, true, 0), { at: 4, wrapped: true });
        holds("no row holding it lands nowhere", Search.landing([], 0, true, 0), null);

        // A place of 400 rows, each a window's long title and what it says
        // beside it, made searchable once as a search begins, its names
        // already lowered as the launcher's are for ranking, and searched
        // as each letter is typed, each in a small part of a key's 4 ms
        // (docs/design.md): it says how long each took.
        const names = [];
        const sides = [];
        for (let at = 0; at < 400; at++) {
            names.push(`Pull request #${at} \u00b7 Jitsusama/core.nix \u2014 Firefox`);
            sides.push(`firefox \u00b7 ${at % 9} \u00b7 window`);
        }
        const folded = names.map(name => name.toLowerCase());
        const tries = 200;
        const timed = work => {
            const began = Date.now();
            for (let at = 0; at < tries; at++)
                work();
            return (Date.now() - began) / tries;
        };
        const make = () => Search.searchable(names.map((name, at) => `${name}\n${sides[at]}`),
            folded.map((name, at) => `${name}\n${sides[at].toLowerCase()}`));
        const making = timed(make);
        const made = make();
        const finding = timed(() => {
            Search.holding(made, "term");
            Search.holding(made, "Program");
        }) / 2;
        console.log(`400 rows made searchable in ${making.toFixed(3)} ms,`
            + ` searched in ${finding.toFixed(3)} ms`);
        // A search's first letter pays for both, so both together keep to
        // half the key's budget, leaving the other half for drawing.
        holds("400 rows made searchable and searched in half a key's 4 ms",
            making + finding < 2, true);
        holds("and each later letter in a quarter of a millisecond", finding < 0.25, true);
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
            [":q and its Return close", ["Escape", ":", "q", "Return"], ["normal", "close"]],
            ["and so do Vim's other ways of quitting",
                ["Escape", ":", "q", "a", "!", "Return", ":", "w", "q", "Return", ":", "x",
                    "Return"], ["normal", "close", "close", "close"]],
            ["an ex line it doesn't know does nothing", ["Escape", ":", "w", "Return", "j"],
                ["normal", "next"]],
            ["the ex line's letters are its own, not normal's", ["Escape", ":", "j", "q", "G"],
                ["normal"]],
            ["Escape lets the ex line go, then steps back", ["Escape", ":", "q", "Escape", "Escape"],
                ["normal", "back"]],
            ["Backspace takes its letters, then lets it go",
                ["Escape", ":", "q", "Backspace", "Backspace", "j"], ["normal", "next"]],
            ["a colon in insert is the field's to type", [":", "q", "Return"], ["run"]],
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
                ["normal", "next"]],
            [":12 goes to the 12th row, as :12 goes to its line",
                ["Escape", ":", "1", "2", "Return"], ["normal", "12 first"]],
            ["a count before a colon is let go", ["Escape", "3", ":", "q", "Return", "j"],
                ["normal", "close", "next"]]
        ];
        for (const [what, keys, want] of counting)
            holds(what, typed(keys).counted, want);
        holds("a digit in insert is the field's to type", typed(["3"]).ours, false);
        holds("and counts nothing", typed(["3", "Escape", "j"]).counted, ["normal", "next"]);
        holds("a count typed is the line's", typed(["Escape", "4"]).ours, true);
        holds("and stands until its move", typed(["Escape", "4", "2"]).standing,
            { count: "42", typed: [] });
        holds("an ex line stands as it's typed, waiting for its Return",
            typed(["Escape", ":", "q"]).standing, { count: "", typed: [], ex: "q" });
        holds("and is let go with Escape", typed(["Escape", ":", "q", "Escape"]).standing,
            { count: "", typed: [] });
        holds("a g waiting its timeoutlen lets go, keeping the count",
            Keys.waited({ count: "3", typed: ["g"] }), { count: "3", typed: [] });
        for (const act of ["next", "previous", "first", "last", "viewTop", "viewMiddle",
            "viewBottom", "halfDown", "halfUp", "pageDown", "pageUp"])
            holds(`${act} repeats while held`, Keys.repeats(act), true);
        for (const act of ["run", "into", "out", "back", "close", "insert", "change",
            "restore", ""])
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

        searching(holds);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
