// The held notices' keys, read from their one table in Qt's own JavaScript
// engine, the one the shell runs it in. The `the-notices-take-vi-keys`
// check copies modules/quickshell/NoticeKeys.js, LineEditing.js, which
// edits the lines typed on, and Search.js, which finds what the cards say,
// beside this file and runs
// `qml -platform offscreen notice-keys.qml`.
import QtQuick
import "NoticeKeys.js" as Keys
import "Search.js" as Search

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

    // What the keys do, typed one after another from rest, each [key, at]
    // or a key pressed at 0: each act done, as its count, its operator and
    // its motion where it has them, and how the keys stand after.
    function typed(keys, mode, repeated) {
        let standing = Keys.rest();
        const acts = [];
        let ours = false;
        for (const pressed of keys) {
            const [key, at] = Array.isArray(pressed) ? pressed : [pressed, 0];
            const read = Keys.read(standing, key, mode, { repeated: repeated ?? false, now: at });
            standing = read.standing;
            ours = read.ours;
            if (read.act !== "") {
                const done = read.act === "operate" ? `${read.operator}:${read.motion}`
                    : read.act === "pending" ? `pending:${read.operator}` : read.act;
                acts.push(read.count > 0 ? `${read.count}${done}` : done);
            }
        }
        return { acts, ours, standing };
    }

    function run() {
        const failures = [];
        function holds(what, got, want) {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        }

        // [what, the keys typed in normal, the acts done]
        const normal = [
            // The arrows are their letters' motion and nothing else (Joel,
            // 2026-11-05: "In normal mode, in a buffer, arrows move the
            // cursor"), with a count as any motion.
            ["j and k move, and Down and Up as they do", ["j", "Down", "k", "Up"],
                ["down", "down", "up", "up"]],
            ["a count before an arrow", ["3", "Down"], ["3down"]],
            ["and an arrow is a motion an operator takes", ["d", "Down"],
                ["pending:done", "done:down"]],
            // A card's acts are commands, never a row stepped into, so h, l,
            // Backspace and - step nowhere (Joel, 2026-11-05: "where in vi
            // does that step into shit?"), and Left and Right are h and l.
            ["h, l, Backspace and - mean nothing on the cards", ["l", "h", "Backspace", "-"],
                ["notHere", "notHere", "notHere", "notHere"]],
            ["and the side arrows say not here", ["Right", "Left"], ["notHere", "notHere"]],
            ["Return opens, as Enter", ["Return", "Enter"], ["open", "open"]],
            ["gg and G go to the newest and the oldest", ["g", "g", "G"], ["first", "last"]],
            ["a count goes before a motion", ["5", "j", "1", "2", "k"], ["5down", "12up"]],
            ["a count goes before G", ["3", "G"], ["3last"]],
            ["a lone 0 counts nothing", ["0", "j"], ["notHere", "down"]],
            ["H M L, { } and the views", ["H", "M", "L", "{", "}", "Ctrl+D", "Ctrl+U",
                "Ctrl+F", "Ctrl+B"], ["viewTop", "viewMiddle", "viewBottom", "runBack", "runOn",
                "halfDown", "halfUp", "viewDown", "viewUp"]],
            // Vim's scrolling, which moves the view and keeps the cursor in
            // it (Joel, 2026-11-05: "that is very vi like").
            ["Ctrl+E and Ctrl+Y scroll a line, with a count", ["Ctrl+E", "3", "Ctrl+Y"],
                ["lineDown", "3lineUp"]],
            ["zz zt zb put the cursor's line in the view", ["z", "z", "z", "t", "z", "b"],
                ["viewCentre", "viewAtTop", "viewAtBottom"]],
            ["and in visual too", ["V", "Ctrl+E", "z", "z"], ["select", "lineDown", "viewCentre"]],
            ["d waits for what it takes", ["d"], ["pending:done"]],
            ["dd is done with the cursor card", ["d", "d"], ["pending:done", "done:line"]],
            ["3dd, with three", ["3", "d", "d"], ["pending:done", "3done:line"]],
            ["dj takes a motion", ["d", "j"], ["pending:done", "done:down"]],
            ["2d3j takes six", ["2", "d", "3", "j"], ["pending:done", "6done:down"]],
            ["dG to the oldest", ["d", "G"], ["pending:done", "done:last"]],
            ["dip takes a sender's run, as dap", ["d", "i", "p", "d", "a", "p"],
                ["pending:done", "done:inRun", "pending:done", "done:inRun"]],
            ["yy copies, yj two", ["y", "y", "y", "j"],
                ["pending:copy", "copy:line", "pending:copy", "copy:down"]],
            ["d then a key that's no motion says so", ["d", "u"], ["pending:done", "notHere"]],
            ["dy takes nothing", ["d", "y"], ["pending:done", "notHere"]],
            ["Escape cancels a waiting d", ["d", "Escape", "j"], ["pending:done", "down"]],
            [". u Ctrl+R", [".", "u", "Ctrl+R"], ["repeat", "undo", "redo"]],
            ["V and v choose, gv again", ["V", "v", "g", "v"], ["select", "select", "reselect"]],
            ["/ and ? find, i a I A again", ["/", "?", "i", "a", "I", "A"],
                ["find", "find", "findStart", "findEnd", "findStart", "findEnd"]],
            [": is the command line", [":"], ["command"]],
            ["ZZ and Space w let go", ["Z", "Z", "Space", "w"], ["close", "close"]],
            ["q does nothing, and says so", ["q"], ["quiet"]],
            // Escape never closes anything of ours (Joel, 2026-11-05: "fuck
            // escape quitting"): at rest it does nothing, as Vim's.
            ["Escape at rest does nothing", ["Escape"], []],
            ["Escape cancels a count first", ["4", "Escape", "j"], ["down"]],
            ["Escape cancels a key waiting for its second", ["g", "Escape", "g"], []],
            ["Space ? and F1 list the keys", ["Space", "?", "F1"], ["keys", "keys"]],
            ["Space u l turns the numbers", ["Space", "u", "l"], ["numbers"]],
            ["n N c x p and Delete say not here", ["n", "N", "c", "x", "p", "Delete"],
                ["notHere", "notHere", "notHere", "notHere", "notHere", "notHere"]],
            ["Shift+Delete forgets nothing", ["Shift+Delete"], ["notHere"]],
            ["a key with no name changes nothing", ["d", "", "G", "3", "", "j"],
                ["pending:done", "done:last", "3down"]]
        ];
        for (const [what, keys, want] of normal)
            holds(what, typed(keys).acts, want);

        const visual = [
            ["d and x are done with the selection", ["d", "x"],
                ["done:selection", "done:selection"]],
            ["y copies it", ["y"], ["copy:selection"]],
            ["moves carry it, with a count", ["2", "j", "k", "G", "g", "g"],
                ["2down", "up", "last", "first"]],
            ["o goes to its other end", ["o"], ["otherEnd"]],
            ["ip takes the sender's run", ["i", "p"], ["inRun"]],
            ["V and Escape end it", ["V", "Escape"], ["select", "back"]],
            [": gives its commands", [":"], ["command"]],
            ["u and / are no keys of a selection", ["u", "/"], ["notHere", "notHere"]]
        ];
        for (const [what, keys, want] of visual)
            holds(what, typed(keys, "visual").acts, want);

        // V j j d d: the second d, within the wait, joins the first.
        let standing = Keys.rest();
        standing = Keys.read(standing, "d", "visual", { now: 1000 }).standing;
        const joined = Keys.read(standing, "d", "normal", { now: 1200 });
        holds("a second d within the wait is taken with visual's", [joined.act, joined.ours],
            ["", true]);
        holds("and leaves nothing waiting", joined.standing, Keys.rest());
        const late = Keys.read(standing, "d", "normal", { now: 1400 });
        holds("one after it starts afresh", late.act, "pending");
        holds("another key goes on as ever", Keys.read(standing, "j", "normal", { now: 1100 }).act,
            "down");
        // The wait runs from when visual's d has done its work, so a slow
        // act under load never spends the time the second d has.
        const settled = Keys.settled(standing, 1350);
        holds("the wait runs from when the act is done",
            Keys.read(settled, "d", "normal", { now: 1600 }).act, "");
        holds("and nothing else changes", Object.assign({}, settled, { taken: null }),
            Object.assign({}, standing, { taken: null }));
        holds("with nothing taken it settles nothing", Keys.settled(Keys.rest(), 1350),
            Keys.rest());

        holds("a held j moves", typed(["j"], "normal", true).acts, ["down"]);
        for (const key of ["d", "u", ".", "Ctrl+R", "Return", "V"])
            holds(`a held ${key} does nothing`, typed([key], "normal", true).acts, []);
        const waiting = Keys.read(Keys.rest(), "d", "normal").standing;
        holds("a held j after d takes nothing",
            Keys.read(waiting, "j", "normal", { repeated: true }).act, "");

        // A line typed key by key, in a mode, from what it held.
        function line(keys, mode, from) {
            let typing = { act: "", line: Keys.line(from ?? "") };
            const acts = [];
            for (const key of keys) {
                typing = Keys.typing(typing.line, key, mode, false);
                if (typing.act !== "")
                    acts.push(typing.act);
            }
            return { text: typing.line.text, caret: typing.line.caret, acts };
        }
        holds("letters type and Space a space", line(["B", "u", "Space", "l"], "find"),
            { text: "Bu l", caret: 4, acts: [] });
        holds("Backspace takes the one before the caret",
            line(["a", "b", "c", "Left", "Backspace"], "find"),
            { text: "ac", caret: 1, acts: [] });
        holds("Home, End and typing at the caret", line(["b", "Home", "a", "End", "c"], "find"),
            { text: "abc", caret: 3, acts: [] });
        holds("Ctrl+W takes a word, Ctrl+U the rest before",
            line(["Ctrl+W"], "find", "go to slack"), { text: "go to ", caret: 6, acts: [] });
        holds("Ctrl+U", line(["Left", "Ctrl+U"], "find", "slack"), { text: "k", caret: 0, acts: [] });
        holds("Backspace on nothing leaves", line(["a", "Backspace", "Backspace"], "find").acts,
            ["leave"]);
        holds("a held Backspace stops at empty",
            Keys.typing(Keys.line(""), "Backspace", "find", true).act, "");
        holds("a colon first is the command line", line([":"], "find").acts, ["command"]);
        holds("a colon later types", line(["a", ":"], "find").text, "a:");
        holds("the find's own keys", line(["Return", "Down", "Tab", "Ctrl+N", "Up",
            "Shift+Tab", "Ctrl+P", "Ctrl+Y", "Escape", "Ctrl+C"], "find").acts,
            ["open", "choose", "choose", "choose", "chooseBack", "chooseBack", "chooseBack",
                "take", "normal", "normal"]);
        holds("the command line's own keys", line(["Return", "Tab", "Ctrl+E", "Escape"],
            "command").acts, ["run", "choose", "away", "drop"]);
        holds("Ctrl+R + puts the clipboard", line(["Ctrl+R", "+"], "command").acts, ["paste"]);
        holds("insert's own keys", line(["Return", "Escape"], "insert").acts, ["send", "normal"]);
        holds("in insert, letters type", line(["o", "k"], "insert").text, "ok");

        // A prompt's words in normal, key by key from a caret.
        function prompt(keys, text, caret) {
            let standing = Keys.rest();
            let typing = { act: "", line: Keys.line(text, caret) };
            const acts = [];
            for (const key of keys) {
                typing = Keys.prompting(standing, typing.line, key);
                standing = typing.standing;
                if (typing.act !== "")
                    acts.push(typing.act);
            }
            return { text: typing.line.text, caret: typing.line.caret, acts };
        }
        holds("0 and $ go to its ends", prompt(["0"], "on my way", 4).caret, 0);
        holds("$ to the last letter", prompt(["$"], "on my way", 0).caret, 8);
        holds("w to the next word", prompt(["w"], "on my way", 0).caret, 3);
        holds("b to the word before", prompt(["b"], "on my way", 6).caret, 3);
        holds("e to the word's end", prompt(["e"], "on my way", 3).caret, 4);
        holds("x cuts a letter", prompt(["x"], "on my way", 0).text, "n my way");
        holds("dw cuts a word", prompt(["d", "w"], "on my way", 3),
            { text: "on way", caret: 3, acts: [] });
        holds("cw changes one, into insert", prompt(["c", "w"], "on my way", 3),
            { text: "on  way", caret: 3, acts: ["insert"] });
        holds("a inserts after", prompt(["a"], "ok", 0), { text: "ok", caret: 1, acts: ["insert"] });
        holds("A at the end", prompt(["A"], "ok", 0).caret, 2);
        holds("Return sends, : commands, u undoes", prompt(["Return", ":", "u"], "ok", 0).acts,
            ["send", "command", "undo"]);
        holds("q says not here", prompt(["q"], "ok", 0).acts, ["notHere"]);
        // Normal over a reply's words moves by h and l alone, as every
        // normal does; its insert keeps the arrows for the caret.
        holds("h and l move along its words", prompt(["l", "l", "h"], "on my way", 0).caret, 1);
        holds("and the arrows as they do", prompt(["Right", "Right", "Left"], "on my way", 4),
            { text: "on my way", caret: 5, acts: [] });

        // Space ?'s list: a heading each mode, then each act's keys.
        const listed = Keys.keyList();
        holds("it opens on normal", listed[0].heading, "Normal: moving and acting");
        // Outside a line, an arrow is only its letter's motion: alone, never
        // held with another key or in a sequence, and on an act its letter
        // does (Joel, 2026-11-05). The find, the command line and insert keep
        // them for the caret and choosing, as Vim's insert does.
        const lines = ["find", "command", "insert"];
        const letters = { Down: "j", Up: "k", Left: "h", Right: "l" };
        const pressed = key => Array.isArray(key) ? key : String(key).split(" ");
        const arrowed = key => pressed(key)
            .some(part => part.split("+").pop() in letters);
        holds("outside a line an arrow is only its letter's motion", Keys.table
            .filter(one => (one.modes ?? ["normal"]).some(mode => !lines.includes(mode)))
            .filter(one => one.keys.some(key => arrowed(key)
                && !(key in letters && one.keys.includes(letters[key]))))
            .map(one => one.act), []);
        // Never named in normal's or visual's hints or lines of the key
        // list, where the letter is.
        let section = "";
        const named = [];
        for (const one of listed) {
            section = one.heading !== "" ? one.heading : section;
            if (/^(Normal|Visual)/.test(section) && one.keys.split(" ").some(key => key in letters))
                named.push(one.keys);
        }
        holds("normal's and visual's lines of the key list name no arrow", named, []);
        holds("nor does a hint", Keys.hints("normal", {}, {}).concat(Keys.hints("visual", {}, {}))
            .filter(one => one.key in letters).map(one => one.key), []);
        holds("a move lists its letter alone", listed[1],
            { heading: "", keys: "j", does: "down a card", act: "down" });
        holds("a sequence is listed as typed", listed.find(one => one.act === "first").keys, "gg");
        holds("each mode has its heading", listed.filter(one => one.heading !== "")
            .map(one => one.heading), ["Normal: moving and acting", "The find: the / line",
            "Commands: the : line", "Visual: choosing several", "Insert: writing words"]);
        holds("an act is listed once a mode", listed.filter(one => one.act === "done").length, 2);

        // The mode, named at the foot's start in three capitals (Joel,
        // 2026-11-05: "the mode should be 3 letters, in caps, and in a place
        // where they are more obvious"), as the key language's five; a
        // reply's normal and the key list are normal, and pending is too,
        // as Vim's status line names it.
        holds("each mode's mark", ["normal", "find", "command", "insert", "visual", "prompt",
            "listing", "pending"].map(mode => Keys.mark(mode)),
            ["NOR", "FND", "CMD", "INS", "VIS", "NOR", "NOR", "NOR"]);
        holds("every line fits at 2", listed.filter(one => one.does.length > 18
            || one.keys.length > 14).map(one => one.does), []);
        holds(":help dd opens at d", listed[Keys.helpAt(listed, "done")].act, "done");

        // As the launcher's foot: Return at the right end, Space ? just before it.
        const said = (mode, has, words) => Keys.hints(mode, has, words ?? {})
            .map(one => `${one.key} ${one.word}`);
        holds("normal's hints, Space ? then Return last", said("normal", {}, { done: "read" }),
            ["d read", ": commands", "Space ? keys", "\u21b5 go to"]);
        holds("a card that opens nothing hints no Return",
            Keys.hints("normal", { open: false }, {}).map(one => one.act),
            ["done", "command", "keys"]);
        holds("visual's hints", said("visual", {}, { done: "forget" }),
            ["d forget", "y copy", ": commands", "Space ? keys"]);
        holds("the find's hints", said("find", {}), ["Esc normal", "\u21b5 go to"]);
        holds("insert ends with Return's", said("insert", {}), ["Esc normal", "\u21b5 send"]);
        holds("the key list says how to leave it", said("listing", {}),
            ["j k scroll", "Esc back"]);
        holds("keys are spelt as the launcher's foot spells them",
            [Keys.spoken(["Space", "?"]), Keys.spoken("Escape"), Keys.spoken("Ctrl+R")],
            ["Space ?", "Esc", "Ctrl+R"]);
        holds("a count's motion says it", listed.find(one => one.act === "first").does,
            "newest, or {n}th");

        holds("a list that fits is All", Keys.ruler(0, 5, 5), "All");
        holds("at its start, Top", Keys.ruler(0, 5, 13), "Top");
        holds("at its end, Bot", Keys.ruler(8, 5, 13), "Bot");
        holds("between, how far through", Keys.ruler(2, 5, 13), "25%");

        holds("lowercase finds either case", Search.found("build", "Build log, build"),
            [[0, 5], [11, 16]]);
        holds("a capital finds only itself", Search.found("Build", "build Build"), [[6, 11]]);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
