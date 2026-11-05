// The launcher's field's keys in its two modes, read from their one
// table in Qt's own JavaScript engine, the one the shell runs it in, and
// what its operators do to the rows. The `the-query-field-takes-vi-modes`
// check copies modules/quickshell/QueryKeys.js, ListedKeys.js, Search.js and
// Operators.js beside this file and runs `qml -platform offscreen
// query-keys.qml`.
import QtQuick
import "ListedKeys.js" as Listed
import "QueryKeys.js" as Keys
import "Search.js" as Search
import "Operators.js" as Operators

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
    // in `counted`, and an operator's with the motion it took, as
    // "ending 6 next".
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
                const motion = !read.motion ? "" : read.motion.count > 0
                    ? ` ${read.motion.count} ${read.motion.act}` : ` ${read.motion.act}`;
                counted.push((read.count > 0 ? `${read.count} ${read.act}` : read.act) + motion);
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
        // Each is timed as the fastest of twenty batches of fifty, since a
        // check shares its machine: a slower batch is the machine's other
        // work or the collector, not the search's own cost. Fifty a batch
        // keeps Date.now's whole milliseconds a fiftieth of one each.
        const batches = 20;
        const tries = 50;
        const timed = work => {
            let fastest = Infinity;
            for (let batch = 0; batch < batches; batch++) {
                const began = Date.now();
                for (let at = 0; at < tries; at++)
                    work();
                fastest = Math.min(fastest, (Date.now() - began) / tries);
            }
            return fastest;
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

    // d and y, Vim's operators, over the rows as lines: the keys that make
    // one, the rows it spans, and which of a row's actions it does.
    function operating(holds) {
        // [what, the keys typed from the opening, each act with its count
        // and motion]
        const operations = [
            ["dd ends the row", ["Escape", "d", "d"], ["normal", "ending"]],
            ["a count before dd ends that many", ["Escape", "3", "d", "d"],
                ["normal", "3 ending"]],
            ["and so does a count between", ["Escape", "d", "3", "d"], ["normal", "3 ending"]],
            ["the two counts multiply, as Vim's do", ["Escape", "2", "d", "3", "d"],
                ["normal", "6 ending"]],
            ["dj and dk end the row and the one below or above",
                ["Escape", "d", "j", "d", "k", "d", "Down", "d", "Up"],
                ["normal", "ending next", "ending previous", "ending next", "ending previous"]],
            ["a count after d goes with its motion", ["Escape", "d", "3", "j"],
                ["normal", "ending 3 next"]],
            ["and one before it too, multiplied", ["Escape", "2", "d", "3", "j"],
                ["normal", "ending 6 next"]],
            ["a count before d alone goes with its motion", ["Escape", "4", "d", "k"],
                ["normal", "ending 4 previous"]],
            ["dgg and dG end to the first and the last", ["Escape", "d", "g", "g", "d", "G"],
                ["normal", "ending first", "ending last"]],
            ["d5G ends to the 5th", ["Escape", "d", "5", "G"], ["normal", "ending 5 last"]],
            ["dH dM dL end to the view's top, middle and bottom",
                ["Escape", "d", "H", "d", "M", "d", "L"],
                ["normal", "ending viewTop", "ending viewMiddle", "ending viewBottom"]],
            ["yy copies the row", ["Escape", "y", "y"], ["normal", "copy"]],
            ["3yy and y3y copy three", ["Escape", "3", "y", "y", "y", "3", "y"],
                ["normal", "3 copy", "3 copy"]],
            ["y takes the motions d does", ["Escape", "y", "2", "k", "y", "G", "y", "g", "g"],
                ["normal", "copy 2 previous", "copy last", "copy first"]],
            ["Escape lets d go, then steps back", ["Escape", "d", "Escape", "j", "Escape"],
                ["normal", "next", "back"]],
            ["and a count with it, at once, as Vim's does",
                ["Escape", "3", "d", "Escape", "j"], ["normal", "next"]],
            ["and a count after it", ["Escape", "d", "3", "Escape", "j"], ["normal", "next"]],
            ["a key that ends nothing lets d go", ["Escape", "d", "x", "j"], ["normal", "next"]],
            ["Ctrl+D is a scroll, not a motion, so it lets d go",
                ["Escape", "d", "Ctrl+d", "j"], ["normal", "next"]],
            ["and so are the screens and Page Down",
                ["Escape", "d", "Ctrl+f", "d", "PageDown", "y", "Ctrl+u", "k"],
                ["normal", "previous"]],
            ["another operator lets d go", ["Escape", "d", "y", "y"], ["normal"]],
            ["a g with anything but g after it lets d go", ["Escape", "d", "g", "j", "j"],
                ["normal", "next"]],
            ["a colon after d opens no ex line", ["Escape", "d", ":", "j"], ["normal", "next"]],
            ["Shift pressed for a capital keeps d waiting", ["Escape", "d", "", "G"],
                ["normal", "ending last"]],
            ["in insert, d and y type", ["d", "d", "y", "y"], []]
        ];
        for (const [what, keys, want] of operations)
            holds(what, typed(keys).counted, want);
        holds("a d begun is the line's", typed(["Escape", "d"]).ours, true);
        holds("and stands with the count before it", typed(["Escape", "2", "d"]).standing,
            { count: "", typed: [], operator: { key: "d", count: "2" } });
        holds("d and y leave the line in normal", typed(["Escape", "d", "d", "y", "j"]).mode,
            "normal");
        holds("an operator waits past timeoutlen, as Vim's does",
            Keys.waited({ count: "3", typed: [], operator: { key: "d", count: "" } }),
            { count: "3", typed: [], operator: { key: "d", count: "" } });
        holds("though a g after it lets go",
            Keys.waited({ count: "", typed: ["g"], operator: { key: "y", count: "2" } }),
            { count: "", typed: [], operator: { key: "y", count: "2" } });
        for (const act of ["ending", "copy"])
            holds(`${act} is once a press, so a held d never makes dd`, Keys.repeats(act), false);
        holds("the field says dd for ending", Keys.keyOf("ending", "normal"), "dd");
        holds("and yy for copying", Keys.keyOf("copy", "normal"), "yy");
        // d's ask says the keys that answer it, as d2j, so a motion is
        // said by Vim's letters rather than its first key, an arrow.
        holds("a motion is said by its letters",
            ["next", "previous", "first", "last", "viewTop", "viewMiddle", "viewBottom"]
                .map(act => Keys.lettersOf(act, "normal")),
            ["j", "k", "gg", "G", "H", "M", "L"]);
        holds("and an act with no letters by its first key",
            Keys.lettersOf("halfDown", "normal"), "Ctrl+d");

        // Twenty rows, the 5th to the 12th in view, the 7th selected, as
        // the moves' cases have them: [what, the count, the motion, the
        // rows it spans].
        const place = { at: 6, highest: 0, last: 19, top: 4, bottom: 11, screen: 8 };
        const spans = [
            ["dd, the row", 0, null, { from: 6, to: 6 }],
            ["3dd, it and the two below", 3, null, { from: 6, to: 8 }],
            ["30dd, to the last, as Neovim's does", 30, null, { from: 6, to: 19 }],
            ["dj, it and the next", 0, { act: "next", count: 0 }, { from: 6, to: 7 }],
            ["d3j, four", 0, { act: "next", count: 3 }, { from: 6, to: 9 }],
            ["dk, the one above and it", 0, { act: "previous", count: 0 }, { from: 5, to: 6 }],
            ["dgg, to the first", 0, { act: "first", count: 0 }, { from: 0, to: 6 }],
            ["dG, to the last", 0, { act: "last", count: 0 }, { from: 6, to: 19 }],
            ["d5G, back to the 5th", 0, { act: "last", count: 5 }, { from: 4, to: 6 }],
            ["dH, to the view's top", 0, { act: "viewTop", count: 0 }, { from: 4, to: 6 }],
            ["dM, to its middle", 0, { act: "viewMiddle", count: 0 }, { from: 6, to: 7 }],
            ["dL, to its bottom", 0, { act: "viewBottom", count: 0 }, { from: 6, to: 11 }]
        ];
        for (const [what, count, motion, want] of spans)
            holds(`${what} is spanned`, Operators.spanned(count, motion, place), want);
        holds("yy on an answer's headline spans it",
            Operators.spanned(0, null, Object.assign({}, place, { at: -1, highest: -1 })),
            { from: -1, to: -1 });
        holds("no rows span nothing",
            Operators.spanned(2, null,
                { at: 0, highest: 0, last: -1, top: 0, bottom: 0, screen: 8 }),
            { from: 0, to: -1 });

        // Rows and what can be done to them, as the launcher's providers
        // make them: a window, whose Close ends it and whose Copy Title
        // copies; an answer's row, which is a copy itself; an act that
        // ends, as a step into a window lists it; and one with neither.
        const run = () => {};
        const close = { key: "verb:close", verb: "close", act: "ending", run };
        const title = { key: "verb:title", verb: "copy", copies: () => "Notes", run };
        const float = { key: "verb:float", verb: "float", run };
        const window = { key: "window:1", verb: "go", run, verbs: () => [float, title, close] };
        const answer = { key: "answer", verb: "copy", copies: "42", run };
        // An answer that says something and copies nothing, whose Return
        // only closes: its verb is close, and it ends nothing.
        const said = { key: "said", verb: "close", run };
        const bare = { key: "bare", verb: "open", run };
        const actions = row => row.verbs ? row.verbs() : [];
        const copies = row => row.verb === "copy" && !!row.run;
        holds("a window's ending action is its Close",
            Operators.endingOf(window, actions(window)), close);
        holds("an ending action is its own", Operators.endingOf(close, []), close);
        holds("a verb of close alone ends nothing", Operators.endingOf(said, []), null);
        holds("nor does a row with no ending action", Operators.endingOf(bare, []), null);
        // A program's Quit steps in to the row that quits on Return, so
        // Return asks once; d's own ask stands for that, so d quits.
        const quitting = () => "quit";
        const quit = { key: "verb:program:quit", act: "ending",
            targets: () => [{ key: "verb:program:quit:sure", run: quitting }] };
        const program = { key: "program:notes", run, verbs: () => [quit] };
        holds("a program's ending action is its Quit, which runs nothing itself",
            Operators.endingOf(program, actions(program)), quit);
        holds("so d does the row it steps in to", Operators.doing(quit)(), "quit");
        holds("and a Close as it runs", Operators.doing(close), run);
        holds("an ending action with nothing to do ends nothing",
            Operators.endingOf({ key: "verb:close", act: "ending" }, []), null);
        holds("a window's copy is its Copy Title",
            Operators.copyOf(window, actions(window), copies), title);
        holds("an answer's row is its own copy", Operators.copyOf(answer, [], copies), answer);
        holds("a row with no copy has none", Operators.copyOf(bare, [], copies), null);
        holds("what a copy copies, by a function or as it is",
            [Operators.textOf(title), Operators.textOf(answer), Operators.textOf(bare)],
            ["Notes", "42", ""]);
        holds("several copies are joined a line each, as a linewise yank holds them",
            Operators.copied([title, bare, answer]), "Notes\n42");
        const rows = [bare, window, said, window, answer];
        holds("the ending actions over a span, by the row each ends",
            Operators.endings(Operators.taken(rows, { from: 0, to: 3 }, null), actions)
                .map(ending => [ending.at, ending.action.key]),
            [[1, "verb:close"], [3, "verb:close"]]);
        holds("and the copies",
            Operators.copiesIn(Operators.taken(rows, { from: 1, to: 4 }, null), actions, copies)
                .map(copy => [copy.at, copy.action.key]),
            [[1, "verb:title"], [3, "verb:title"], [4, "answer"]]);
        holds("a headline above the rows is taken as the row at -1",
            Operators.taken(rows, { from: -1, to: 0 }, answer).map(one => [one.at, one.row]),
            [[-1, answer], [0, bare]]);
        holds("and none where there's none",
            Operators.taken(rows, { from: -1, to: 0 }, null).map(one => one.at), [0]);
        holds("an ending action with no undo asks once", Operators.asks([close]), true);
        holds("one that's put back doesn't",
            Operators.asks([Object.assign({ undo: run }, close)]), false);
        holds("a second dd over the same rows answers the ask",
            Operators.answers({ act: "ending", from: 3, to: 5 }, "ending", { from: 3, to: 5 }),
            true);
        holds("over other rows it doesn't",
            Operators.answers({ act: "ending", from: 3, to: 5 }, "ending", { from: 3, to: 4 }),
            false);
        holds("nor does a y",
            Operators.answers({ act: "ending", from: 3, to: 3 }, "copy", { from: 3, to: 3 }),
            false);
        holds("nor anything with no ask", Operators.answers(null, "ending", { from: 3, to: 3 }),
            false);
        // The ask says the keys that answer it, as they were typed.
        holds("the ask says the keys that answer it", [
            Operators.typedAs("ending", 0, null),
            Operators.typedAs("ending", 3, null),
            Operators.typedAs("ending", 0, { act: "next", count: 0 }),
            Operators.typedAs("ending", 0, { act: "next", count: 6 }),
            Operators.typedAs("ending", 0, { act: "last", count: 0 }),
            Operators.typedAs("copy", 0, { act: "first", count: 0 })
        ], ["dd", "3dd", "dj", "d6j", "dG", "ygg"]);

        // A place of 400 windows, each closed and copied by its actions:
        // the keys, the span and the actions over it of yG and of dG, each
        // timed as the fastest of twenty batches of fifty, as the search's
        // are, in a quarter of a key's 4 ms.
        const windows = [];
        for (let at = 0; at < 400; at++) {
            const named = `Pull request #${at} \u00b7 Jitsusama/core.nix`;
            windows.push({ key: `window:${at}`, verb: "go", run,
                verbs: () => [float, { key: "verb:title", verb: "copy", copies: () => named,
                    run }, close] });
        }
        const all = { at: 0, highest: 0, last: 399, top: 0, bottom: 7, screen: 8 };
        const spanOf = operator => {
            let standing = Keys.rest();
            let last = null;
            for (const key of [operator, "G"]) {
                last = Keys.read(standing, key, "normal");
                standing = last.standing;
            }
            return Operators.spanned(last.count, last.motion, all);
        };
        const timed = work => {
            let fastest = Infinity;
            for (let batch = 0; batch < 20; batch++) {
                const began = Date.now();
                for (let at = 0; at < 50; at++)
                    work();
                fastest = Math.min(fastest, (Date.now() - began) / 50);
            }
            return fastest;
        };
        const yanking = timed(() => Operators.copied(Operators.copiesIn(
            Operators.taken(windows, spanOf("y"), null), actions, copies).map(one => one.action)));
        const ending = timed(() => Operators.asks(Operators.endings(
            Operators.taken(windows, spanOf("d"), null), actions).map(one => one.action)));
        console.log(`yG over 400 rows in ${yanking.toFixed(3)} ms, dG in ${ending.toFixed(3)} ms`);
        holds("yG over 400 rows in a quarter of a key's 4 ms", yanking < 1, true);
        holds("and dG", ending < 1, true);
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
        holds("a space on the ex line is typed", typed(["Escape", ":", "q", "Space"]).standing,
            { count: "", typed: [], ex: "q " });
        holds("and Return reads past it",
            typed(["Escape", ":", "q", "Space", "Return"]).acts, ["normal", "close"]);
        holds("insert's Space is the field's to type", typed(["Space"]).ours, false);
        holds("a search's Space is a letter of it",
            Keys.read({ count: "", typed: [] }, "Space", "search").act, "");
        holds("a Space waiting its timeoutlen lets go", Keys.waited({ count: "", typed: ["Space"] }),
            { count: "", typed: [] });
        // Space ? lists the keys by what each line says, its keys as the
        // desktop draws them, Vim's own where an act has them, and its arrow.
        const lines = { rows: "rows", screen: "screen", inOut: "in and out", run: "run",
            lastPlace: "last", keys: "keys", close: "close", drawer: "drawer", ends: "ends",
            search: "search", found: "found", nextRow: "next row", caret: "caret",
            word: "word" };
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
        holds("the list's searches, ? among them alone", keyed("search"), "/  ?");
        holds("the list's n and N", keyed("found"), "n  N");
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
        // in that mode's section.
        const arrows = ["Up", "Down", "Left", "Right"];
        holds("every act an arrow does lists its arrow", Keys.table
            .filter(one => one.mode !== "search")
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

        searching(holds);
        operating(holds);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
