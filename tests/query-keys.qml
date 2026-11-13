// The launcher's field's keys in its two modes, read from their one
// table in Qt's own JavaScript engine, the one the shell runs it in, and
// what its operators do to the rows. The `the-query-field-takes-vi-modes`
// check copies modules/quickshell/QueryKeys.js, Editing.js, Typing.js,
// ListedKeys.js, DrawerPage.js, DrawerFigure.js, DrawerProse.js, DrawerValue.js, Search.js,
// Reading.js,
// LineEditing.js, Operators.js, Finding.js and Confirm.js beside this file and runs
// `qml -platform offscreen query-keys.qml`.
import QtQuick
import "Editing.js" as Editing
import "ListedKeys.js" as Listed
import "KeyNames.js" as KeyNames
import "QueryKeys.js" as Keys
import "ModeMarks.js" as ModeMarks
import "Typing.js" as Typing
import "Operators.js" as Operators
import "Finding.js" as Finding
import "Confirm.js" as Confirm
import "DrawerPage.js" as Page
import "DrawerFigure.js" as Figure
import "DrawerProse.js" as Prose
import "DrawerValue.js" as Value
import "Identifier.js" as Identifier

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

    // What the keys do, typed one after another from the line's opening, on
    // the find, or in insert where the line holds a prompt's words (`words`):
    // each act done, the mode it's left in, and whether the last key was the
    // line's rather than the field's to type.
    // Each act done is said with the count typed before it, as "3 next",
    // in `counted`, an operator's with the motion it took, as "ending 6
    // next", each search run with what it sought, in `sought`, and each
    // key that meant nothing where it was pressed, which the foot answers
    // with "not here", in `idle`.
    function typed(keys, words = false) {
        const typing = words ? "insert" : "find";
        let standing = Keys.rest();
        let mode = typing;
        const acts = [];
        const counted = [];
        const sought = [];
        const idle = [];
        let ours = false;
        for (const key of keys) {
            const read = Keys.read(standing, key, mode);
            standing = read.standing;
            ours = read.ours;
            mode = Keys.modeAfter(read.act, mode, typing);
            if (read.act !== "") {
                acts.push(read.act);
                const motion = !read.motion ? "" : read.motion.count > 0
                    ? ` ${read.motion.count} ${read.motion.act}` : ` ${read.motion.act}`;
                counted.push((read.count > 0 ? `${read.count} ${read.act}` : read.act) + motion);
            }
            if (read.sought)
                sought.push(read.sought);
            if (read.idle)
                idle.push(key);
        }
        return { acts, counted, sought, idle, ours, mode, standing };
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
                ["Escape", "d", "j", "d", "k"], ["normal", "ending next", "ending previous"]],
            ["an arrow is its letter's motion, d Down as dj", ["Escape", "d", "Down", "d", "Up"],
                ["normal", "ending next", "ending previous"]],
            ["3 Down goes three, as 3j", ["Escape", "3", "Down"], ["normal", "3 next"]],
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
            ["Escape lets d go, then does nothing", ["Escape", "d", "Escape", "j", "Escape"],
                ["normal", "next"]],
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
            ["a colon after d opens no command line", ["Escape", "d", ":", "j"], ["normal", "next"]],
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
        for (const act of ["ending", "copy"])
            holds(`${act} is once a press, so a held d never makes dd`, Keys.repeats(act), false);
        holds("the field says dd for ending", Keys.keyOf("ending", "normal"), "dd");
        holds("and yy for copying", Keys.keyOf("copy", "normal"), "yy");

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
        // make them: a window, whose Close ends it and whose Copy the Title
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
        holds("a window's copy is its Copy the Title",
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
        // A row whose thing has gone stays in its place until a key, and
        // there's nothing left to end or copy there, so it's never taken
        // nor counted.
        const leftBehind = [bare, { name: "Gone", gone: true }, bare];
        holds("a row whose thing has gone is never taken",
            Operators.taken(leftBehind, { from: 0, to: 2 }, null).map(one => one.at), [0, 2]);
        // What asks is the key language's to say, by each action's command
        // (Options.commands): what can't be undone asks once; what's
        // undoable acts at once; and a guarded one, as a window's Close,
        // whose program keeps its own work, asks only over more than one.
        const commands = { halt: { undoable: false, guarded: false },
            close: { undoable: false, guarded: true },
            disconnect: { undoable: true, guarded: false } };
        const as = command => Object.assign({ command }, close);
        holds("what can't be undone asks once", Operators.asks([as("halt")], commands), true);
        holds("what's undoable doesn't, once the row says how it's put back",
            Operators.asks([Object.assign({ undo: run }, as("disconnect"))], commands), false);
        holds("though it asks until the launcher can put it back",
            Operators.asks([as("disconnect")], commands), true);
        holds("nor one guarded thing", Operators.asks([as("close")], commands), false);
        holds("though more than one guarded thing asks once",
            Operators.asks([as("close"), as("close")], commands), true);
        holds("as does any that can't be undone among undoable ones",
            Operators.asks([Object.assign({ undo: run }, as("disconnect")), as("halt")],
                commands), true);
        holds("an action the language doesn't name asks", Operators.asks([close], commands),
            true);
        holds("unless it says how it's put back",
            Operators.asks([Object.assign({ undo: run }, close)], commands), false);
        // One that says what it leaves asks even where it's guarded and
        // alone, as a session's Close does, since the ask says it.
        holds("one guarded thing saying what it leaves asks",
            Operators.asks([Object.assign({ leaves: "Its folder stays." }, as("close"))],
                commands), true);
        // The ask, as Vim's ask_yesno asks: what it would end, by name for
        // one row and by count for more, its verb the one they share.
        // It names the one row it ends, and more by their count and kind,
        // each verb with its own, so Joel needn't look to see what they are
        // (VKCF3Z, 2026-11-05).
        const asking = { ask: "{ends} {name}? (y/n)", asks: "{what}? (y/n)",
            askLeaves: "{ends} {name}? {leaves} (y/n)",
            part: "{ends} {count} {kind}", ends: "end", thing: "thing", and: "and",
            many: { copy: "copies" } };
        const closing = name => ({ ends: "close", name, kind: "window" });
        const copied = { ends: "forget", name: "84", kind: "copy" };
        holds("the ask names the one row it ends",
            Confirm.question([closing("Notes")], asking), "Close Notes? (y/n)");
        // A row whose name already says its act, as a line run from Forget
        // the Clipboard, is asked by its name alone, not "Forget Forget".
        holds("and one whose name begins with its verb by its name alone",
            Confirm.question([{ ends: "forget", name: "Forget the Clipboard" }], asking),
            "Forget the Clipboard? (y/n)");
        // An act with words of its own is asked in them, its thing and
        // what it ends, never the place it's in (VKCF3Z, 2026-11-08).
        holds("and one with its own words in them whole",
            Confirm.question([{ ends: "reboot", name: "Power", kind: "place",
                asked: "Reboot the Machine, Closing 12 Windows" }], asking),
            "Reboot the Machine, Closing 12 Windows? (y/n)");
        // An act asked from its own row is an act of the thing holding it,
        // never of a place, on the : line and from a terminal alike.
        const keychron = { name: "Keychron" };
        const disconnect = { name: "Disconnect", ends: "disconnect" };
        holds("an act held in a thing is asked of the thing",
            Confirm.actOf(keychron, disconnect, "device"), keychron);
        holds("and one held in a place is asked of itself",
            Confirm.actOf({ name: "Power" }, disconnect, "place"), disconnect);
        holds("and one held in nothing is asked of itself",
            Confirm.actOf(null, disconnect, ""), disconnect);
        holds("and a window's Close names the window",
            Confirm.question([{ ends: "close", name: Confirm.actOf({ name: "Plover Notes" },
                { name: "Close" }, "window").name }], asking), "Close Plover Notes? (y/n)");
        holds("and one that says what it leaves says so before y/n, as a sentence",
            Confirm.question([Object.assign({ leaves: "its worktree stays." },
                closing("Lamplight"))], asking), "Close Lamplight? Its worktree stays. (y/n)");
        // Where the line can't hold it, what stays gives way from its end
        // at a whole word, never what it does, to what, or its keys, so
        // it's still a true question (VKCF3Z, 2026-11-07).
        const staying = Confirm.questionParts([Object.assign(
            { leaves: "its worktree and 1 to-do stay." }, closing("TESTQC"))], asking);
        holds("one saying what stays is asked in its parts", staying, { head: "Close TESTQC?",
            clause: "Its worktree and 1 to-do stay.", tail: "(y/n)" });
        const within = most => words => words.length <= most;
        holds("said whole where it fits", Confirm.fitted(staying, within(60)),
            "Close TESTQC? Its worktree and 1 to-do stay. (y/n)");
        holds("what stays gives way from its end at a whole word",
            Confirm.fitted(staying, within(32)), "Close TESTQC? Its worktree\u2026(y/n)");
        holds("and all of it before what it closes does", Confirm.fitted(staying, within(20)),
            "Close TESTQC? \u2026(y/n)");
        holds("one saying nothing stays is said whole, its line cutting it",
            Confirm.fitted(Confirm.questionParts([closing("Notes")], asking), within(5)),
            "Close Notes? (y/n)");
        holds("and counts more by their kind", Confirm.question([closing("Notes"),
            closing("Mail")], asking), "Close 2 windows? (y/n)");
        holds("and each verb with its own, the first first", Confirm.question([closing("Notes"),
            copied, closing("Mail")], asking), "Close 2 windows and forget 1 copy? (y/n)");
        holds("three by commas, the last by and", Confirm.question([closing("Notes"), copied,
            { ends: "quit", name: "Firefox", kind: "program" }, copied], asking),
        "Close 1 window, forget 2 copies and quit 1 program? (y/n)");
        holds("a row with no kind is a thing", Confirm.question([{ ends: "close", name: "a" },
            { ends: "close", name: "b" }], asking), "Close 2 things? (y/n)");
        holds("and one with no verb takes the word for ending", Confirm.question([
            { name: "a", kind: "window" }, closing("b")], asking),
        "End 1 window and close 1 window? (y/n)");
        // Only y answers yes, and only after the ask has stood 300 ms, so
        // a key typed on from what asked can't answer it; any other no.
        const ask = Confirm.asked("Close Notes? (y/n)", run, run, 1000);
        holds("the ask holds its question", ask.question, "Close Notes? (y/n)");
        holds("and one asked in its parts holds it whole",
            Confirm.asked(staying, run, run, 1000).question,
            "Close TESTQC? Its worktree and 1 to-do stay. (y/n)");
        holds("a key in the ask's first 300 ms is let by", Confirm.answer(ask, "y", 1299, 300),
            "early");
        holds("y after it answers yes", Confirm.answer(ask, "y", 1300, 300), "yes");
        holds("n, Escape and Ctrl+C answer no, as Vim's ask_yesno",
            ["n", "Escape", "Ctrl+c"].map(key => Confirm.answer(ask, key, 1400, 300)),
            ["no", "no", "no"]);
        holds("and any other key, Y among them, does nothing",
            ["Y", "d", "Return", "j"].map(key => Confirm.answer(ask, key, 1400, 300)),
            ["nothing", "nothing", "nothing", "nothing"]);
        holds("and no key at all, as a modifier alone, is let by",
            Confirm.answer(ask, "", 1400, 300), "early");
        pending(holds);
        visual(holds);
        changes(holds);

        // A place of 400 windows, each closed and copied by its actions:
        // the keys, the span and the actions over it of yG and of dG, each
        // timed as the fastest of twenty batches of fifty, since a check
        // shares its machine, in half a key's 4 ms: about 0.4 ms alone, and
        // past 1 ms while VM tests shared the machine (2026-11-04). And the
        // longest place the launcher lists, a folder of 1000 files, within
        // the key's 4 ms. A machine at a load of 40 to 70 took every one
        // past its bound, even the fastest batch (2026-11-05), so each is
        // held to its bound or to six walks over the same rows timed beside
        // it, whichever is longer: the walk slows as the machine does, and
        // yG and dG took one to four walks at a load of 40, while a span
        // done row by row over the place would take hundreds of walks.
        const made = { run, float, close, actions, copies, commands };
        operatorsTimed(holds, 400, 2, made);
        operatorsTimed(holds, 1000, 4, made);
    }

    // The key list in the launcher's own words (words.json): each section
    // named by its purpose, as the vi-paradigm spec names them, and each
    // chord at most 14, for the keys' column. A saying is as long as it
    // needs to be clear, wrapping at a word where it must, since the list
    // is for someone who forgets (VKCF3Z, 2026-11-06).
    function listedWords(holds) {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("words.json"), false);
        request.send();
        const listed = JSON.parse(request.responseText).field.listed;
        const list = Listed.keyList(Keys.table, listed.lines, listed.modes);
        holds("the key list's sections, by their purposes",
            list.filter(line => line.heading !== undefined).map(line => line.heading),
            ["Normal: moving and acting", "The find: the / line", "Commands: the : line",
                "Visual: choosing several", "Insert: writing words", "Drawer", "Choosing"]);
        const lines = list.filter(line => line.heading === undefined);
        holds("every line says what it does", lines.filter(line => line.value === "")
            .map(line => line.label), []);
        holds("and each chord in at most 14", [].concat(...lines.map(line => line.chords))
            .filter(chord => chord.length > 14), []);
    }

    // yG and dG over a place of `count` windows, made of `made`'s acts as
    // operators' rows are, each taking less than `under` ms.
    function operatorsTimed(holds, count, under, made) {
        const { run, float, close, actions, copies, commands } = made;
        const windows = [];
        for (let at = 0; at < count; at++) {
            const named = `Pull request #${at} \u00b7 Jitsusama/core.nix`;
            windows.push({ key: `window:${at}`, verb: "go", run,
                verbs: () => [float, { key: "verb:title", verb: "copy", copies: () => named,
                    run }, close] });
        }
        const all = { at: 0, highest: 0, last: count - 1, top: 0, bottom: 7, screen: 8 };
        const spanOf = operator => {
            let standing = Keys.rest();
            let last = null;
            for (const key of [operator, "G"]) {
                last = Keys.read(standing, key, "normal");
                standing = last.standing;
            }
            return Operators.spanned(last.count, last.motion, all);
        };
        // The least any operator does over the place: ask each row for its
        // actions and read its copy.
        const walk = () => {
            for (const window of windows)
                window.verbs()[1].copies();
        };
        // Twenty a batch and five batches each: the fastest of five is as
        // steady as of twenty, and a busy builder's slowing of a thousand
        // runs each took the whole case past its minute (80ECO2,
        // 2026-11-13, b5e5f9c5: it ended on its timeout just after this).
        const batch = work => {
            const began = Date.now();
            for (let at = 0; at < 20; at++)
                work();
            return (Date.now() - began) / 20;
        };
        // The fastest batch of `work` and of the walk, each batch of one
        // timed beside one of the other, so both see the same machine.
        const timed = work => {
            let fastest = Infinity;
            let walked = Infinity;
            for (let at = 0; at < 5; at++) {
                walked = Math.min(walked, batch(walk));
                fastest = Math.min(fastest, batch(work));
            }
            return { fastest, bound: Math.max(under, 6 * walked) };
        };
        const yanking = timed(() => Operators.copied(Operators.copiesIn(
            Operators.taken(windows, spanOf("y"), null), actions, copies).map(one => one.action)));
        const ending = timed(() => Operators.asks(Operators.endings(
            Operators.taken(windows, spanOf("d"), null), actions).map(one => one.action),
            commands));
        console.log(`yG over ${count} rows in ${yanking.fastest.toFixed(3)} ms of `
            + `${yanking.bound.toFixed(3)}, dG in ${ending.fastest.toFixed(3)} ms of `
            + `${ending.bound.toFixed(3)}`);
        holds(`yG over ${count} rows in under ${under} ms, or six walks on a busy machine`,
            yanking.fastest < yanking.bound, true);
        holds(`and dG over ${count}`, ending.fastest < ending.bound, true);
    }

    // An operator waiting for what it acts on, as the mode word's room says
    // it: the keys typed so far, and what the cursor row would take.
    function pending(holds) {
        const standing = keys => typed(["Escape", ...keys]).standing;
        holds("nothing pending at rest", Operators.pending(Keys.rest()), null);
        holds("nor with a count alone", Operators.pending(standing(["3"])), null);
        holds("the keys an operator waits after, and its count", ["d", "3d", "d2", "2d3",
            "dg", "y"].map(keys => Operators.pending(standing(keys.split(""))))
            .map(hit => `${hit.typed} ${hit.count}`),
        ["d 0", "3d 3", "d2 2", "2d3 6", "dg 0", "y 0"]);

        // The verb is the row's: its ending's for d, copy for y, or none
        // where it has nothing d or y takes.
        const words = { pending: "{typed} \u00b7 {what}", hit: "{verb} {name}",
            hits: "{verb} {count}", nothing: "nothing to end", noCopy: "nothing to copy" };
        const says = (typedSoFar, act, verb, count) => Operators.pendingSays({
            typed: typedSoFar, act, verb, name: "Home-5G", count }, words);
        holds("d says the cursor row's end", says("d", "ending", "disconnect", 0),
            "d \u00b7 disconnect Home-5G");
        holds("a count says how many it takes", says("3d", "ending", "close", 3),
            "3d \u00b7 close 3");
        holds("a count of one is the row", says("1d", "ending", "close", 1),
            "1d \u00b7 close Home-5G");
        holds("y says copy", says("y", "copy", "copy", 0), "y \u00b7 copy Home-5G");
        holds("a row d can't end says so", says("d", "ending", null, 0),
            "d \u00b7 nothing to end");
        holds("and one y can't copy", says("y", "copy", null, 0), "y \u00b7 nothing to copy");
    }

    // A paragraph of rows, as Vim's of lines (VKCF3Z, 2026-11-05): a run of
    // one kind, or of one sender's notices, ending at a gap or where that
    // changes. ip and ap take it, the same rows, since a list has no blank
    // rows; } and { move by it, and d} and d{ stop short of where they
    // land, as Vim's exclusive motions do.
    function paragraphs(holds) {
        const rows = [{ what: "window" }, { what: "window" },
            { what: "notice", sender: "Slack" }, { what: "notice", sender: "Slack" },
            { what: "notice", sender: "Mail" }, { what: "window" },
            { what: "window", gap: true }];
        const of = row => row.what === "notice" ? `notice:${row.sender}` : row.what;
        const runOf = at => Operators.runAt(rows, at, of);
        holds("a run of one kind", runOf(1), { from: 0, to: 1 });
        holds("one sender's notices", runOf(2), { from: 2, to: 3 });
        holds("another sender's apart", runOf(4), { from: 4, to: 4 });
        holds("and a gap ends one, the kind the same", [runOf(5), runOf(6)],
            [{ from: 5, to: 5 }, { from: 6, to: 6 }]);
        const place = at => ({ at, highest: 0, last: 6, top: 0, bottom: 6, screen: 8, runOf });
        const moved = (act, count, at) => Operators.moved(act, count, place(at));
        holds("} goes to the next run's first row", moved("paragraphOn", 0, 0), 2);
        holds("with a count", moved("paragraphOn", 2, 0), 4);
        holds("and stops at the last", moved("paragraphOn", 9, 3), 6);
        holds("{ to its own run's first", moved("paragraphBack", 0, 3), 2);
        holds("or the one before, from there", moved("paragraphBack", 0, 2), 0);
        holds("a place with no runs is one", Operators.moved("paragraphOn", 0,
            { at: 1, highest: 0, last: 6, top: 0, bottom: 6, screen: 8 }), 6);
        holds("{ and } repeat as moves", [Keys.repeats("paragraphOn"),
            Keys.repeats("paragraphBack")], [true, true]);
        const object = (act, count, at) => Operators.spanned(0, { act, count }, place(at));
        holds("dip takes the run", object("inParagraph", 0, 3), { from: 2, to: 3 });
        holds("and ap the same rows", object("aParagraph", 0, 3), { from: 2, to: 3 });
        holds("d2ip two runs", object("inParagraph", 2, 3), { from: 2, to: 4 });
        holds("d} up to the next run", object("paragraphOn", 0, 0), { from: 0, to: 1 });
        holds("and to the end from the last run", object("paragraphOn", 0, 6),
            { from: 6, to: 6 });
        holds("d{ back to its run's first, the cursor's row left",
            object("paragraphBack", 0, 3), { from: 2, to: 2 });
    }

    // Visual line, Vim's V over the rows: its keys, the rows it takes and
    // what Return over them says.
    // Changes to the rows, as Vim's to lines: Ctrl+R does again what u took
    // back, and . the last change with its count, or the count typed
    // before it in its place, each once a press. d's actions that say how
    // they're put back are what u takes back, and the foot says so.
    function changes(holds) {
        holds("Ctrl+R does again what u took back, and . the last change",
            typed(["Escape", "Ctrl+r", "3", "."]).counted, ["normal", "redoChange", "3 again"]);
        for (const act of ["undoChange", "redoChange", "again"])
            holds(`${act} is once a press`, Keys.repeats(act), false);
        const dd = { count: 2, motion: null };
        const dj = { count: 0, motion: { act: "next", count: 3 } };
        holds(". does the last change as it was", Operators.again(dd, 0), dd);
        holds("and a count before it stands in for its count", [Operators.again(dd, 4),
            Operators.again(dj, 4)], [{ count: 4, motion: null },
            { count: 0, motion: { act: "next", count: 4 } }]);
        const run = () => {};
        const back = () => {};
        holds("u takes back what d did that says how it's put back",
            Operators.undoable([{ run, undo: back }, { run }]), [{ redo: run, undo: back }]);
        const words = { done: "{count} done \u00b7 u brings them back",
            doneOne: "1 done \u00b7 u brings it back" };
        holds("and the foot says so until the next key",
            [Operators.doneSaid(3, words), Operators.doneSaid(1, words)],
            ["3 done \u00b7 u brings them back", "1 done \u00b7 u brings it back"]);
    }

    function visual(holds) {
        const doing = [
            ["V begins visual line", ["Escape", "V"], ["normal", "selectLines"], "visual"],
            ["V again ends it", ["Escape", "V", "V"], ["normal", "selectLines",
                "selectLines"], "normal"],
            ["and so does Escape", ["Escape", "V", "Escape"], ["normal", "selectLines",
                "normal"], "normal"],
            ["and Ctrl+C, as Vim's", ["Escape", "V", "Ctrl+c"], ["normal", "selectLines",
                "normal"], "normal"],
            ["Escape cancels a count first", ["Escape", "V", "3", "Escape"],
                ["normal", "selectLines"], "visual"],
            ["normal's moves extend it, with counts", ["Escape", "V", "j", "3", "k", "g", "g",
                "G", "Ctrl+d", "Down"], ["normal", "selectLines", "next", "3 previous", "first",
                "last", "halfDown", "next"], "visual"],
            ["d takes the selection at once, and ends it", ["Escape", "V", "j", "d"],
                ["normal", "selectLines", "next", "ending"], "normal"],
            ["and y", ["Escape", "V", "y"], ["normal", "selectLines", "copy"], "normal"],
            ["and Return", ["Escape", "V", "Return"], ["normal", "selectLines", "run"],
                "normal"],
            ["a colon opens the command line, the selection standing", ["Escape", "V", ":"],
                ["normal", "selectLines", "command"], "visual"],
            // Space leads in visual as in normal (the vi-paradigm spec's
            // Leader), so the foot's Space ? there is a key that works.
            ["and Space ? and F1 list the keys", ["Escape", "V", "Space", "?", "F1"],
                ["normal", "selectLines", "keys", "keys"], "visual"],
            ["nothing types, and i a and A begin nothing", ["Escape", "V", "i", "a", "A"],
                ["normal", "selectLines"], "visual"],
            ["on the find, V types", ["V"], [], "find"],
            // On a list v is V, as the spec has it: rows have no letters
            // to choose among, so either begins visual line and ends it.
            ["v begins visual line on the rows", ["Escape", "v"], ["normal", "selectLines"],
                "visual"],
            ["and v or V ends it", ["Escape", "v", "V", "V", "v"], ["normal", "selectLines",
                "selectLines", "selectLines", "selectLines"], "normal"],
            ["x takes the selection as d does", ["Escape", "V", "j", "x"],
                ["normal", "selectLines", "next", "ending"], "normal"],
            ["but in normal x is nothing on the rows", ["Escape", "x"], ["normal"], "normal"],
            ["o goes to its other end", ["Escape", "V", "j", "o"],
                ["normal", "selectLines", "next", "otherEnd"], "visual"],
            ["gv brings the last back", ["Escape", "g", "v"], ["normal", "reselect"],
                "visual"],
            ["ip takes the run the cursor's in", ["Escape", "V", "i", "p"],
                ["normal", "selectLines", "inParagraph"], "visual"],
            ["and ap", ["Escape", "V", "a", "p"],
                ["normal", "selectLines", "aParagraph"], "visual"],
            ["dip and yap are an operator over them", ["Escape", "d", "i", "p", "y", "a",
                "p"], ["normal", "ending inParagraph", "copy aParagraph"], "normal"],
            ["with a count", ["Escape", "2", "d", "i", "p"], ["normal", "ending 2 inParagraph"],
                "normal"],
            ["while i alone after d lets it go", ["Escape", "d", "i", "j"], ["normal"],
                "normal"],
            ["} and { move by runs", ["Escape", "}", "{"], ["normal", "paragraphOn",
                "paragraphBack"], "normal"],
            ["and extend visual", ["Escape", "V", "}"], ["normal", "selectLines",
                "paragraphOn"], "visual"],
            ["and are d's motions", ["Escape", "d", "}"], ["normal", "ending paragraphOn"],
                "normal"]
        ];
        for (const [what, keys, want, mode] of doing) {
            const got = typed(keys);
            holds(what, [got.counted, got.mode], [want, mode]);
        }
        holds("a letter in visual is the line's, never typed",
            typed(["Escape", "V", "x"]).ours, true);
        holds("d in visual waits for no motion",
            typed(["Escape", "V", "d"]).standing.operator, undefined);
        holds("V is once a press", Keys.repeats("selectLines"), false);
        holds("and so are o, gv, ip and ap", ["otherEnd", "reselect", "inParagraph",
            "aParagraph"].some(act => Keys.repeats(act)), false);
        holds("the selection ends on x as on d", Keys.takesSelection("ending"), true);
        // The operator pressed again within 300 ms of visual's is taken
        // with it, so V j j d d takes three and never two more: the same
        // operator's key, d or y, never x, which is no operator.
        const visualD = { key: "d", at: 1000 };
        holds("d again within 300 ms is taken with visual's",
            Operators.takenWith(visualD, "d", 1299, 300), true);
        holds("but not from 300 ms", Operators.takenWith(visualD, "d", 1300, 300), false);
        holds("nor another key", Operators.takenWith(visualD, "y", 1100, 300), false);
        holds("nor after x",
            Operators.takenWith({ key: "x", at: 1000 }, "x", 1100, 300), false);
        holds("nor with none before", Operators.takenWith(null, "d", 1100, 300), false);
        paragraphs(holds);
        holds("the command line takes the selection's range", [Operators.rangeFor(0, "visual"),
            Operators.rangeFor(3, "visual"), Operators.rangeFor(3, "normal"),
            Operators.rangeFor(0, "normal")],
        ["'<,'>", "'<,'>", ".,.+2", ""]);

        // The rows the selection takes, from where it began to the cursor,
        // either way; none where it hasn't begun.
        holds("down from where it began", Operators.selection(2, 5), { from: 2, to: 5 });
        holds("or up", Operators.selection(5, 2), { from: 2, to: 5 });
        holds("or the one row", Operators.selection(4, 4), { from: 4, to: 4 });
        holds("and none before it begins", Operators.selection(-1, 3), { from: 0, to: -1 });
        // Where it began is held by that row, as the cursor is, so rows
        // arriving or leaving above it never move what it takes.
        const named = keys => keys.map(key => ({ key }));
        const keyOf = row => row.key;
        holds("a row arriving above keeps it on the row it began on",
            Operators.heldFrom(named(["new", "a", "b", "c"]), "b", 1, keyOf), 2);
        holds("and one leaving above", Operators.heldFrom(named(["b", "c"]), "b", 1, keyOf), 0);
        holds("its row gone, it stays where it was, within the rows",
            [Operators.heldFrom(named(["a", "c"]), "b", 1, keyOf),
                Operators.heldFrom(named(["a"]), "b", 1, keyOf)], [1, 0]);
        holds("and none begun stays none", Operators.heldFrom(named(["a"]), "", -1, keyOf), -1);
        // gv's last selection is held by its rows too, both ends, so rows
        // changing between choosing and gv never hand d other rows.
        const chosen = Operators.marked(named(["a", "b", "c"]), 1, 2, keyOf);
        holds("gv brings back the rows chosen, a row having arrived above them",
            Operators.reselected(named(["new", "a", "b", "c"]), chosen, keyOf),
            { from: 2, at: 3 });
        holds("and where one has gone, what's left of them",
            Operators.reselected(named(["a", "c"]), chosen, keyOf), { from: 1, at: 1 });

        // Return over the selection opens the rows it can, and says how
        // many of how many where it couldn't open them all.
        const words = { opens: "open {count}", opened: "{open} of {count} open" };
        holds("the foot says how many Return opens", Operators.opensSaid(3, words), "open 3");
        holds("and what it opened, where not all", Operators.openedSaid(2, 3, words),
            "2 of 3 open");
        holds("and nothing where it opened every one", Operators.openedSaid(3, 3, words), "");
        // The mode's word says only what it's doing, now the mark names the
        // mode: how many are chosen, "3 chosen", the one row among them.
        const counting = { counted: "{count} chosen" };
        holds("the mode word counts what's chosen",
            [Operators.chosenSaid(3, counting), Operators.chosenSaid(1, counting)],
            ["3 chosen", "1 chosen"]);
        // Each mode in three capitals at the foot's start (Joel, 2026-11-05:
        // "the mode should be 3 letters, in caps"): the find line, insert,
        // visual on the rows or the drawer's page, the command line whatever
        // mode : came from, and normal for the rest, the drawer's page too.
        holds("each mode is named by its mark",
            ["find", "insert", "normal", "page", "visual", "pageVisual", "pageVisualLine"]
                .map(mode => ModeMarks.mark(mode, false)),
            ["FND", "INS", "NOR", "NOR", "VIS", "VIS", "VIS"]);
        holds("and the command line by its own", [ModeMarks.mark("normal", true),
            ModeMarks.mark("page", true), ModeMarks.mark("insert", true)],
            ["CMD", "CMD", "CMD"]);
        holds("every mark is three capitals", ["find", "insert", "normal", "visual"]
            .map(mode => ModeMarks.mark(mode, false)).concat([ModeMarks.mark("normal", true)])
            .every(said => /^[A-Z]{3}$/.test(said)), true);
    }

    // The keys typed from a line's opening in insert, on a prompt's words.
    function wordsTyped(keys) {
        let standing = Keys.rest();
        let mode = "insert";
        const acts = [];
        for (const key of keys) {
            const read = Keys.read(standing, key, mode, true);
            standing = read.standing;
            mode = Keys.modeAfter(read.act, mode, "insert");
            if (read.act !== "")
                acts.push(read.count > 0 ? `${read.count} ${read.act}` : read.act);
        }
        return { acts, mode };
    }

    // A prompt's words edited in normal, as Vim edits a line, and Ctrl+H in
    // insert, on every line.
    function editing(holds) {
        // [what, the keys typed from the opening, the acts done]
        const doing = [
            ["w b e move by words", ["Escape", "w", "b", "e"],
                ["normal", "wordNext", "wordBack", "wordEnd"]],
            ["h l move by letters, and so do the arrows", ["Escape", "h", "l", "Left", "Right"],
                ["normal", "letterBack", "letterNext", "letterBack", "letterNext"]],
            // Backspace is h, as Vim's is; - still goes up, as his oil's.
            ["Backspace moves back a letter, as h, and - goes up", ["Escape", "Backspace", "-"],
                ["normal", "letterBack", "out"]],
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
        holds("on the rows, u takes back what d did, never the find",
            typed(["Escape", "u"]).acts, ["normal", "undoChange"]);
        holds("and w and x do nothing", typed(["Escape", "w", "x"]).acts, ["normal"]);
        holds("Ctrl+H deletes back in insert", typed(["Ctrl+h"]).acts, ["deleteBack"]);
        for (const act of ["wordNext", "wordBack", "wordEnd"])
            holds(`${act} repeats while held`, Keys.repeats(act), true);
        holds("x is once a press", Keys.repeats("deleteLetter"), false);
        // A held key's repeat that does nothing takes no key, so a Return
        // held on Reboot leaves it armed, rather than letting it go and
        // making the next Return arm it again (Y2YBTS, 2026-11-12).
        for (const mode of ["insert", "find", "normal", "visual", "page"])
            holds(`a held Return does nothing more in ${mode}`, Keys.heldDoes("run", mode), false);
        holds("a held j moves on in normal", Keys.heldDoes("next", "normal"), true);
        holds("a held x does nothing more", Keys.heldDoes("deleteLetter", "normal"), false);
        holds("a held Backspace deletes on in insert", Keys.heldDoes("deleteBack", "insert"),
            true);
        holds("and a held left arrow moves on", Keys.heldDoes("caretLeft", "find"), true);

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

    // The drawer as a window: Vim's window keys go into it from the rows,
    // where its keys are a page's in normal, Ctrl+W l to the window right,
    // and Ctrl+W h, ZZ or :q come back, Ctrl+W w going either way. No arrow
    // goes in or out of it; inside, each is its letter's move and nothing
    // else, as in a buffer (Joel, 2026-11-05: "In normal mode, in a buffer,
    // arrows move the cursor"). q alone only says how to close there, as
    // Joel asked of every modal surface, and Escape at rest does nothing in
    // a window (2026-11-04, VKCF3Z's spec v2).
    function drawerKeys(holds) {
        const into = ["Escape", "Ctrl+w", "l"];
        // [what, the keys typed from the opening, the acts done]
        const doing = [
            ["Ctrl+W l goes into the drawer, as Vim goes to the window right", into,
                ["normal", "drawer"]],
            // His Neovim's Space and an arrow overlay them, as a leader's
            // chord may (Joel, 2026-11-05: "leader stuff can overlay
            // default stuff").
            ["Space and the right arrow go in, and Space and the left come back",
                ["Escape", "Space", "Right", "Space", "Left", "j"],
                ["normal", "drawer", "rows", "next"]],
            ["and Ctrl+W h comes back to the rows", into.concat(["Ctrl+w", "h"]),
                ["normal", "drawer", "rows"]],
            ["Ctrl+W w goes to the other either way",
                ["Escape", "Ctrl+w", "w", "Ctrl+w", "w"], ["normal", "drawer", "rows"]],
            ["Ctrl+W h does nothing on the rows, nor Ctrl+W l in the drawer",
                into.concat(["Ctrl+w", "l", "Ctrl+w", "h", "Ctrl+w", "h", "j"]),
                ["normal", "drawer", "rows", "next"]],
            ["and so does ZZ, as his Neovim closes a window, where q only says how",
                into.concat(["q", "Z", "Z"]), ["normal", "drawer", "quitSaid", "rows"]],
            ["where Escape at rest does nothing", into.concat(["Escape", "j"]),
                ["normal", "drawer", "next"]],
            ["Escape cancels a count, and the keys stay",
                into.concat(["3", "Escape", "Escape", "j"]), ["normal", "drawer", "next"]],
            ["and a g waiting", into.concat(["g", "Escape", "Escape", "j"]),
                ["normal", "drawer", "next"]],
            ["j k move its cursor a line, and Down and Up as they do",
                into.concat(["j", "k", "Down", "Up"]),
                ["normal", "drawer", "next", "previous", "next", "previous"]],
            ["h l move it along its line, and Left and Right as they do",
                into.concat(["h", "l", "Left", "Right"]),
                ["normal", "drawer", "left", "right", "left", "right"]],
            ["Ctrl+E and Ctrl+Y scroll it a line", into.concat(["Ctrl+e", "Ctrl+y"]),
                ["normal", "drawer", "scrollDown", "scrollUp"]],
            ["Ctrl+D Ctrl+U go half its view, Ctrl+F Ctrl+B and the page keys a view",
                into.concat(["Ctrl+d", "Ctrl+u", "Ctrl+f", "Ctrl+b", "PageDown", "PageUp"]),
                ["normal", "drawer", "halfDown", "halfUp", "pageDown", "pageUp", "pageDown",
                    "pageUp"]],
            ["gg and G go to its ends", into.concat(["g", "g", "G"]),
                ["normal", "drawer", "first", "last"]],
            ["H M L go to the top, middle and bottom of its view",
                into.concat(["H", "M", "L"]),
                ["normal", "drawer", "viewTop", "viewMiddle", "viewBottom"]],
            ["the arrows with Shift do nothing in it, as nowhere",
                into.concat(["Shift+Down", "Shift+Up"]), ["normal", "drawer"]],
            ["Return does the row from it", into.concat(["Return"]),
                ["normal", "drawer", "run"]],
            [": opens the command line, whose :q comes back to the rows", into.concat([":"]),
                ["normal", "drawer", "command"]],
            ["the rows' keys that aren't a page's do nothing in it",
                into.concat(["Tab", "-", "Backspace", "u", "]", "b"]),
                ["normal", "drawer"]],
            ["Ctrl+W l does nothing more in it, nor Space and the right arrow",
                into.concat(["Ctrl+w", "l", "Space", "Right"]), ["normal", "drawer"]],
            // Space ? is his leader's list of keys everywhere (VKCF3Z), so
            // in the drawer it lists the drawer's, and F1 with it.
            ["Space ? and F1 list its keys", into.concat(["Space", "?", "F1"]),
                ["normal", "drawer", "keys", "keys"]],
            ["but not while choosing, where ? searches back",
                into.concat(["v", "Space", "?"]), ["normal", "drawer", "select"]],
            ["Ctrl+W h does nothing on the rows",
                ["Escape", "Ctrl+w", "h", "j"], ["normal", "next"]],
            ["Space then anything else lets Space go", ["Escape", "Space", "j", "j"],
                ["normal", "next"]],
            ["Space and the arrows in insert type and move the caret",
                ["Space", "Right", "Space", "Left"], ["caretRight", "caretLeft"]]
        ];
        for (const [what, keys, want] of doing)
            holds(`the drawer: ${what}`, typed(keys).acts, want);
        holds("the drawer: Space waits for its second key", typed(["Escape", "Space"]).standing,
            { count: "", typed: ["Space"] });
        holds("the drawer: Space in insert is the field's to type", typed(["Space"]).ours, false);
        holds("the drawer: its keys are the page's", typed(into).mode, "page");
        holds("the drawer: and they stay the page's as its keys are listed",
            typed(into.concat(["Space", "?"])).mode, "page");
        holds("the drawer: and the rows' again once back",
            typed(into.concat(["Ctrl+w", "h"])).mode, "normal");
        holds("the drawer: a letter in it is its own, typing nothing",
            typed(into.concat(["x"])).ours, true);
        // [what, the keys typed from the opening, each act with its count]
        const counting = [
            ["a count moves its cursor that many lines", into.concat(["3", "j", "2", "k"]),
                ["normal", "drawer", "3 next", "2 previous"]],
            ["and that many letters", into.concat(["4", "l", "2", "h"]),
                ["normal", "drawer", "4 right", "2 left"]],
            ["an arrow takes a count, as its letter does",
                into.concat(["3", "Down", "2", "Right"]),
                ["normal", "drawer", "3 next", "2 right"]],
            ["a count before G names the line", into.concat(["1", "2", "G"]),
                ["normal", "drawer", "12 last"]],
            ["a count before the scrolls", into.concat(["2", "Ctrl+e", "3", "Ctrl+d"]),
                ["normal", "drawer", "2 scrollDown", "3 halfDown"]],
            ["a count before a colon is let go, since its lines aren't rows",
                into.concat(["3", ":"]), ["normal", "drawer", "command"]]
        ];
        for (const [what, keys, want] of counting)
            holds(`the drawer: ${what}`, typed(keys).counted, want);
        for (const act of ["left", "right", "scrollDown", "scrollUp"])
            holds(`the drawer: ${act} repeats while held`, Keys.repeats(act), true);
        for (const act of ["drawer", "rows"])
            holds(`the drawer: ${act} is once a press`, Keys.repeats(act), false);
    }

    // The drawer's search, as Vim searches a page (VKCF3Z's spec v2, The
    // Reader): / and ? type what to seek on the line, as : types a
    // command there, and Return runs it, putting the cursor where it
    // lands; n and N go on to the next match the way it went and the
    // other. It moves the cursor and narrows nothing, so it's a page's
    // search, never the rows' find.
    function drawerSearch(holds) {
        const into = ["Escape", "Ctrl+w", "l"];
        // [what, the keys typed from going in, the acts done, what's sought]
        const doing = [
            ["/ seeks forward and Return runs it", ["/", "d", "e", "Return"], ["seek"],
                [{ pattern: "de", forward: true }]],
            ["? seeks back", ["?", "d", "Return"], ["seek"],
                [{ pattern: "d", forward: false }]],
            ["a page's keys typed in a search are the search's",
                ["/", "j", "q", "G", ":", "n", "Return"], ["seek"],
                [{ pattern: "jqG:n", forward: true }]],
            ["Space types a space", ["/", "a", "Space", "b", "Return"], ["seek"],
                [{ pattern: "a b", forward: true }]],
            ["Backspace takes the last letter", ["/", "a", "b", "Backspace", "Return"],
                ["seek"], [{ pattern: "a", forward: true }]],
            ["Escape lets it go, the keys staying in the drawer", ["/", "a", "Escape", "j"],
                ["next"], []],
            ["and so does Backspace with nothing left", ["/", "Backspace", "j"], ["next"],
                []],
            // Vim's command line, as the find line and the command line edit
            // theirs (LineEditing.js).
            ["and so does Ctrl+C, as Vim's cancels", ["/", "a", "Ctrl+c", "j"], ["next"],
                []],
            ["Ctrl+W takes the word before",
                ["/", "a", "Space", "b", "c", "Ctrl+w", "d", "Return"], ["seek"],
                [{ pattern: "a d", forward: true }]],
            ["Ctrl+U takes all of it", ["/", "a", "b", "Ctrl+u", "c", "Return"], ["seek"],
                [{ pattern: "c", forward: true }]],
            ["Ctrl+H takes the last letter", ["/", "a", "b", "Ctrl+h", "Return"], ["seek"],
                [{ pattern: "a", forward: true }]],
            ["and lets it go with nothing left", ["/", "Ctrl+h", "j"], ["next"], []],
            ["n and N go on to the next match, either way", ["n", "N"],
                ["seekNext", "seekPrevious"], []]
        ];
        for (const [what, keys, acts, sought] of doing) {
            const done = typed(into.concat(keys));
            holds(`the drawer's search: ${what}`, done.acts, ["normal", "drawer"].concat(acts));
            holds(`the drawer's search: ${what}, seeking`, done.sought, sought);
        }
        holds("the drawer's search: a count goes before / and n",
            typed(into.concat(["2", "/", "x", "Return", "3", "n", "4", "N"])).counted,
            ["normal", "drawer", "2 seek", "3 seekNext", "4 seekPrevious"]);
        holds("the drawer's search: while typed, the line holds it",
            typed(into.concat(["?", "a", "b"])).standing.seek, { pattern: "ab", forward: false });
        holds("the drawer's search: a letter in it is its own",
            typed(into.concat(["/", "x"])).ours, true);
        holds("the drawer's search: and it leaves the keys in the drawer",
            typed(into.concat(["/", "x", "Return"])).mode, "page");
        for (const act of ["seekNext", "seekPrevious"])
            holds(`the drawer's search: ${act} repeats while held`, Keys.repeats(act), true);
        holds("the drawer's search: seek is once a press", Keys.repeats("seek"), false);
        // za opens or folds the drawer's nerdy bits, as Vim's za a fold, once
        // a press, in the drawer and on the rows alike.
        holds("the drawer: za opens or folds its nerdy bits", typed(into.concat(["z", "a"])).acts,
            ["normal", "drawer", "fold"]);
        holds("the drawer: za twice folds them back",
            typed(into.concat(["z", "a", "z", "a"])).acts, ["normal", "drawer", "fold", "fold"]);
        holds("the drawer: a fold is once a press", Keys.repeats("fold"), false);
        holds("the rows: za opens or folds the drawer's nerdy bits too",
            typed(["Escape", "z", "a"]).acts, ["normal", "fold"]);
        holds("the drawer's search: / on the rows is no page's search",
            typed(["Escape", "/", "x", "Return"]).acts.includes("seek"), false);

        // Where a search lands (DrawerPage.sought): lines of the pitch, the
        // view ten of them, and the cursor's line and letter. [what, from
        // line, from letter, pattern, forward, count, [line, letter, the
        // end it went round, as Vim's wrapscan says it hit the bottom or
        // the top]], or null where it's found nowhere.
        const texts = ["Deploy log", "line 1 of the deploy", "line 2 of the Deploy",
            "nothing here"].concat(Array.from({ length: 16 }, (_, at) => `filler ${at}`));
        const lines = texts.map((text, at) => ({ top: at * 24, height: 24, length: text.length,
            text }));
        const page = (line, letter) => ({
            lines, line, letter, want: letter * 10, top: 0, height: 240, end: 240, pitch: 24,
            off: 4, xOf: (at, along) => along * 10,
            letterAt: (at, x) => Math.round(x / 10)
        });
        const landings = [
            ["the next match after the cursor", 0, 0, "deploy", true, 0, [1, 14, ""]],
            ["a count goes on that many", 0, 0, "deploy", true, 2, [2, 14, ""]],
            ["past the last, round to the first", 0, 0, "deploy", true, 3, [0, 0, "bottom"]],
            ["a capital finds only itself", 0, 0, "Deploy", true, 0, [2, 14, ""]],
            ["on the cursor's own line, further along", 1, 0, "the", true, 0, [1, 10, ""]],
            ["never where the cursor stands", 1, 10, "the", true, 0, [2, 10, ""]],
            ["? the one before", 1, 14, "the", false, 0, [1, 10, ""]],
            ["? from the first, round to the last", 0, 0, "deploy", false, 0, [2, 14, "top"]],
            ["? a count round past the first", 2, 14, "deploy", false, 3, [2, 14, "top"]],
            ["a space is a letter like any", 0, 0, "1 of", true, 0, [1, 5, ""]]
        ];
        const where = landed => landed === null ? null
            : [landed.line, landed.letter, landed.wrapped];
        for (const [what, line, letter, pattern, forward, count, want] of landings)
            holds(`the drawer's search lands: ${what}`,
                where(Page.sought(page(line, letter), pattern, forward, count)), want);
        holds("the drawer's search lands nowhere it isn't",
            Page.sought(page(0, 0), "zzz", true, 0), null);
        holds("or for nothing sought", Page.sought(page(0, 0), "", true, 0), null);
        const far = Page.sought(page(0, 0), "filler 8", true, 0);
        holds("the view follows it, keeping four lines below", [far.line, far.top], [12, 168]);
        holds("and its letter is where it wants to be", far.want, 0);
        // Which match it lands on, of how many, as the foot counts them
        // ("/deploy \u00b7 2 of 3"), going round too.
        const counted = landed => [landed.at, landed.of];
        holds("the drawer's search counts the match it lands on",
            counted(Page.sought(page(0, 0), "deploy", true, 0)), [2, 3]);
        holds("and counts round past the last",
            counted(Page.sought(page(0, 0), "deploy", true, 3)), [1, 3]);
        holds("and back", counted(Page.sought(page(0, 0), "deploy", false, 0)), [3, 3]);
        // Where the cursor stands on a match, which of them it is, else null.
        holds("the cursor on a match says which", Page.found(lines, "deploy", 2, 14),
            { at: 3, of: 3 });
        holds("the title's own counts first", Page.found(lines, "deploy", 0, 0),
            { at: 1, of: 3 });
        holds("the cursor off a match says none", Page.found(lines, "deploy", 1, 0), null);
        holds("nor where nothing's sought", Page.found(lines, "", 0, 0), null);
    }

    // What would change the drawer, which a page can't be: Vim's E21, as
    // his Neovim says it on a buffer that can't be modified (VKCF3Z's spec
    // v2, The Reader: "d c p x say E21: Cannot make changes"). x, p and S
    // say it at once, and so does every other key Vim's nv_edit refuses on
    // such a buffer, i a I A o O among them, entering no insert (Y2YBTS);
    // d and c wait for their motion first, as Vim's do, a count, a text
    // object's i or a and a g going before it, and r for its letter, any
    // digit being one; Escape or Ctrl+C lets them go unsaid. q says how to
    // close, as on the rows.
    // The face each fact's value is set in: an identifier (a node, a serial,
    // a bus name) in the Name face, as the drawer's last facts are (VKCF3Z,
    // 2026-11-07); a sentence in the words' face; Latin on a script's plate
    // in the theme's own; and the rest at Figure.
    function drawerFaces(holds) {
        const cases = [
            ["an identifier", { identifies: true }, "", "name"],
            ["an identifier on a script's plate", { identifies: true }, "hebrew", "name"],
            ["a sentence", { words: true }, "", "words"],
            ["Latin on a script's plate", { latin: true }, "hebrew", "latin"],
            ["Latin on the theme's own plate", { latin: true }, "", "value"],
            ["a figure", {}, "", "value"],
            ["no fact", null, "", "value"]
        ];
        for (const [what, fact, script, want] of cases)
            holds(`a drawer sets ${what} in its face`, Page.faceOf(fact, script), want);
    }

    // A value given as items laid on lines no wider than the drawer's
    // measure, each letter ten wide here: wrapped between items, never
    // inside one, the dot ending the line it leaves and measured with it; a
    // path too wide for a line of its own giving way from its middle; any
    // other item from its own (VKCF3Z, 2026-11-08: nothing draws past its
    // frame).
    function drawerValueLines(holds) {
        const width = text => text.length * 10;
        const disk = [
            { said: "/nix +9.66 GB", path: "/nix" },
            { said: "/var +537 MB", path: "/var" }
        ];
        const firefox = "/run/current-system/sw/bin/firefox";
        const command = [{ said: firefox, path: firefox }, { said: "--private-window" }];
        const mpv = ["mpv", "--player-operation-mode=pseudo-gui", "--"]
            .map(said => ({ said, whole: true }));
        const store = "/nix/store/abc/def";
        const cases = [
            ["items that fit share a line", disk, " \u00b7 ", 400,
                ["/nix +9.66 GB \u00b7 /var +537 MB"]],
            ["items too wide together wrap between them, the dot going with the break",
                disk, " \u00b7 ", 150, ["/nix +9.66 GB", "/var +537 MB"]],
            ["an item that fits its line whole needs no room for a dot",
                disk, " \u00b7 ", 130, ["/nix +9.66 GB", "/var +537 MB"]],
            ["no line ends on a dot, however short", [{ said: "on" }, { said: "/var +537 MB" }],
                " \u00b7 ", 120, ["on", "/var +537 MB"]],
            ["a short last item brings the one before it down, leaving no dot behind",
                ["ab", "cdef", "j"].map(said => ({ said, whole: true })), " \u00b7 ", 90,
                ["ab", "cdef \u00b7 j"]],
            ["a command's arguments wrap at the space between them, a path from its middle",
                command, " ", 200, ["/\u2026/firefox", "--private-window"]],
            ["a path inside an item gives way and leaves the rest whole",
                [{ said: `${store} +9.66 GB`, path: store }], " \u00b7 ", 150,
                ["/\u2026/def +9.66 GB"]],
            ["an item naming no path gives way from its own middle",
                [{ said: "--a-very-long-argument" }], " ", 100, ["--a-v\u2026ment"]],
            ["a path still too wide in its shortest form gives way from its middle too",
                [{ said: firefox, path: firefox }], " ", 80, ["\u2026/fi\u2026fox"]],
            ["an empty item is left out", [{ said: "" }, { said: "on" }], " ", 100, ["on"]],
            ["an identifier wraps at its spaces first, shown whole", mpv, " ", 400,
                ["mpv", "--player-operation-mode=pseudo-gui --"]],
            ["a short last argument never stands alone", mpv, " ", 340,
                ["mpv", "--player-operation-mode=", "pseudo-gui --"]],
            ["one too long alone breaks at its = before its hyphens", mpv, " ", 240,
                ["mpv", "--player-operation-mode=", "pseudo-gui --"]],
            ["and at its hyphens where a piece is still too long", mpv, " ", 160,
                ["mpv", "--player-", "operation-mode=", "pseudo-gui --"]],
            ["a short last argument brings the one before it down",
                ["mpv", "-v", "--"].map(said => ({ said, whole: true })), " ", 60,
                ["mpv", "-v --"]],
            ["one never breaks at the marks it opens with to keep it company",
                ["mpv", "--x", "--"].map(said => ({ said, whole: true })), " ", 40,
                ["mpv", "--x", "--"]],
            ["a whole path breaks after its slashes, nothing left out",
                [{ said: firefox, whole: true }], " ", 200,
                ["/run/current-system/", "sw/bin/firefox"]],
            ["an emoji in its middle is kept whole or left out, never split",
                [{ said: "ab\ud83c\udf89cdefgh" }], " ", 60, ["ab\u2026gh"]]
        ];
        for (const [what, items, between, wide, want] of cases)
            holds(`a drawer's value: ${what}`, Value.valueLines(items, between, wide, width), want);
        // An identifier's value, as a program's Desktop ID: whole where it
        // fits, else broken at its own marks, a dot among them but never a
        // figure's (VKCF3Z, 2026-11-09; Y2YBTS: "only an identifier's
        // dots").
        const desktop = "com.onepassword.OnePassword";
        const identifiers = [
            ["one that fits is whole", desktop, 300, [desktop]],
            ["one too wide breaks after its dots", desktop, 200,
                ["com.onepassword.", "OnePassword"]],
            ["a version's dots are a figure's, never a break", "linux-6.12.1", 70,
                ["linux-", "6.12.1"]],
            ["nor a figure's like 0.50", "mpv-0.50.0", 60, ["mpv-", "0.50.0"]],
            ["a run of them wraps between its items, the dot going with the break",
                "usb-1d50:6122 \u00b7 ABC123", 150, ["usb-1d50:6122", "ABC123"]],
            ["one with spaces breaks at them, ending on none", "Ploopy Adept Trackball", 130,
                ["Ploopy Adept", "Trackball"]],
            // No line after the first opens with a mark (Y2YBTS, 2026-11-09).
            ["a run of marks stays with the piece before", "fe80::8a2e:370:7334", 60,
                ["fe80::", "8a2e:", "370:", "7334"]],
            ["a doubled slash too", "/srv//bar", 60, ["/srv//", "bar"]],
            ["and a doubled space, which goes", "Ploopy  Adept", 70, ["Ploopy", "Adept"]],
            ["a word as wide as the line is whole, its space going", "Ploopy Adept", 60,
                ["Ploopy", "Adept"]],
            ["a version too long breaks at its figures' dots, never opening on one",
                "v1.2.3.4.5-rc1", 60, ["v1.2.", "3.4.5-", "rc1"]],
            ["an address breaks between its numbers, never inside one", "192.168.100.200", 60,
                ["192.", "168.", "100.", "200"]],
            ["one broken by its letters keeps each mark with the letter before",
                "abcdefgh.ij", 50, ["abcde", "fgh.", "ij"]],
            ["a weak mark after a strong one stays with it", "LDFLAGS=-lfoo", 80,
                ["LDFLAG", "S=-lfoo"]],
            ["an option's value its dashes too", "--gtk-options=--verbose", 160,
                ["--gtk-options=--", "verbose"]],
            ["after a space a mark opens the next piece, as an argument's do", "-a --verbose",
                90, ["-a", "--verbose"]],
            ["a run of marks alone still breaks, never past the frame", "::::::::", 30,
                [":::", ":::", "::"]]
        ];
        for (const [what, said, wide, want] of identifiers)
            holds(`a drawer's identifier: ${what}`, Value.identifierLines(said, wide, width), want);
        // Whether an identifier's lines, as drawn, open on no mark but after
        // a space, which the drawer reports as valuesWhole (Y2YBTS,
        // 2026-11-09).
        const laid = [
            ["lines opening on letters", "fe80::8a2e", ["fe80::", "8a2e"], true],
            ["a line opening on a mark", "LDFLAGS=-lfoo", ["LDFLAGS=", "-lfoo"], false],
            ["an argument after a space", "mpv --player-operation-mode=pseudo-gui",
                ["mpv", "--player-operation-mode=", "pseudo-gui"], true],
            ["an item after a run's dot", "usb-1d50 \u00b7 -x", ["usb-1d50", "-x"], true],
            ["kept spaces read as spaces", "a b --c", ["a\u00a0b", "--c"], true]
        ];
        for (const [what, said, lines, want] of laid)
            holds(`an identifier laid: ${what}`, Identifier.laidWell(said, lines), want);
        // The words an identifier is read as, each to stand whole, so a
        // UUID broken at its hyphens reads as whole (RTOIMQ, 2026-11-09).
        const hash = "0c8vy4kd2hxq7rm5ppa6f3jz1nbsw9li";
        const read = [
            ["a UUID's are between its hyphens", "0f8fad5b-d9cb-469f-a165-70867728950e",
                ["0f8fad5b-", "d9cb-", "469f-", "a165-", "70867728950e"]],
            ["a desktop entry's are between its dots", desktop,
                ["com.", "onepassword.", "OnePassword"]],
            ["a figure's dot is inside its word", "6.12", ["6.12"]],
            ["a run of marks ends the word before", "fe80::8a2e", ["fe80::", "8a2e"]],
            ["spaces end words, kept by none", "Ploopy Adept", ["Ploopy", "Adept"]],
            // A hash has no words, so it may break at any letter (VKCF3Z,
            // 2026-11-09), its mark kept with its last.
            ["a hash's are its letters", `/nix/store/${hash}-mpv`,
                ["/nix/", "store/"].concat([...hash.slice(0, -1)], ["i-", "mpv"])],
            ["a long word of letters alone is still a word",
                "org.fwupd.RollbackProtectionEnabled",
                ["org.", "fwupd.", "RollbackProtectionEnabled"]],
            ["a short run with digits is still a word", "usb-1d50abc", ["usb-", "1d50abc"]]
        ];
        for (const [what, said, want] of read)
            holds(`an identifier's words: ${what}`, Value.identifierWords(said), want);
        // A title breaks at its spaces, and inside a word only where the word
        // is too wide for a line alone: "10.12.03.svg" stood split at a dot
        // beside a space at 2 (RTOIMQ, 2026-11-09).
        const titles = [
            ["a word that fits keeps its dots", "Drawing 2026-11-09 10.12.03.svg", 200,
                "Drawing 2026-11-09 10.12.03.svg"],
            ["a word too wide breaks at its dots", "Server time.harbour.test", 100,
                "Server time.\u200bharbour.\u200btest"],
            ["a name with no space breaks at its marks", "/srv/files_kept", 100,
                "/\u200bsrv/\u200bfiles_\u200bkept"],
            ["an empty title stays empty", "", 100, ""],
            ["a word as wide as the line keeps its dots", "Server a.b.c.d.e.f", 110,
                "Server a.b.c.d.e.f"],
            ["a letter more breaks it", "Server a.b.c.d.e.fg", 110,
                "Server a.\u200bb.\u200bc.\u200bd.\u200be.\u200bfg"],
            ["spaces running stay as they are", "Two  a.b", 100, "Two  a.b"],
            ["a tab ends a word too", "Tab\tfile.name.txt", 140, "Tab\tfile.name.txt"]
        ];
        for (const [what, said, room, want] of titles)
            holds(`a title's breaks: ${what}`, Identifier.titleBreaks(said, room, width), want);
        // A title is one line where it fits, else two balanced, never one
        // word alone below where it could share, and past two it ends at a
        // whole word (VKCF3Z's wrap rule 4, 2026-11-11).
        const titleLines = [
            ["one that fits is one line", "Week 46", 100, ["Week 46"]],
            ["two lines as near the same width as the words allow",
                "One two three four five six", 200, ["One two three", "four five six"]],
            ["never one word alone below, as filling the first would leave it",
                "Shared drive folder kept", 200, ["Shared drive", "folder kept"]],
            ["one word below only where nothing else fits", "Aaaa Bbbb Cccccccccccc", 160,
                ["Aaaa Bbbb", "Cccccccccccc"]],
            ["past two, the second ends at a whole word",
                "One two three four five six seven eight", 100, ["One two", "three\u2026"]],
            ["a word too wide alone keeps its breaks inside it", "Server time.harbour.test",
                100, ["Server time.\u200bharbour.\u200btest"]],
            ["an empty title is no line", "", 100, []]
        ];
        for (const [what, said, room, want] of titleLines)
            holds(`a title's lines: ${what}`, Identifier.titleLines(said, room, width, "\u2026"),
                want);
        // A list alone in a value, on one line, giving way to a tail.
        const kinds = ["PDF", "HTML", "PNG", "JPEG"].map(said => ({ said }));
        const lines = [
            ["a list that fits is every item a dot apart", kinds, 300,
                ["PDF \u00b7 HTML \u00b7 PNG \u00b7 JPEG"]],
            ["one that doesn't gives way by whole items from its end to the tail", kinds, 200,
                ["PDF \u00b7 HTML \u00b7 2 more"]],
            ["where not one item fits beside it, the tail alone", kinds, 60, ["4 more"]],
            ["nothing to list is no line", [], 200, []]
        ];
        for (const [what, items, wide, want] of lines)
            holds(`a drawer's list: ${what}`, Value.valueLine(items, "{count} more", wide, width),
                want);
        // A lead's one line: where and when never give way, and what follows
        // gives way at a whole word (VKCF3Z, 6ea9cd26).
        const kept = [{ said: "in Slack" }, { said: "14:02" }];
        const asked = "Can you look at the deploy?";
        const leads = [
            ["one that fits is whole", 500, [`in Slack \u00b7 14:02 \u00b7 ${asked}`]],
            ["one that doesn't ends at a whole word", 400,
                ["in Slack \u00b7 14:02 \u00b7 Can you look at the\u2026"]],
            ["and keeps where and when whole", 300, ["in Slack \u00b7 14:02 \u00b7 Can you\u2026"]],
            ["where not a word fits, where and when alone", 200, ["in Slack \u00b7 14:02"]],
            ["even past the frame, where and when never give way", 100,
                ["in Slack \u00b7 14:02"]]
        ];
        for (const [what, wide, want] of leads)
            holds(`a drawer's lead: ${what}`, Value.leadLine(kept, asked, wide, width), want);
        holds("a lead is measured as the line it's drawn on", Value.besideText({
            value: asked, valueItems: kept, gives: asked }, 300, width),
            "in Slack \u00b7 14:02 \u00b7 Can you\u2026");
        // What's measured beside a name, to choose a table or a list.
        const opens = { value: kinds.map(kind => kind.said).join(" \u00b7 "), valueItems: kinds,
            tail: "{count} more" };
        holds("a list with a tail is measured as the line it's drawn on, never whole",
            Value.besideText(opens, 200, width), "PDF \u00b7 HTML \u00b7 2 more");
        holds("any other value is measured whole", Value.besideText({ value: 42,
            valueItems: kinds, between: " " }, 60, width), "42");
        // Where a fact's figure stands: a stroke of 100 that reads down to
        // 30, a spark of 100 that reads only whole (VKCF3Z, 2026-11-09).
        const stroke = { shape: "stroke", usual: 100, least: 30 };
        const spark = { shape: "spark", usual: 100, least: 100 };
        const places = [
            ["a stroke with its room beside a one-line value", stroke, 1, 140, 260,
                { place: "beside", wide: 100 }],
            ["a stroke takes what room the line leaves", stroke, 1, 60, 260,
                { place: "beside", wide: 60 }],
            ["a stroke short of reading beside its value goes under it, as wide as ever",
                stroke, 1, 20, 260, { place: "under", wide: 100 }],
            ["a stroke under a value of two lines", stroke, 2, 140, 260,
                { place: "under", wide: 100 }],
            ["a stroke under its value no wider than the value runs", stroke, 1, 10, 80,
                { place: "under", wide: 80 }],
            ["a spark whole beside its value", spark, 1, 100, 260,
                { place: "beside", wide: 100 }],
            ["a spark that won't fit beside goes under", spark, 1, 99, 260,
                { place: "under", wide: 100 }],
            ["a spark that won't fit under either is the only one that goes", spark, 1, 20, 90,
                { place: "none", wide: 0 }],
            ["no figure, nothing", null, 1, 140, 260, { place: "none", wide: 0 }]
        ];
        // A value made of items, laid a dot apart, the dot going with a
        // break, so no line ends or starts on one (VKCF3Z, 2026-11-09).
        const runs = [
            ["a run that fits is one line", "40% \u00b7 resets Monday 09:00", 300,
                ["40% \u00b7 resets Monday 09:00"]],
            ["a run that breaks leaves its dot behind", "40% \u00b7 resets Monday 09:00", 200,
                ["40%", "resets Monday 09:00"]],
            ["as many items to a line as fit", "38 s \u00b7 4.1k out \u00b7 0.9 s first token", 170,
                ["38 s \u00b7 4.1k out", "0.9 s first token"]],
            ["an item wider than a line stands alone, for its words to wrap",
                "85% \u00b7 a reading far too long to stand on one line", 100,
                ["85%", "a reading far too long to stand on one line"]],
            ["one item is itself", "reading fine", 50, ["reading fine"]]
        ];
        for (const [what, words, wide, want] of runs)
            holds(`a drawer's run: ${what}`, Figure.runLines(words, wide, width), want);
        // A drawer's prose as five lines at most, the rest in its fold (VKCF3Z's
        // wrap rule 4, 76ba12d0): laid as Qt wraps words, at spaces, and where
        // it runs past, cut after the last word the fifth line holds with room
        // for the ellipsis that says there's more, a styled text's tags
        // closed where it's cut.
        const proses = [
            ["prose that fits is itself", "one two three", false, 100,
                { said: "one two three", cut: false }],
            ["five lines whole are not cut", "aa bb cc dd ee", false, 20,
                { said: "aa bb cc dd ee", cut: false }],
            ["past five, cut after the fifth line's last word that leaves room",
                "aa bb cc dd ee f gg", false, 40,
                { said: "aa bb cc dd ee\u2026", cut: true }],
            ["a last word with no room for it keeps it, as one word runs past",
                "aa bb cc dd eee ff", false, 30,
                { said: "aa bb cc dd eee\u2026", cut: true }],
            ["its own lines count, a blank one too", "a\nb\n\nc\nd\ne", false, 100,
                { said: "a\nb\n\nc\nd\u2026", cut: true }],
            ["a word wider than a line stands alone, as Qt lets it",
                "aaaaaaaaaaaaaa b c d e f", false, 20,
                { said: "aaaaaaaaaaaaaa b c d e\u2026", cut: true }],
            ["a styled text is measured by its letters, its tags closed",
                "<b>aa bb</b> cc<br>dd <a href=\"x\">ee ff</a> gg", true, 30,
                { said: "<b>aa bb</b> cc<br>dd <a href=\"x\">ee\u2026</a>", cut: true }],
            ["an entity is one letter", "a &amp; b c d e f g", true, 30,
                { said: "a &amp; b c d e f g", cut: false }]
        ];
        for (const [what, words, styled, wide, want] of proses)
            holds(`a drawer's prose: ${what}`,
                JSON.stringify(Prose.lines({ text: words, styled }, wide, 5, width)),
                JSON.stringify(want));
        for (const [what, figure, lines, room, under, want] of places)
            holds(`a figure's place: ${what}`,
                JSON.stringify(Figure.place(figure, lines, room, under)),
                JSON.stringify(want));
    }

    // How tall a drawer's preview stands, on a pitch of 24, two beats of
    // 12, with groups 20 apart: so the facts after it start on the first
    // beat at least the groups' gap below a picture, never a band rounded up
    // to the pitch (VKCF3Z, 2026-11-08); lines whole lines of the pitch.
    function drawerPreviewHeights(holds) {
        const grid = { pitch: 24, beat: 12, gap: 20 };
        // [what, the picture's height, lines of words, what it stands]
        const previews = [
            ["a picture and its gap ending on a beat stand no more", 100, 0, 96],
            ["a picture whose gap ends past a beat stands to the next", 100.2, 0, 108],
            ["a picture is never rounded up to the pitch", 105, 0, 108],
            ["a short picture keeps the whole gap", 12, 0, 12],
            ["lines stand whole lines of the pitch", 0, 3, 72],
            ["nothing yet stands nothing", 0, 0, 0]
        ];
        for (const [what, picture, lines, want] of previews)
            holds(`a drawer's preview: ${what}`, Page.previewHeight(picture, lines, grid), want);
    }

    function drawerRefusals(holds) {
        const into = ["Escape", "Ctrl+w", "l"];
        // [what, the keys typed from going in, the acts done]
        const doing = [
            ["x says E21", ["x"], ["cannotChange"]],
            ["and p and S", ["p", "S"], ["cannotChange", "cannotChange"]],
            ["and Vim's other change keys, none going into insert",
                ["D", "C", "X", "P", "s", "J", "~", "i", "a", "I", "A", "o", "O"],
                Array(13).fill("cannotChange")],
            ["r waits for its letter", ["r"], []],
            ["and says E21 once it comes, a digit too", ["r", "x", "r", "5"],
                ["cannotChange", "cannotChange"]],
            ["Escape lets r go unsaid", ["r", "Escape", "j"], ["next"]],
            ["d waits for its motion", ["d"], []],
            ["and says E21 once it comes", ["d", "j"], ["cannotChange"]],
            ["as dd and cc do", ["d", "d", "c", "c"], ["cannotChange", "cannotChange"]],
            ["a count goes before its motion", ["d", "3", "j"], ["cannotChange"]],
            ["a text object waits for its letter", ["d", "i"], []],
            ["and says it after", ["d", "i", "p", "c", "a", "w"],
                ["cannotChange", "cannotChange"]],
            ["as gg does", ["d", "g", "g"], ["cannotChange"]],
            ["a key that's no motion is refused too", ["c", "w"], ["cannotChange"]],
            ["Escape lets d go unsaid", ["d", "Escape", "j"], ["next"]],
            ["and Ctrl+C lets c go", ["c", "Ctrl+c", "j"], ["next"]],
            ["q says how to close", ["q"], ["quitSaid"]]
        ];
        for (const [what, keys, acts] of doing) {
            const done = typed(into.concat(keys));
            holds(`the drawer refuses: ${what}`, done.acts, ["normal", "drawer"].concat(acts));
            holds(`the drawer refuses: ${what}, the keys staying in it`, done.mode, "page");
        }
        holds("the drawer refuses: d waiting is something begun",
            Keys.pending(typed(into.concat(["d"])).standing), true);
        holds("the drawer refuses: and each key means something, never not here",
            typed(into.concat(["x", "d", "j", "q"])).idle, []);
        for (const act of ["cannotChange", "quitSaid"])
            holds(`the drawer refuses: ${act} is once a press`, Keys.repeats(act), false);
    }

    // Visual in the drawer, as on a page (VKCF3Z's spec v2): v chooses
    // letters from the cursor's and V whole lines, each again ending it and
    // the other changing its kind; the page's moves extend it, o goes to its
    // other end, y copies it and Escape lets it go, each back on the page;
    // and gv on the page brings the last back.
    function drawerVisual(holds) {
        const into = ["Escape", "Ctrl+w", "l"];
        // [what, the keys typed from going in, the acts done, the mode left in]
        const doing = [
            ["v chooses letters", ["v"], ["select"], "pageVisual"],
            ["V chooses lines", ["V"], ["selectLines"], "pageVisualLine"],
            ["v again ends it", ["v", "v"], ["select", "select"], "page"],
            ["V again ends it", ["V", "V"], ["selectLines", "selectLines"], "page"],
            ["V in letters takes lines", ["v", "V"], ["select", "selectLines"],
                "pageVisualLine"],
            ["v in lines takes letters", ["V", "v"], ["selectLines", "select"], "pageVisual"],
            ["the page's moves extend it", ["v", "j", "l", "G", "g", "g", "Ctrl+d", "n"],
                ["select", "next", "right", "last", "first", "halfDown", "seekNext"],
                "pageVisual"],
            ["and the lines'", ["V", "k", "Ctrl+e", "N"],
                ["selectLines", "previous", "scrollDown", "seekPrevious"], "pageVisualLine"],
            ["and the arrows, as their letters", ["v", "Down", "Left", "V", "Up", "Right"],
                ["select", "next", "left", "selectLines", "previous", "right"],
                "pageVisualLine"],
            ["o goes to its other end", ["v", "o"], ["select", "otherEnd"], "pageVisual"],
            ["y copies it and goes back to the page", ["v", "y"], ["select", "copy"], "page"],
            // Y copies the lines it's on however it chooses, as Vim's v_Y.
            ["Y copies its whole lines", ["V", "Y"], ["selectLines", "copyLines"], "page"],
            ["and by letters too", ["v", "Y"], ["select", "copyLines"], "page"],
            ["Escape lets it go", ["V", "Escape"], ["selectLines", "back"], "page"],
            ["and Ctrl+C", ["v", "Ctrl+c"], ["select", "back"], "page"],
            ["Escape lets a count go first", ["v", "3", "Escape"], ["select"], "pageVisual"],
            ["and on the page after does nothing", ["v", "Escape", "Escape", "j"],
                ["select", "back", "next"], "page"],
            ["its search lands in it", ["v", "/", "a", "Return"], ["select", "seek"],
                "pageVisual"],
            // A page can't be changed, so its operators say Vim's E21 over
            // what's chosen and end it, as visual's d does on a buffer that
            // can't be modified (VKCF3Z's spec v2, Operators).
            ["d says E21 and lets it go", ["v", "d"], ["select", "cannotChange"], "page"],
            ["and so do x, c and p", ["V", "x", "v", "c", "v", "p"],
                ["selectLines", "cannotChange", "select", "cannotChange", "select",
                    "cannotChange"], "page"],
            ["and Vim's other change keys",
                ["D", "C", "X", "P", "s", "S", "J", "~"].reduce((keys, key) =>
                    keys.concat(["v", key]), []),
                Array(8).fill(["select", "cannotChange"]).reduce((acts, two) =>
                    acts.concat(two), []), "page"],
            ["q says how to close, choosing on", ["v", "q", "j"],
                ["select", "quitSaid", "next"], "pageVisual"],
            ["and no way back to the rows is in it, Space nothing and the arrow a move",
                ["v", "Z", "Z", "Space", "Left", "h"], ["select", "left", "left"],
                "pageVisual"],
            ["gv on the page brings the last back", ["v", "Escape", "g", "v"],
                ["select", "back", "reselect"], "page"]
        ];
        for (const [what, keys, acts, mode] of doing) {
            const done = typed(into.concat(keys));
            holds(`the drawer's visual: ${what}`, done.acts, ["normal", "drawer"].concat(acts));
            holds(`the drawer's visual: ${what}, in its mode`, done.mode, mode);
        }
        holds("the drawer's visual: a count goes before a move in it",
            typed(into.concat(["v", "3", "j"])).counted, ["normal", "drawer", "select", "3 next"]);
        holds("the drawer's visual: and before an arrow",
            typed(into.concat(["v", "2", "Up"])).counted,
            ["normal", "drawer", "select", "2 previous"]);
        holds("the drawer's visual: a key that does nothing in it is its own",
            typed(into.concat(["v", "d"])).ours, true);
        holds("the drawer's visual: : in it opens the command line, whose :q goes back",
            typed(into.concat(["V", ":"])).acts, ["normal", "drawer", "selectLines", "command"]);
        for (const act of ["select", "selectLines", "otherEnd", "copy", "copyLines", "reselect",
            "back"])
            holds(`the drawer's visual: ${act} is once a press`, Keys.repeats(act), false);
        holds("the drawer's visual: d and q mean something in it, never not here",
            typed(into.concat(["v", "d", "v", "q"])).idle, []);
        holds("the drawer's visual: v on the rows is V, the rows' visual line",
            typed(["Escape", "v"]).mode, "visual");
        for (const mode of ["pageVisual", "pageVisualLine"])
            holds(`the drawer's visual: ${mode} is the drawer's`, Keys.paged(mode), true);
        holds("the drawer's visual: the page is the drawer's", Keys.paged("page"), true);
        holds("the drawer's visual: normal isn't", Keys.paged("normal"), false);

        // What the drawer's foot counts of the words it hides
        // (DrawerPage.paragraphs): a paragraph each, a line Qt wrapped
        // part of its own, however Qt keeps the breaks between them.
        holds("the words hidden: a log's lines are a paragraph each",
            Page.paragraphs("line 14\u2028line 15\u2028line 16"), 3);
        holds("the words hidden: so are a plain text's and a styled one's",
            Page.paragraphs("one\ntwo\u2029three"), 3);
        holds("the words hidden: a wrapped paragraph is one, a blank line none",
            Page.paragraphs("a long paragraph Qt wrapped\n\n  \u2028"), 1);

        // What visual takes (DrawerPage.chosen), from where it began to the
        // cursor, either way round, on lines ten letters long but the 3rd,
        // empty: [what, began, cursor, kind, each line's [line, from, to]],
        // `to` past the last letter taken, since the cursor's own is.
        const lengths = [10, 10, 0, 10, 10].map(length => ({ length }));
        const chose = [
            ["letters along one line", [1, 2], [1, 5], "letters", [[1, 2, 6]]],
            ["and back along it", [1, 5], [1, 2], "letters", [[1, 2, 6]]],
            ["one letter, where it began", [1, 4], [1, 4], "letters", [[1, 4, 5]]],
            ["letters down three lines, the empty one whole", [1, 3], [3, 1], "letters",
                [[1, 3, 10], [2, 0, 0], [3, 0, 2]]],
            ["and up them", [3, 1], [1, 3], "letters", [[1, 3, 10], [2, 0, 0], [3, 0, 2]]],
            ["whole lines, from either end", [3, 4], [1, 7], "lines",
                [[1, 0, 10], [2, 0, 0], [3, 0, 10]]],
            ["one whole line", [4, 9], [4, 0], "lines", [[4, 0, 10]]]
        ];
        for (const [what, began, at, kind, want] of chose) {
            const got = Page.chosen({ line: began[0], letter: began[1] },
                { line: at[0], letter: at[1] }, kind, lengths);
            holds(`visual takes ${what}`, got.map(one => [one.line, one.from, one.to]), want);
        }
    }

    // Where the drawer's keys take its cursor and its view (DrawerPage.js),
    // on a page of twenty lines of the pitch, 24 px, set as a plate sets
    // them: a title's two, an empty line, three facts', another, and its
    // words', each letter 10 px wide and each line ten long but the 7th,
    // three. Its view holds ten lines, so it scrolls 288 px at most, and
    // keeps four lines about the cursor, as his scrolloff does.
    function drawerPage(holds) {
        const tops = [0, 24, 72, 96, 120, 168, 192, 216, 240, 264, 288, 312, 336, 360, 384,
            408, 432, 456, 480, 504];
        const lines = tops.map((top, at) => ({ top, height: 24, length: at === 6 ? 3 : 10 }));
        const page = (from, extra) => Object.assign({
            lines,
            line: 0,
            letter: 0,
            want: 0,
            top: 0,
            height: 240,
            end: 288,
            pitch: 24,
            off: 4,
            xOf: (line, letter) => letter * 10,
            letterAt: (line, x) => Math.max(0, Math.min(Math.round(x / 10),
                lines[line].length - 1))
        }, from, extra ?? {});
        const where = moved => [moved.line, moved.letter, moved.want, moved.top];
        // [what, where it starts, the act, the count, [line, letter, want, top]]
        const moves = [
            ["j goes down a line", {}, "next", 0, [1, 0, 0, 0]],
            ["5j scrolls to keep four lines below it", {}, "next", 5, [5, 0, 0, 48]],
            ["k at the first line stays", {}, "previous", 0, [0, 0, 0, 0]],
            ["k scrolls to keep four lines above it", { line: 9, top: 192 }, "previous", 0,
                [8, 0, 0, 144]],
            ["G goes to the last line, scrolled to the end", { letter: 4, want: 40 }, "last", 0,
                [19, 0, 0, 288]],
            ["gg goes to the first, scrolled to the top", { line: 19, top: 288 }, "first", 0,
                [0, 0, 0, 0]],
            ["3G goes to the 3rd line", {}, "last", 3, [2, 0, 0, 0]],
            ["a line past the last, the last", {}, "first", 99, [19, 0, 0, 288]],
            ["l goes on a letter", {}, "right", 0, [0, 1, 10, 0]],
            ["3l goes on three", {}, "right", 3, [0, 3, 30, 0]],
            ["l stops at the line's last letter", {}, "right", 20, [0, 9, 90, 0]],
            ["h at the first letter stays", {}, "left", 0, [0, 0, 0, 0]],
            ["j onto a shorter line, its last letter, keeping where it was",
                { line: 5, letter: 7, want: 70, top: 48 }, "next", 0, [6, 2, 70, 72]],
            ["and on to a longer line, back where it was",
                { line: 6, letter: 2, want: 70, top: 72 }, "next", 0, [7, 7, 70, 96]],
            ["Ctrl+E scrolls a line, the cursor kept four from the top", {}, "scrollDown", 0,
                [4, 0, 0, 24]],
            ["Ctrl+E at the end stays", { line: 19, top: 288 }, "scrollDown", 0,
                [19, 0, 0, 288]],
            ["Ctrl+Y at the top stays", {}, "scrollUp", 0, [0, 0, 0, 0]],
            ["Ctrl+Y scrolls up a line, the cursor kept four from the bottom",
                { line: 11, top: 192 }, "scrollUp", 0, [10, 0, 0, 168]],
            ["Ctrl+D scrolls half the view, the cursor with it and kept from the top", {},
                "halfDown", 0, [7, 0, 0, 120]],
            ["Ctrl+U back", { line: 7, top: 120 }, "halfUp", 0, [3, 0, 0, 0]],
            ["Ctrl+F scrolls a view less two lines, the cursor at its top", {}, "pageDown", 0,
                [10, 0, 0, 192]],
            ["Ctrl+B back, the cursor at its bottom", { line: 19, top: 288 }, "pageUp", 0,
                [7, 0, 0, 96]],
            ["Ctrl+F at the end goes to the last line", { line: 15, top: 288 }, "pageDown", 0,
                [19, 0, 0, 288]],
            ["Ctrl+B at the top goes to the first", { line: 3 }, "pageUp", 0, [0, 0, 0, 0]],
            ["H goes to the view's top line", { line: 3 }, "viewTop", 0, [0, 0, 0, 0]],
            ["L to its bottom, four lines up", {}, "viewBottom", 0, [4, 0, 0, 0]],
            ["M to the middle of the eight lines wholly in view", {}, "viewMiddle", 0,
                [3, 0, 0, 0]],
            ["H scrolled, four lines down", { line: 8, top: 120 }, "viewTop", 0,
                [7, 0, 0, 120]],
            ["3H past what's in view, the last of it", { line: 7, top: 120 }, "viewTop", 3,
                [8, 0, 0, 120]]
        ];
        for (const [what, from, act, count, want] of moves)
            holds(`the drawer's page: ${what}`, where(Page.moved(act, count, page(from))), want);

        // A page that fits its view, three lines, scrolls nowhere, its
        // cursor going among its lines.
        const short = { lines: lines.slice(0, 3), end: 0 };
        const fitting = [
            ["j", {}, "next", 0, [1, 0, 0, 0]],
            ["Ctrl+E", {}, "scrollDown", 0, [0, 0, 0, 0]],
            ["Ctrl+D, to its last line", {}, "halfDown", 0, [2, 0, 0, 0]],
            ["Ctrl+F, to its last line", {}, "pageDown", 0, [2, 0, 0, 0]],
            ["G", {}, "last", 0, [2, 0, 0, 0]],
            ["L, its last line", {}, "viewBottom", 0, [2, 0, 0, 0]]
        ];
        for (const [what, from, act, count, want] of fitting)
            holds(`a drawer that fits: ${what}`, where(Page.moved(act, count,
                page(from, short))), want);
        holds("a drawer with no lines stays", where(Page.moved("next", 0,
            page({}, { lines: [], end: 0 }))), [0, 0, 0, 0]);

        // Where the view goes to show a line, and keeps four about it.
        holds("the view comes down to show a line below it",
            Page.shown(lines[12], 0, page({})), 216);
        holds("and up to show one above", Page.shown(lines[2], 288, page({})), 0);

        // A page whose eyebrow and title aren't a pitch tall, as a plate's
        // are: the view's top is always on a whole line, as Vim's topline is,
        // never a pitch partway through the title (VKCF3Z, 2026-11-05), here
        // 48. The body's lines lie on the pitch a pixel short of it, as the
        // drawer lays them.
        const titled = [{ top: 4, height: 16, length: 10 }, { top: 28, height: 40, length: 10 }]
            .concat(Array.from({ length: 20 }, (_, at) => ({ top: 72 + at * 24, height: 23,
                length: 10 })));
        const onTitled = from => page(from, { lines: titled, end: 336 });
        const titledMoves = [
            ["5j from the title, down to the first body line", { line: 1 }, "next", 5,
                [6, 0, 0, 72]],
            ["k up to the title shows it whole", { line: 6, top: 96 }, "previous", 0,
                [5, 0, 0, 24]]
        ];
        for (const [what, from, act, count, want] of titledMoves)
            holds(`a titled page: ${what}`, where(Page.moved(act, count, onTitled(from))), want);
        holds("a titled page: Ctrl+E puts the title at the top",
            Page.moved("scrollDown", 0, onTitled({ line: 6, top: 0 })).top, 24);
        holds("a titled page: and again the first body line, past the title whole",
            Page.moved("scrollDown", 0, onTitled({ line: 6, top: 24 })).top, 72);
        holds("a titled page: Ctrl+Y back to the title",
            Page.moved("scrollUp", 0, onTitled({ line: 12, top: 72 })).top, 24);
        holds("a titled page: G stops at the end, on a line",
            Page.moved("last", 0, onTitled({})).top, 336);
        // A title whose descenders reach two pixels past a pitch, as the
        // drawer's did at 2 (scrolled 96, the title's box ending at 98):
        // that pitch would show their tips, so the view goes on past it.
        const descended = [{ top: 4, height: 16, length: 10 },
            { top: 28, height: 46, length: 10 }].concat(Array.from({ length: 20 },
            (_, at) => ({ top: 96 + at * 24, height: 23, length: 10 })));
        holds("a title two pixels past a pitch: Ctrl+E goes on past it",
            Page.moved("scrollDown", 0, page({ line: 6, top: 24 },
                { lines: descended, end: 360 })).top, 96);
        // A view resting below the page's top, as the drawer's does so a
        // line scrolled to its top takes the eyebrow's baseline, and
        // stepping on its body's pitch: its rest a stop, then the body's
        // lines' tops, none a pitch partway through the eyebrow or title
        // and none over the bare room between the title and the body, though
        // a blank line between groups is a stop (the drawer's page, above).
        const headed = [{ top: 20, height: 18, length: 10 }, { top: 38, height: 36, length: 10 }]
            .concat(Array.from({ length: 20 }, (_, at) => ({ top: 121 + at * 24, height: 23,
                length: 10 })));
        const onHeaded = from => page(from, { lines: headed, start: 16, phase: 121, end: 361 });
        holds("a view resting below the top: its stops", Page.tops(onHeaded({})).slice(0, 4),
            [16, 121, 145, 169]);
        holds("a view resting below the top: Ctrl+E past the title whole, to the body's first",
            Page.moved("scrollDown", 0, onHeaded({ line: 2, top: 16 })).top, 121);
        holds("a view resting below the top: Ctrl+Y back to its rest",
            Page.moved("scrollUp", 0, onHeaded({ line: 8, top: 121 })).top, 16);
        holds("a view resting below the top: gg back to its rest",
            Page.moved("first", 0, onHeaded({ line: 12, top: 145 })).top, 16);
        holds("a view resting below the top: G at its end, on a stop",
            Page.moved("last", 0, onHeaded({ top: 16 })).top, 361);
        holds("and stays where a line is in view", Page.shown(lines[4], 0, page({})), 0);
        holds("never past the end", Page.shown(lines[19], 0, page({})), 288);
        holds("a short view keeps less about it, never past its middle",
            Page.shown(lines[5], 0, page({}, { height: 120, end: 408 })), 120);
    }

    function run() {
        function holds(what, got, want) {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        }

        // [what, the keys typed from the opening, the acts done]
        const doing = [
            ["the find's arrows, Home and End move the caret",
                ["Left", "Right", "Home", "End"],
                ["caretLeft", "caretRight", "caretStart", "caretEnd"]],
            ["the find's Backspace, Ctrl+W and Ctrl+U delete", ["Backspace", "Ctrl+w", "Ctrl+u"],
                ["deleteBack", "deleteWord", "deleteToStart"]],
            ["Ctrl+H deletes back, as Backspace", ["Ctrl+h"], ["deleteBack"]],
            ["the find's next row, as his completion menu's", ["Down", "Ctrl+n"],
                ["next", "next"]],
            // Tab on the find goes into a row holding parts, and is its next
            // row on any other, as the launcher decides by the row (VKCF3Z,
            // 7f53e7b3).
            ["the find's Tab goes in, or else to the next row", ["Tab"], ["into"]],
            ["the find's row before", ["Up", "Shift+Tab", "Ctrl+p"],
                ["previous", "previous", "previous"]],
            ["Ctrl+Y completes", ["Ctrl+y"], ["complete"]],
            ["Return does the row on the find, and in normal", ["Return", "Escape", "Return"],
                ["run", "normal", "run"]],
            ["Ctrl+C leaves the find for normal", ["Ctrl+c"], ["normal"]],
            // Escape at rest does nothing, as Vim's does in normal: it
            // never closes or quits anything (Joel, 2026-11-05 15:3x:
            // "fuck escape quitting"), which :q, ZZ and Space w do.
            ["Escape gives normal, and then does nothing", ["Escape", "Escape", "Escape"],
                ["normal"]],
            // Return goes in and - up, as oil's; h and l move along a line,
            // which a row hasn't, so they only say Vim's way in and out, and
            // Backspace is h (the vi-paradigm spec, Normal).
            ["normal's h and l say the way in and out, and the arrows nothing",
                ["Escape", "Left", "h", "Right", "l"], ["normal", "outSaid", "intoSaid"]],
            ["normal's - goes up, and Backspace says how, as h", ["Escape", "-", "Backspace"],
                ["normal", "out", "outSaid"]],
            ["Return goes in from normal", ["Escape", "Return"], ["normal", "run"]],
            ["q only says how to close, and ZZ closes", ["Escape", "q", "Z", "Z"],
                ["normal", "quitSaid", "close"]],
            ["a Z with anything but Z after it lets go", ["Escape", "Z", "j", "Z", "Z"],
                ["normal", "close"]],
            [": in normal opens the command line", ["Escape", ":"], ["normal", "command"]],
            ["a colon after letters on the find is the field's to type", ["s", ":", "Return"],
                ["run"]],
            ["normal's Down and Up are j and k, with counts (Joel, 2026-11-05 12:1x)",
                ["Escape", "Down", "Up", "j", "k"], ["normal", "next", "previous", "next", "previous"]],
            ["Space Tab goes to the place used last", ["Escape", "Space", "Tab"],
                ["normal", "lastPlace"]],
            // Ctrl+I alone goes forward through the places stepped into,
            // Ctrl+O back, and Tab, which Neovim under kitty's keyboard
            // protocol tells apart from it, goes in (VKCF3Z, 7fd0aa50).
            ["Ctrl+O and Ctrl+I go back and forward through the places stepped into",
                ["Escape", "Ctrl+o", "Ctrl+i"], ["normal", "jumpBack", "jumpForward"]],
            ["Tab goes into what a row holds", ["Escape", "Tab"], ["normal", "into"]],
            // Ctrl+] goes to the row's own place, as Vim follows a tag: where
            // a row found elsewhere lives, or into the parts of a row whose
            // Return opens it (VKCF3Z, 2026-11-05); Ctrl+T comes back through
            // those followed.
            ["Ctrl+] follows the row to its own place, and Ctrl+T back",
                ["Escape", "Ctrl+]", "Ctrl+t"], ["normal", "follow", "tagBack"]],
            // And Ctrl+] from the find, where a program's name is typed, so
            // its commands and windows are a key away (Joel, 2026-11-05);
            // Ctrl+T there stays the field's.
            ["and from the find", ["Ctrl+]"], ["follow"]],
            ["on the find, Ctrl+T is the field's", ["Ctrl+t"], []],
            ["Space ? and F1 show every key here", ["Escape", "Space", "?", "F1"],
                ["normal", "keys", "keys"]],
            ["Space w closes, as his leader closes a split", ["Escape", "Space", "w"],
                ["normal", "close"]],
            // Space u l switches the gutter's numbers between relative and
            // absolute, as his u l toggles relativenumber.
            ["Space u l switches the numbers", ["Escape", "Space", "u", "l"],
                ["normal", "numbers"]],
            ["Space alone begins, and a key it doesn't lead lets it go",
                ["Escape", "Space", "j", "j"], ["normal", "next"]],
            ["Escape cancels a Space waiting, then does nothing",
                ["Escape", "Space", "Escape", "Escape", "j"], ["normal", "next"]],
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
            ["the find's Ctrl+U still deletes to the start", ["Ctrl+u"], ["deleteToStart"]],
            ["a g with anything but g after it lets go", ["Escape", "g", "j", "g", "g"],
                ["normal", "first"]],
            // / and ? open the one find afresh, going down and up, which
            // n and N go on with as Vim's do (Joel, 2026-11-05).
            ["/ opens a fresh find going down, ? one going up", ["Escape", "/", "Escape", "?"],
                ["normal", "freshFind", "normal", "freshFindBack"]],
            ["n and N go to the find's next row and the one before", ["Escape", "n", "N"],
                ["normal", "matchNext", "matchBack"]],
            // f and F wait for a letter, then go to the next row whose name
            // begins with it and the one before; ; and , go again, the same
            // way and the other, as Vim's go to a letter on the line.
            ["f and F go to a row by its letter", ["Escape", "f", "s", "F", "S"],
                ["normal", "toLetter", "toLetterBack"]],
            ["f's letter is any that types, a move's among them", ["Escape", "f", "j", "F", "n"],
                ["normal", "toLetter", "toLetterBack"]],
            ["; and , go again, the same way and the other", ["Escape", ";", ","],
                ["normal", "letterAgain", "letterAgainBack"]],
            ["Escape lets f go, then does nothing", ["Escape", "f", "Escape", "j", "Escape"],
                ["normal", "next"]],
            ["a key that types nothing lets f go", ["Escape", "f", "Return", "j"],
                ["normal", "next"]],
            ["visual extends by n and f", ["Escape", "V", "n", "f", "x", ";"],
                ["normal", "selectLines", "matchNext", "toLetter", "letterAgain"]],
            ["on the find, n N f F ; and , type", ["n", "N", "f", "F", ";", ","], []],
            ["i a I A go back to the find",
                ["Escape", "i", "Escape", "a", "Escape", "I", "Escape", "A"],
                ["normal", "insert", "normal", "append", "normal", "insertStart", "normal",
                    "appendEnd"]],
            ["cc and S clear the line onto the find", ["Escape", "c", "c", "Escape", "S"],
                ["normal", "change", "normal", "change"]],
            ["]b and [b go to the place beside, the next and the one before",
                ["Escape", "]", "b", "[", "b"], ["normal", "nextFile", "previousFile"]],
            ["gx opens the row's file in its own program", ["Escape", "g", "x"],
                ["normal", "openOwn"]],
            ["@: runs the last command line again", ["Escape", "@", ":"],
                ["normal", "commandAgain"]],
            ["Escape cancels a c waiting for its second", ["Escape", "c", "Escape", "c"],
                ["normal"]],
            ["a key that ends nothing lets c go", ["Escape", "c", "x", "c"], ["normal"]],
            ["Shift pressed for a capital changes nothing", ["Escape", "c", "", "c"],
                ["normal", "change"]],
            ["the arrows with Shift do nothing in either mode: Ctrl+W l goes into the drawer",
                ["Shift+Down", "Escape", "Shift+Up"], ["normal"]],
            ["Page Up and Down page the rows in either mode",
                ["PageDown", "Escape", "PageUp"], ["pageDown", "normal", "pageUp"]],
            ["Return with Shift, Ctrl or Alt does nothing, on the find and in normal",
                ["Shift+Return", "Ctrl+Return", "Alt+Return", "Shift+Enter", "Escape",
                    "Shift+Return", "Ctrl+Return", "Alt+Return", "Ctrl+Enter"], ["normal"]],
            ["a prompt's words take the find's keys",
                ["Left", "Ctrl+h", "Ctrl+w", "Ctrl+y", "Return", "Escape"],
                ["caretLeft", "deleteBack", "deleteWord", "complete", "run", "normal"], true]
        ];
        for (const [what, keys, want] of doing)
            holds(what, typed(keys).acts, want);
        // A press named as the table names keys (nameOf): Alt with a letter
        // it typed as the key's own is Vim's M- key, never the letter, so
        // Alt+d never begins d's operator; a letter Alt shifts to another
        // on the layout goes by what it typed, and one it types beyond the
        // table's marks is the field's.
        const press = (fields, text, letter, named) => Object.assign({ control: false,
            shift: false, alt: false, text, letter, named: named ?? "" }, fields);
        const names = [
            ["a letter", press({}, "d", "d"), "d"],
            ["a capital", press({ shift: true }, "D", "d"), "D"],
            ["Alt with a letter", press({ alt: true }, "d", "d"), "Alt+d"],
            ["Alt with a capital", press({ alt: true, shift: true }, "D", "d"), "Alt+D"],
            ["Alt with Return", press({ alt: true }, "\r", "", "Return"), "Alt+Return"],
            ["Ctrl with a letter", press({ control: true }, "\u0017", "w"), "Ctrl+w"],
            ["Alt shifting a letter to a mark", press({ alt: true }, "@", "q"), "@"],
            ["Alt typing beyond the marks", press({ alt: true }, "\u00f0", "d"), ""],
            ["Shift with Tab", press({ shift: true }, "", "", "Tab"), "Shift+Tab"],
            // Ctrl+] types a control character, so it goes by its key's
            // name, as Prompt.qml names the bracket.
            ["Ctrl with ]", press({ control: true }, "\u001d", "", "]"), "Ctrl+]"]
        ];
        for (const [what, pressed, want] of names)
            holds(`${what} is named ${want}`, KeyNames.nameOf(pressed), want);

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
            ["Escape only cancels a count, then does nothing",
                ["Escape", "5", "Escape", "j", "Escape"], ["normal", "next"]],
            ["Escape only cancels a g waiting, then does nothing",
                ["Escape", "g", "Escape", "Escape", "j"], ["normal", "next"]],
            ["a key that ends nothing lets the count go", ["Escape", "3", "x", "j"],
                ["normal", "next"]],
            ["a count before a colon goes with it", ["Escape", "3", ":"],
                ["normal", "3 command"]]
        ];
        for (const [what, keys, want] of counting)
            holds(what, typed(keys).counted, want);
        holds("a digit on the find is the field's to type", typed(["3"]).ours, false);
        holds("and counts nothing", typed(["3", "Escape", "j"]).counted, ["normal", "next"]);
        holds("a count typed is the line's", typed(["Escape", "4"]).ours, true);
        holds("and stands until its move", typed(["Escape", "4", "2"]).standing,
            { count: "42", typed: [] });
        holds("a count before a colon is the range Vim's 3: gives",
            [Operators.counted(0), Operators.counted(1), Operators.counted(3)],
            ["", ".", ".,.+2"]);
        holds("and the keys stand at rest once it opens",
            typed(["Escape", "3", ":"]).standing, { count: "", typed: [] });
        holds("the find's Space is the field's to type", typed(["Space"]).ours, false);
        holds("a prompt's letters are its own to type", typed(["x"], true).ours, false);
        // The rows have no search of their own: the find line is the list's
        // one way to find (VKCF3Z's vi-paradigm spec, The Find Line).
        holds("/ goes to the find", typed(["Escape", "/"]).mode, "find");
        // A colon as the find's first letter opens the command line, as
        // Vim's does from anywhere; after letters it's one of the find's.
        holds("a colon on an empty find opens the command line",
            Keys.opensCommands(":", "find", ""), true);
        holds("but not after letters", Keys.opensCommands(":", "find", "sl"), false);
        holds("nor in a prompt's words", Keys.opensCommands(":", "insert", ""), false);
        holds("f waits for its letter, keeping the count before it",
            typed(["Escape", "3", "f"]).standing,
            { count: "3", typed: [], letter: { forward: true } });
        holds("and goes as many rows on, as 3fx does", typed(["Escape", "3", "f", "x"]).counted,
            ["normal", "3 toLetter"]);
        holds("and so do n, ; and their others", typed(["Escape", "2", "n", "4", ","]).counted,
            ["normal", "2 matchNext", "4 letterAgainBack"]);
        holds("f's letter is said with it",
            Keys.read({ count: "", typed: [], letter: { forward: false } }, "S", "normal").letter,
            "S");
        holds("a d waiting lets go at f, rows having no letters to end to",
            typed(["Escape", "d", "f", "j"]).counted, ["normal", "next"]);
        holds("f on a prompt's words means nothing, and says so",
            Keys.read(Keys.rest(), "f", "normal", true).idle, true);
        // A key with no meaning here says so, never silent (the vi-paradigm
        // spec, Normal); a count or a sequence begun means something.
        holds("p means nothing in normal, and says so",
            typed(["Escape", "p"]).idle, ["p"]);
        // The looks and finds-by-name VM tests press t for a key meaning
        // nothing, since z begins za's fold and waits for its second letter.
        holds("t means nothing in normal, and says so, where z waits for za",
            [typed(["Escape", "t"]).idle, typed(["Escape", "z"]).idle], [["t"], []]);
        holds("a key that ends what was begun means nothing, and says so",
            typed(["Escape", "]", "x"]).idle, ["x"]);
        holds("a count, a sequence begun and a move each mean something",
            typed(["Escape", "3", "g", "g", "Space", "?"]).idle, []);
        // Escape at rest is Vim's, which says nothing in his Neovim, its
        // bell off: pressed out of habit to be sure of normal, it's never
        // told "not here".
        holds("Escape at rest in normal does nothing, and says nothing",
            typed(["Escape", "Escape", "Escape"]), typed(["Escape"]));
        // The read says it's at rest, so a prompt does what rest does
        // (Prompt.qml's rested) by one rule rather than its own copy.
        holds("Escape at rest in normal is read as rest",
            Keys.read(Keys.rest(), "Escape", "normal").rested, true);
        holds("but not while a count waits, which it lets go",
            !!Keys.read(typed(["Escape", "3"]).standing, "Escape", "normal").rested, false);
        holds("nor on the find, which it leaves",
            !!Keys.read(Keys.rest(), "Escape", "find").rested, false);
        // Escape only leaves what was begun or goes back a step, so no
        // act added to it later can close or quit anything.
        holds("Escape's rows only leave for normal or go back",
            [...new Set(Keys.table.filter(one => one.keys.includes("Escape"))
                .map(one => one.act))].sort(), ["back", "normal"]);
        holds("a letter on the find types, meaning nothing idle", typed(["n"]).idle, []);
        // A begun Space waits for its next key however long, as his
        // which-key holds it (Prompt.qml's standing); Escape lets it go.
        holds("Escape lets a begun Space go, so a ? after it finds upward",
            typed(["Escape", "Space", "Escape", "?"]).acts, ["normal", "freshFindBack"]);
        // Space ? lists the keys by what each line says, its keys as the
        // desktop draws them, Vim's own where an act has them, and its arrow.
        const lines = { rows: "rows", screen: "screen", up: "up", openIn: "open in",
            follow: "follow", run: "run", lastPlace: "last", jumps: "jumps", find: "find",
            keys: "keys", close: "close",
            numbers: "numbers", ends: "ends", nextRow: "next row", caret: "caret",
            word: "word", intoDrawer: "into the drawer", lines: "lines", along: "along",
            scroll: "scroll", back: "back", ending: "ending", seek: "seek", seekOn: "seek on",
            select: "select", reselect: "reselect", otherEnd: "other end", copy: "copy",
            copyRow: "copy row", option: "option", optionHere: "option here", undo: "undo",
            redo: "redo", again: "again", chooseRows: "choose rows", paragraph: "paragraph",
            endChosen: "end chosen", copyChosen: "copy chosen", openChosen: "open chosen",
            menu: "menu", matches: "matches", letters: "letters" };
        const modes = { normal: "Normal", find: "Find", command: "Commands", visual: "Visual",
            insert: "Insert", page: "Drawer", pageVisual: "Choosing" };
        const list = Listed.keyList(Keys.table, lines, modes);
        const keyed = value => (list.find(line => line.value === value) ?? {}).label;
        holds("the list's rows, Vim's alone", keyed("rows"), "j  k");
        holds("the list's ends, a sequence as Vim writes it", keyed("ends"), "gg  G");
        holds("the list's screens, Ctrl's before the named keys", keyed("screen"),
            "Ctrl+F  Ctrl+B");
        holds("the list's way up, - alone, h and l stepping nowhere", keyed("up"), "-");
        holds("and normal's Return, which opens or goes in", keyed("open in"), "\u21b5");
        holds("the list's places stepped into, back and forward", keyed("jumps"),
            "Ctrl+O  Ctrl+I");
        holds("the list's fresh find", keyed("find"), "/  ?");
        holds("the list's next match and the one before", keyed("matches"), "n  N");
        holds("the list's rows by a letter, and again", keyed("letters"), "f  F  ;  ,");
        holds("the list's Return, once", keyed("run"), "\u21b5");
        holds("the list's place used last", keyed("last"), "Space Tab");
        // Each chord a run of its own, which the drawer sets apart, a
        // sequence's keys staying in one.
        holds("the list's chords, each its own",
            (list.find(line => line.value === "screen") ?? {}).chords, ["Ctrl+F", "Ctrl+B"]);
        holds("the list's sequence, one chord",
            (list.find(line => line.value === "last") ?? {}).chords, ["Space Tab"]);
        // Each chord's keys runs of their own, which the drawer ties, and
        // letters typed together one run, as they're typed.
        holds("a chord's keys, each its own",
            (list.find(line => line.value === "last") ?? {}).keys, [["Space", "Tab"]]);
        holds("a sequence of letters, one run",
            (list.find(line => line.value === "ends") ?? {}).keys, [["gg"], ["G"]]);
        holds("and Ctrl+], to a row's own place, and Ctrl+T back", keyed("follow"),
            "Ctrl+]  Ctrl+T");
        holds("the list's own keys", keyed("keys"), "Space ?  F1");
        holds("the list's numbers", keyed("numbers"), "Space u l");
        holds("the list's closing, :q among them and q not", keyed("close"), "ZZ  :q");
        holds("the list's d, over the row and as far as a motion", keyed("ending"), "dd  dj");
        // An option, the machine's by :set and that of what it's on by :setl,
        // as Vim's :setlocal is the buffer's (VKCF3Z, 2026-11-05).
        holds("the list's options, the machine's and what it's on",
            [keyed("option"), keyed("option here")], [":set dnd", ":setl mute"]);
        holds("and y", keyed("copy row"), "yy  yG");
        holds("u, Ctrl+R and . over the rows' changes",
            [keyed("undo"), keyed("redo"), keyed("again")], ["u", "Ctrl+R", "."]);
        holds("visual's own keys, as it takes them", [keyed("choose rows"),
            keyed("paragraph"), keyed("end chosen"), keyed("copy chosen"),
            keyed("open chosen")], ["V  v", "ip  ap", "d  x", "y", "\u21b5"]);
        holds("the command line's menu, as Vim's", keyed("menu"), "Ctrl+N  Ctrl+P");
        holds("no line scrolls the drawer with Shift and an arrow",
            list.filter(line => (line.label ?? "").includes("\u21e7+\u2193")).length, 0);
        holds("the list's way into the drawer, Vim's window keys first",
            keyed("into the drawer"), "Ctrl+W l");
        // In the drawer, its lines and along them, Vim's first and the arrow,
        // a line scrolled by Ctrl, and Space with the left arrow back.
        holds("the drawer's lines", keyed("lines"), "j  k");
        holds("the drawer's letters along a line", keyed("along"), "h  l");
        holds("the drawer's scroll by a line", keyed("scroll"), "Ctrl+E  Ctrl+Y");
        holds("the drawer's way back, Vim's window key and :q", keyed("back"),
            "Ctrl+W h  :q");
        holds("the drawer's search, either way", keyed("seek"), "/  ?");
        holds("and on to its next match, either way", keyed("seek on"), "n  N");
        holds("the drawer's visual, by letters and lines", keyed("select"), "v  V");
        holds("and the last brought back", keyed("reselect"), "gv");
        holds("choosing, its other end", keyed("other end"), "o");
        holds("and copying it", keyed("copy"), "y  Y");
        holds("choosing has a section of its own",
            list.some(line => line.heading === "Choosing"), true);
        // Listed from the drawer, only the drawer's keys: its page's and
        // choosing's, its own Space ? among them.
        const drawn = Listed.keyList(Keys.table, lines, modes, ["page", "pageVisual"]);
        holds("the drawer's list, its sections alone",
            drawn.filter(line => line.heading !== undefined).map(line => line.heading),
            ["Drawer", "Choosing"]);
        holds("and its own list of keys among them",
            drawn.filter(line => line.value === "keys").map(line => line.label),
            ["Space ?  F1"]);
        holds("and the rows' not", drawn.some(line => line.value === "into the drawer"),
            false);
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
            ["Normal", "Find", "Commands", "Visual", "Insert", "Drawer", "Choosing"]);
        listedWords(holds);
        holds("the drawer's moves, as normal's on the rows",
            ["gg  G", "H  M  L", "Ctrl+D  Ctrl+U", "Ctrl+F  Ctrl+B", "3j  12G"]
                .filter(keys => !sections.Drawer.includes(keys.split("  ")[0])), []);
        holds("the find's next row, Ctrl+N", keyed("next row"), "Ctrl+N");
        holds("the find's caret", keyed("caret"), "\u2190  \u2192");
        holds("the find's word deleted", keyed("word"), "Ctrl+W");
        // The find's foot names Tab on a row holding parts, so its list
        // names it too, and Ctrl+] beside it (MVL7F9, #1979; VKCF3Z, 7f53e7b3).
        holds("the find lists Tab into a row's parts", sections.Find.includes("Tab"), true);
        holds("the find lists Ctrl+] to where a row lives", sections.Find.includes("Ctrl+]"),
            true);
        holds("normal lists Tab into a row's parts", sections.Normal.includes("Tab"), true);
        holds("normal's keys aren't listed on the find, where they type",
            ["j", "k", "gg", "ZZ"].filter(key => sections.Find.includes(key)),
            []);
        holds("the find's keys aren't listed in normal, where they do nothing",
            ["Ctrl+W", "Ctrl+N", "Ctrl+Y", "Home"].filter(key => sections.Normal.includes(key)),
            []);
        const facts = list.filter(line => line.heading === undefined);
        holds("every line the list gives has its keys",
            facts.filter(line => line.label === "").length, 0);
        // In normal, in a buffer, the arrows move the cursor (Joel,
        // 2026-11-05 12:1x): where nothing types, an arrow is bound only
        // beside its letter, as its motion, and the list names it never.
        const arrows = ["Up", "Down", "Left", "Right"];
        const letterOf = { Down: "j", Up: "k", Left: "h", Right: "l" };
        const motions = ["next", "previous", "letterBack", "letterNext", "left", "right"];
        holds("an arrow alone is only its letter's motion, the drawer's page's too", Keys.table
            .filter(one => ["normal", "visual", "words", "page", "pageVisual"]
                .includes(one.mode))
            .filter(one => one.keys.some(key => arrows.includes(key)
                && (!motions.includes(one.act) || !one.keys.includes(letterOf[key]))))
            .map(one => one.act), []);
        // Nor with Shift, nor as any key of a sequence, as the key language's
        // own check refuses (#1865), which can't see this table, the drawer's
        // page and its visual among them. A chord his
        // leader begins, as his Neovim's Space and an arrow, may overlay
        // Vim's own keys (Joel, 5 November): only those may hold an arrow.
        const arrowed = key => arrows.some(arrow => key === arrow || key === `Shift+${arrow}`);
        const led = keys => Array.isArray(keys) && keys[0] === "Space";
        holds("nor with Shift or in a sequence, the drawer's page's too", Keys.table
            .filter(one => ["normal", "visual", "words", "page", "pageVisual"]
                .includes(one.mode))
            .filter(one => one.keys.some(keys => !led(keys) && (Array.isArray(keys)
                ? keys.some(arrowed) : keys.startsWith("Shift+") && arrowed(keys))))
            .map(one => one.act), []);
        holds("and its sections name no arrow", sections.Drawer.concat(sections.Choosing)
            .filter(key => arrows.map(Listed.face).includes(key)), []);
        holds("normal's section names no arrow alone",
            sections.Normal.filter(key => arrows.map(Listed.face).includes(key)), []);
        // The narrowest plate's names sit whole at 16 figures; 14 leaves a
        // glyph's room.
        holds("every line's keys are short enough to sit whole",
            facts.filter(line => line.label.length > 14).map(line => line.label), []);
        for (const act of ["next", "previous", "first", "last", "viewTop", "viewMiddle",
            "viewBottom", "halfDown", "halfUp", "pageDown", "pageUp", "matchNext", "matchBack",
            "letterAgain", "letterAgainBack"])
            holds(`${act} repeats while held`, Keys.repeats(act), true);
        for (const act of ["run", "intoSaid", "outSaid", "out", "close", "quitSaid",
            "lastPlace", "into", "follow", "tagBack", "jumpBack", "jumpForward", "freshFind",
            "keys",
            "numbers", "insert", "change", ""])
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
            holds(`${what} goes to the row`, Operators.moved(act, count, place), want);
        holds("k goes up to an answer's headline",
            Operators.moved("previous", 0, Object.assign({}, place, { at: 0, highest: -1 })), -1);
        holds("gg goes to an answer's headline",
            Operators.moved("first", 0, Object.assign({}, place, { highest: -1 })), -1);
        holds("j with no rows stays where it is", Operators.moved("next", 0,
            { at: 0, highest: 0, last: -1, top: 0, bottom: 0, screen: 8 }), 0);
        holds("M in a view of three rows, the second", Operators.moved("viewMiddle", 0,
            Object.assign({}, place, { top: 0, bottom: 2, last: 2 })), 1);

        // n and N over five rows the find found: [what, from, count, going
        // down, where it goes and the end it wrapped past, as Vim's wrapscan].
        const found = [
            ["n", 2, 0, true, { to: 3, wrapped: "" }],
            ["n on the last, round to the first", 4, 0, true, { to: 0, wrapped: "bottom" }],
            ["N", 2, 0, false, { to: 1, wrapped: "" }],
            ["N on the first, round to the last", 0, 0, false, { to: 4, wrapped: "top" }],
            ["3n, round", 3, 3, true, { to: 1, wrapped: "bottom" }],
            ["7N, round", 1, 7, false, { to: 4, wrapped: "top" }]
        ];
        for (const [what, at, count, forward, want] of found)
            holds(`${what} goes to the match`,
                Finding.matchTo({ at, count, forward, last: 4, typed: "s" }), want);
        holds("n with nothing found has nowhere to go, as E35 says",
            Finding.matchTo({ at: 0, count: 0, forward: true, last: 4, typed: "" }),
            { to: 0, wrapped: "", none: "E35" });
        holds("nor where the find found nothing, as E486 says",
            Finding.matchTo({ at: -1, count: 0, forward: true, last: -1, typed: "zq" }),
            { to: -1, wrapped: "", none: "E486" });
        // f and F over the rows' names: [what, from, count, the letter and the
        // way, the row it goes to, -1 where none, never wrapping, as Vim's f].
        const rowNames = ["Slack", "Safari", "kitty", "signal", "Steam"];
        const lettered = [
            ["fs, either case", 0, 0, { letter: "s", forward: true }, 1],
            ["2fs, the second", 0, 2, { letter: "s", forward: true }, 3],
            ["fS, a capital's own case alone, as smartcase", 1, 0,
                { letter: "S", forward: true }, 4],
            ["Fs", 4, 0, { letter: "s", forward: false }, 3],
            ["fk", 0, 0, { letter: "k", forward: true }, 2],
            ["fs past the last s, nowhere", 3, 2, { letter: "s", forward: true }, -1],
            ["fz, nowhere", 0, 0, { letter: "z", forward: true }, -1],
            ["Fk from the first, nowhere", 0, 0, { letter: "k", forward: false }, -1]
        ];
        for (const [what, at, count, sought, want] of lettered)
            holds(`${what} goes to its row`,
                Finding.letterTo({ at, count, sought, names: rowNames }), want);
        holds("; is the last f again, the same way", Finding.again({ letter: "x", forward: false },
            "letterAgain"), { letter: "x", forward: false });
        holds("and , the other way", Finding.again({ letter: "x", forward: false },
            "letterAgainBack"), { letter: "x", forward: true });
        holds("f and F set what ; and , go again", Finding.again(null, "toLetterBack", "q"),
            { letter: "q", forward: false });

        holds("the line opens on the find", typed([]).mode, "find");
        holds("a letter on the find is the field's to type", typed(["x"]).ours, false);
        holds("a letter on the find types, never acts", typed(["i", "a", "c", "c", "u"]).acts,
            []);
        holds("a ] on the find is the field's to type", typed(["]"]).ours, false);
        holds("a prompt opens in insert", typed([], true).mode, "insert");
        holds("and Escape and i go back to it, not the find",
            typed(["Escape", "i"], true).mode, "insert");
        holds("a letter in normal is the line's, typing nothing", typed(["Escape", "x"]).ours,
            true);
        holds("Escape leaves the find for normal", typed(["Escape"]).mode, "normal");
        holds("Ctrl+C leaves it for normal too", typed(["Ctrl+c"]).mode, "normal");
        holds("cc leaves normal for the find", typed(["Escape", "c", "c"]).mode, "find");
        holds("i, a, I and A each go back to the find", ["i", "a", "I", "A"]
            .map(key => typed(["Escape", key]).mode), ["find", "find", "find", "find"]);
        holds("a c begun is the line's", typed(["Escape", "c"]).ours, true);
        holds("nothing begun is left after an act", typed(["Escape", "c", "c"]).standing,
            Keys.rest());
        holds("the field says Ctrl+Y for completing", Keys.keyOf("complete", "find"), "Ctrl+y");
        holds("and - for going up, as the foot names it", Keys.keyOf("out", "normal"), "-");
        holds("and Return for going in", Keys.keyOf("run", "normal"), "Return");
        // Return opens a program and Tab goes into what it holds, Joel's "keep"
        // (2026-11-07), so l never steps in.
        holds("and Tab for going into what a row holds", Keys.keyOf("into", "normal"), "Tab");
        holds("and l steps in nowhere", Keys.keyOf("intoSaid", "normal"), "l");

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
            holds(`Ctrl+W takes ${what}`, Typing.wordStart(text, at), want);

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
            holds(`the caret ${what}`, Typing.caretAfter(act, text, at), want);
        holds("normal's caret rests on the last letter", Typing.resting("sound", 5), 4);
        holds("and on an empty line at its start", Typing.resting("", 0), 0);
        holds("a find narrowed says how far", Typing.noteOf("", "3 found", true), "3 found");
        holds("but not where that isn't said", Typing.noteOf("", "3 found", false), "");
        holds("a place that can't answer says so in a count's stead",
            Typing.noteOf("not answering", "3 found", true), "not answering");
        holds("whatever's typed", Typing.noteOf("not answering", "", false), "not answering");

        operating(holds);

        editing(holds);
        drawerKeys(holds);
        drawerPage(holds);
        drawerSearch(holds);
        drawerVisual(holds);
        drawerRefusals(holds);
        drawerFaces(holds);
        drawerValueLines(holds);
        drawerPreviewHeights(holds);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
