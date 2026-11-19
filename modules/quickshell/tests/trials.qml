// What the launcher keeps of each opening, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `the-launcher-keeps-its-trials`
// check copies modules/quickshell/launcher/Trials.js, Stepped.js, Order.js
// and RowsKept.js beside this file and runs
// `qml -platform offscreen trials.qml`. Each case is what a verdict on a
// home leans on: the keys counted by kind, the pick and its time, a miss
// only where letters were typed and nothing picked, and the oldest let go.
import QtQuick
import "Stepped.js" as Stepped
import "Trials.js" as Trials

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
        const expect = (what, holds, said) => {
            if (!holds)
                failures.push(`${what}: ${said}`);
        };

        // Typing "sla", erasing a letter, arrowing once and picking Slack at
        // home, 900 ms in, and closing at 1000.
        let trial = Trials.begun(1000, "today");
        trial = Trials.retyped(trial, "", "s");
        trial = Trials.retyped(trial, "s", "sl");
        trial = Trials.retyped(trial, "sl", "sla");
        trial = Trials.retyped(trial, "sla", "sl");
        trial = Trials.counted(trial, "arrows");
        trial = Trials.counted(trial, "returns");
        trial = Trials.picked(trial, 1900, { key: "program:slack", kind: "program" },
            { typed: "sl", depth: 0 });
        const slack = Trials.ended(trial, 2000);
        expect("letters and erasures", slack.keys.letters === 3 && slack.keys.erased === 1,
            JSON.stringify(slack.keys));
        expect("an arrow and a Return", slack.keys.arrows === 1 && slack.keys.returns === 1,
            JSON.stringify(slack.keys));
        expect("the pick and its time", slack.picked === "program:slack" && slack.ms === 900
            && slack.typed === "sl" && slack.depth === 0 && slack.kind === "program",
            JSON.stringify(slack));
        expect("how long it was open", slack.open === 1000, slack.open);
        expect("a pick isn't a miss", !slack.missed, slack.missed);
        expect("a pick keeps no words it was sought by", slack.sought.length === 0,
            JSON.stringify(slack.sought));
        expect("the home it opened on", slack.home === "today", slack.home);

        // Each trial its own: counting one leaves the one before as it was.
        const before = Trials.begun(0, "today");
        Trials.counted(before, "steps");
        expect("counting copies", before.keys.steps === 0, before.keys.steps);
        expect("a kind not counted", Trials.counted(before, "jumps") === before, "changed");

        // Letters typed and nothing picked is a miss; nothing typed isn't.
        const typedAway = Trials.ended(Trials.retyped(Trials.begun(0, "four"), "", "tme"), 5);
        expect("typed and closed is a miss", typedAway.missed, typedAway.missed);
        const lookedAway = Trials.ended(Trials.counted(Trials.begun(0, "four"), "arrows"), 5);
        expect("looked and closed isn't", !lookedAway.missed, lookedAway.missed);

        // Typed, then stepped in or read a drawer, and left is a look, not a miss: 7 of 13
        // misses of 12 November pressed Return, mostly to step in and back out (1JQPPC,
        // 07938191; 80ECO2's one definition, 51158ed5).
        const typed = Trials.retyped(Trials.begun(0, "today"), "", "audio");
        const steppedAway = Trials.ended(Trials.looked(typed, "stepped"), 9, "audio");
        expect("stepped in and left is a look", !steppedAway.missed
            && steppedAway.looked === "stepped" && steppedAway.sought.length === 0,
            JSON.stringify(steppedAway));
        expect("the first look is kept", Trials.looked(Trials.looked(typed, "drawer"),
            "stepped").looked === "drawer", "replaced");

        // A step in is his look only by a key pressed since the opening began: a step no
        // key made, as an opening given its words or a key sent with no press time, says a
        // press of 0, and a key pressed before the opening began wasn't for it.
        expect("a step in by a key pressed since is a look",
            Trials.wentIn(typed, 500, 600).looked === "stepped", "not looked");
        expect("a step in with no press isn't a look",
            (Trials.wentIn(typed, 500, 0).looked ?? "") === "", "looked");
        expect("a step in by a key pressed before the opening isn't a look",
            (Trials.wentIn(typed, 500, 400).looked ?? "") === "", "looked");
        expect("a step in with no press stays a miss",
            Trials.ended(Trials.wentIn(typed, 500, 0), 9, "audio").missed, "not missed");
        const arrowed = Trials.counted(typed, "arrows");
        const read = Trials.ended(Trials.drawerShown(arrowed, 100, true), 1200, "audio");
        expect("a drawer out a second on a row arrowed to was read", !read.missed
            && read.looked === "drawer", JSON.stringify(read));
        const passed = Trials.ended(Trials.drawerShown(arrowed, 100, true), 600, "audio");
        expect("one passed over on the way wasn't", passed.missed, JSON.stringify(passed));
        const topmost = Trials.ended(Trials.drawerShown(typed, 100, true), 5000, "audio");
        expect("the top row's drawer, no arrow pressed, wasn't read", topmost.missed,
            JSON.stringify(topmost));
        const next = Trials.drawerShown(Trials.drawerShown(arrowed, 100, true), 300, true);
        expect("a drawer for the next row starts its time again",
            Trials.ended(next, 900, "audio").missed && next.drawerAt === 300,
            JSON.stringify(next));
        const putAway = Trials.drawerShown(Trials.drawerShown(arrowed, 100, true), 1500, false);
        expect("a drawer put away after a second was read", putAway.looked === "drawer"
            && putAway.drawerAt === -1, JSON.stringify(putAway));

        // A miss keeps what each search was before Joel gave up on it, so the next one says
        // what wasn't found: two searches of 12 November gave up after 51 and 62 s, and their
        // trials held only counts (1JQPPC, 77f84121). Erasing letter by letter is one search.
        let sought = Trials.begun(0, "today");
        for (const [was, now] of [["", "agnets"], ["agnets", "agnet"], ["agnet", "ag"],
            ["ag", "agents x"], ["agents x", ""], ["", "chrom"]])
            sought = Trials.retyped(sought, was, now, false);
        const gaveUp = Trials.ended(sought, 9, "chrom");
        expect("a miss keeps each search given up",
            JSON.stringify(gaveUp.sought) === JSON.stringify(["agnets", "agents x", "chrom"]),
            JSON.stringify(gaveUp.sought));
        // Never what's typed for an ask, which may be a talk's words or a password.
        let told = Trials.retyped(Trials.begun(0, "today"), "", "dear joel", true);
        told = Trials.ended(Trials.retyped(told, "dear joel", "", true), 9, "");
        expect("an ask's words are never kept", told.missed && told.sought.length === 0,
            JSON.stringify(told));
        // At most the last five, each cut to 60 letters.
        let searches = Trials.begun(0, "today");
        for (const one of ["a", "b", "c", "d", "e", "f", "g".repeat(80)])
            searches = Trials.retyped(Trials.retyped(searches, "", one, false), one, "", false);
        const kept = Trials.ended(searches, 9, "").sought;
        expect("the last five, each cut", kept.length === 5 && kept[0] === "c"
            && kept[4] === "g".repeat(60), JSON.stringify(kept));

        // An answer is counted by its kind, never its empty key, so Return on
        // one is a pick and not a miss: Ask by its source's word, Memory by
        // what it's about, and one saying neither as an answer still
        // (1JQPPC, 2026-11-11: Return on Ask was logged a miss).
        const asked = Trials.ended(Trials.picked(Trials.retyped(Trials.begun(0, "today"), "",
            "why?"), 10, { key: "", kind: "", counted: Trials.answerCounted("ask", "") },
            { typed: "why?", depth: 0 }), 20);
        expect("Ask is picked by its kind", asked.picked === "answer:ask" && !asked.missed,
            JSON.stringify(asked));
        expect("a sum by its kind", Trials.answerCounted("sum", "") === "answer:sum",
            Trials.answerCounted("sum", ""));
        expect("Memory by what it's about",
            Trials.answerCounted("", "machine:memory") === "answer:machine:memory",
            Trials.answerCounted("", "machine:memory"));
        expect("one saying neither, as an answer",
            Trials.answerCounted(undefined, undefined) === "answer",
            Trials.answerCounted(undefined, undefined));

        // A later pick in one opening is the one kept: a collection, then a
        // thing in it, a step in.
        let deeper = Trials.picked(Trials.begun(0, "today"), 10, { key: "collection:wifi" },
            { typed: "wi", depth: 0 });
        deeper = Trials.picked(deeper, 30, { key: "wifi:home" }, { typed: "", depth: 1 });
        expect("the last pick kept", deeper.picked === "wifi:home" && deeper.depth === 1
            && deeper.ms === 30, JSON.stringify(deeper));
        // Yet the first pick's time and how many there were are kept beside
        // it, since an opening kept open through 15 picks read as one 212 s
        // finding (7QEY1S, 2026-11-15, of Joel's 09:38 on 14 Nov).
        expect("the first pick's time kept, and the picks counted", deeper.firstMs === 10
            && deeper.picks === 2, JSON.stringify(deeper));
        const unpicked = Trials.begun(0, "today");
        expect("nothing picked yet", unpicked.firstMs === -1 && unpicked.picks === 0,
            JSON.stringify(unpicked));

        // Each key's search and frame are summed and their worst kept, and a
        // key slow enough to feel is kept with what took its time.
        let keyed = Trials.timed(Trials.begun(0, "today"), 2, 5, null);
        keyed = Trials.timed(keyed, 14, 20, { lists: 11, rows: 2, answers: 1 });
        expect("keys timed", keyed.timed.count === 2 && keyed.timed.searched === 16
            && keyed.timed.searchedMost === 14 && keyed.timed.drawn === 25
            && keyed.timed.drawnMost === 20, JSON.stringify(keyed.timed));
        expect("the felt key kept with its parts", keyed.timed.felt.length === 1
            && keyed.timed.felt[0].parts.lists === 11 && keyed.timed.felt[0].drawn === 20,
            JSON.stringify(keyed.timed.felt));
        expect("a frame's worth isn't felt", !Trials.felt(8) && Trials.felt(9), "felt");
        let many = Trials.begun(0, "today");
        for (let key = 0; key < 30; key += 1)
            many = Trials.timed(many, 20, 30, null);
        expect("only the first felt keys kept", many.timed.felt.length === 20
            && many.timed.count === 30, many.timed.felt.length);
        expect("a trial from before the timing",
            Trials.timed({ keys: {} }, 1, 2, null).timed.count === 1, "lost");

        // A step says where its rows went: the place's own search, finding
        // in 7 ms and showing in 20, keeps its answers and their composing,
        // beside the step's own lists and holding; the 3 ms left after
        // opening are its rows.
        const inner = { before: 0, lists: 7, rows: 0, answers: 20, each: {},
            composing: { asking: 0, placing: 2, showing: 18 } };
        const step = Stepped.stepped(inner, { began: 100, made: 105, pushed: 106, opened: 106,
            ended: 136 });
        expect("a step keeps the place's answers", step.answers === 20
            && step.composing.showing === 18, JSON.stringify(step));
        expect("a step's lists are its own and the place's", step.lists === 12
            && step.each.making === 5 && step.each.finding === 7 && step.each.holding === 1
            && step.rows === 3, JSON.stringify(step));
        // A walk's last step is timed from the walk's start, and the 20 ms
        // its places on the way took are said as passing among its lists,
        // never as its own making; a step of its own passes nothing.
        const walked = Stepped.stepped(inner, { walked: 80, began: 100, made: 105, pushed: 106,
            opened: 106, ended: 136 });
        expect("a walk's last step says its places on the way apart",
            walked.each.passing === 20 && walked.each.making === 5 && walked.lists === 32
            && step.each.passing === 0, JSON.stringify([walked, step]));
        // A walk stopped short says what it passed beside the search that
        // listed where it stopped, and one that passed nothing is as it was.
        const whereStopped = { before: 1, lists: 4, rows: 3, answers: 0, each: { finding: 4 } };
        const stopped = Stepped.passed(whereStopped, 15);
        expect("a walk stopped short says what it passed", stopped.lists === 19
            && stopped.each.passing === 15 && stopped.each.finding === 4
            && whereStopped.lists === 4 && Stepped.passed(whereStopped, 0) === whereStopped
            && Stepped.passed(null, 15) === null,
            JSON.stringify(stopped));
        // Its search's time before searching is said apart, as a key's is,
        // so its rows are only what stepping in did after the search: 30 ms
        // after opening are 2 finding, 8 answering, 4 before and 16 rows.
        const searched = Stepped.stepped({ before: 4, lists: 2, rows: 0, answers: 8, each: {},
            composing: null, beforeParts: { counting: 1, typed: 2, words: 0, readying: 1 } },
        { began: 0, made: 3, pushed: 4, opened: 5, ended: 35 });
        expect("a step says its search's before apart from its rows",
            searched.before === 4 && searched.beforeParts?.typed === 2 && searched.rows === 16,
            JSON.stringify(searched));
        // The key that starts typing says whose rows it made, most first, and
        // a key that made none says nothing of it.
        expect("a key says whose rows it made, most first",
            Trials.madeWhileTyping({ agents: 40, people: 176, quests: 0, notes: 40 })
                === " (made 256: people 176, agents 40, notes 40)",
            Trials.madeWhileTyping({ agents: 40, people: 176, quests: 0, notes: 40 }));
        expect("a key's making says whose rows it spent on, most first",
            Trials.madeOfKinds({ person: 2, password: 8, answer: 0, document: 2 })
            === " (password 8, document 2, person 2)",
            Trials.madeOfKinds({ person: 2, password: 8, answer: 0, document: 2 }));
        expect("a making no kind spent a millisecond on says none",
            Trials.madeOfKinds({ person: 0 }) === "" && Trials.madeOfKinds(undefined) === "",
            Trials.madeOfKinds({ person: 0 }));
        expect("a drawer's making says working out which, its acts and showing it",
            Trials.makingSplit(134, { acted: 0.4, shown: 21.6 })
                === " (112 working out which, 0 its acts, 22 showing it)",
            Trials.makingSplit(134, { acted: 0.4, shown: 21.6 }));
        expect("a split rounded past its making works out none",
            Trials.makingSplit(3, { acted: 1.6, shown: 1.6 })
                === " (0 working out which, 2 its acts, 2 showing it)",
            Trials.makingSplit(3, { acted: 1.6, shown: 1.6 }));
        expect("a drawer made with none of its own, or untold, says no split",
            Trials.makingSplit(0, { acted: 1, shown: 1 }) === ""
                && Trials.makingSplit(40, null) === "",
            Trials.makingSplit(40, null));
        expect("a key that made no rows says none", Trials.madeWhileTyping({ agents: 0 }) === ""
            && Trials.madeWhileTyping(undefined) === "", Trials.madeWhileTyping({ agents: 0 }));
        // A step's making says what it went on, most first, the rest being
        // what it did beside its acts, its parts and their order.
        const split = { acts: 20.4, parts: 59.6, ordering: 2.2 };
        expect("a step's making says its parts, its acts, their order and the rest",
            Stepped.steppedMaking(87, split)
                === " (its parts 60, its acts 20, the rest 5, ordering 2)",
            Stepped.steppedMaking(87, split));
        expect("a step that made nothing, or untold, says no split",
            Stepped.steppedMaking(0, split) === "" && Stepped.steppedMaking(40, null) === "",
            Stepped.steppedMaking(40, null));
        const made = Stepped.stepped(null, { began: 0, made: 87, pushed: 90, opened: 91,
            ended: 95, making: split });
        expect("a step keeps its making's split", made.each.making === 87
            && made.makingSplit === split, JSON.stringify(made));
        // A step out's holding says the rows each source made while it was
        // held, most first, and the time until it was held.
        const held = { runs: 99, agents: 3, quests: 2, rooms: 1 };
        expect("a step out's holding says its sources' rows, then until held",
            Stepped.steppedHolding(38, { untilHeld: 1.4, rows: held })
                === " (runs 99 rows, agents 3 rows, quests 2 rows, until held 1)",
            Stepped.steppedHolding(38, { untilHeld: 1.4, rows: held }));
        // Sources that made as many are said by name, so a line reads the
        // same each time its rows do.
        const tied = { wifi: 2, agents: 2, runs: 2, quests: 2 };
        expect("sources that made as many rows are said by name",
            Stepped.steppedHolding(9, { untilHeld: 0, rows: tied })
                === " (agents 2 rows, quests 2 rows, runs 2 rows)",
            Stepped.steppedHolding(9, { untilHeld: 0, rows: tied }));
        expect("a source that made one row says one row",
            Stepped.steppedHolding(5, { untilHeld: 0, rows: { runs: 1 } }) === " (runs 1 row)",
            Stepped.steppedHolding(5, { untilHeld: 0, rows: { runs: 1 } }));
        expect("a holding that made no rows says so, with no until held where it took none",
            Stepped.steppedHolding(6, { untilHeld: 0, rows: {} }) === " (no rows)",
            Stepped.steppedHolding(6, { untilHeld: 0, rows: {} }));
        expect("a holding of nothing, or untold, says no split",
            Stepped.steppedHolding(0, { untilHeld: 2, rows: held }) === ""
                && Stepped.steppedHolding(14, null) === "",
            Stepped.steppedHolding(14, null));
        const holding = { untilHeld: 2, rows: { runs: 99, now: 2 } };
        const out = Stepped.stepped(null, { began: 0, made: 0, pushed: 38, opened: 38,
            ended: 40, making: {}, holding });
        expect("a step out keeps the rows each source made while held",
            out.each.holding === 38 && out.holdingSplit === holding, JSON.stringify(out));
        expect("a step with no holding told keeps none", made.holdingSplit === null,
            JSON.stringify(made.holdingSplit));
        // The line names a step's three slowest parts, its holding with the
        // rows held, so a slow step out says what its holding made.
        const outLine = Stepped.slowest(Object.assign({}, out.each, { finding: 1 }), out);
        expect("a step out's line says the rows its holding made",
            outLine === ": holding 38 (runs 99 rows, now 2 rows, until held 2), finding 1",
            outLine);
        // A step's making says its split on the line, where a holding of no
        // split told says only its time.
        const madeLine = Stepped.slowest(Object.assign({}, made.each, { finding: 5 }), made);
        expect("a step's line says what its making went on",
            madeLine === ": making 87 (its parts 60, its acts 20, the rest 5, ordering 2),"
                + " finding 5, holding 3",
            madeLine);
        expect("a line with nothing a millisecond says no parts",
            Stepped.slowest({ holding: 0, making: 0 }, out) === ""
                && Stepped.slowest(null, null) === "",
            Stepped.slowest({ holding: 0, making: 0 }, out));
        const unstepped = { back: -1, from: 0, took: 0, whole: false, stepping: false };
        expect("a laying says the rows it built and took back",
            Trials.keptSaid({ built: 3, reused: 7, split: unstepped })
                === ": 3 rows built, 7 taken back",
            Trials.keptSaid({ built: 3, reused: 7, split: unstepped }));
        const parted = { back: 16, from: 9, took: 5.6, whole: true, stepping: true };
        expect("a step back out that split says what it took, where, and to which row",
            Trials.keptSaid({ built: 2, reused: 8, split: parted })
                === ": 2 rows built, 8 taken back;"
                + " 6 of the laying splitting them at row 10, back to row 17",
            Trials.keptSaid({ built: 2, reused: 8, split: parted }));
        const top = { back: 2, from: 0, took: 0, whole: true, stepping: true };
        expect("one put in afresh with nothing above its view says its view was at the top",
            Trials.keptSaid({ built: 1, reused: 9, split: top })
                === ": 1 rows built, 9 taken back; back to row 3, the view at the list's top",
            Trials.keptSaid({ built: 1, reused: 9, split: top }));
        const among = { back: 16, from: 0, took: 0, whole: false, stepping: true };
        expect("one that kept rows says it came back among them",
            Trials.keptSaid({ built: 1, reused: 9, split: among })
                === ": 1 rows built, 9 taken back; back to row 17 among rows kept",
            Trials.keptSaid({ built: 1, reused: 9, split: among }));
        const line = { back: -1, from: 0, took: 0, whole: false, stepping: true };
        expect("one that came back to no row says it stands above them",
            Trials.keptSaid({ built: 0, reused: 0, split: line }) === "; back above the rows",
            Trials.keptSaid({ built: 0, reused: 0, split: line }));
        const emptied = { back: -1, stepping: true, empty: true };
        expect("one that came back to a place with no rows left says so",
            Trials.keptSaid({ built: 0, reused: 0, split: emptied }) === "; back to no rows",
            Trials.keptSaid({ built: 0, reused: 0, split: emptied }));
        const unchanged = { back: 4, stepping: true, empty: false };
        expect("one whose rows all stood as they were says it came back among them",
            Trials.keptSaid({ fresh: 0, laying: 0, split: unchanged })
                === "; back to row 5 among rows kept",
            Trials.keptSaid({ fresh: 0, laying: 0, split: unchanged }));
        const none = { back: 16, from: 9, took: 5, whole: true, stepping: false };
        expect("a laying that drew none and was no step back says nothing, split or not",
            Trials.keptSaid({ built: 0, reused: 0, split: none }) === "",
            Trials.keptSaid({ built: 0, reused: 0, split: none }));
        const splitting = { fresh: 20, laying: 30, built: 2, reused: 8, split: parted };
        expect("a showing says its keeping, what else it took, then where a step came to",
            Trials.keptPart(splitting, ", 5 weighing their keys")
                === " (20 rows new, 30 laying them out: 2 rows built, 8 taken back,"
                + " 5 weighing their keys; 6 of the laying splitting them at row 10,"
                + " back to row 17)",
            Trials.keptPart(splitting, ", 5 weighing their keys"));
        // Whose rows the laying spent on follows the rows it drew, built and
        // taken back together, before what else the showing took.
        const dearLaying = Object.assign({}, splitting,
            { laidBy: { program: { rows: 3, ms: 9.6 }, on: { rows: 7, ms: 4.2 } } });
        expect("a showing says whose rows it laid right after the rows it drew",
            Trials.keptPart(dearLaying, ", 5 weighing their keys")
                === " (20 rows new, 30 laying them out: 2 rows built, 8 taken back,"
                + ' by kind "program" 3 rows in 10 and "on" 7 rows in 4, 5 weighing their keys;'
                + " 6 of the laying splitting them at row 10, back to row 17)",
            Trials.keptPart(dearLaying, ", 5 weighing their keys"));
        expect("a keeping that took time is shown",
            Trials.keptPart({ fresh: 2, laying: 0, split: unstepped })
                === " (2 rows new, 0 laying them out)",
            Trials.keptPart({ fresh: 2, laying: 0, split: unstepped }));
        expect("a step back out that kept every row as it stood is shown",
            Trials.keptPart({ fresh: 0, laying: 0, split: unchanged })
                === " (0 rows new, 0 laying them out; back to row 5 among rows kept)",
            Trials.keptPart({ fresh: 0, laying: 0, split: unchanged }));
        expect("a keeping that took none and was no step back isn't",
            Trials.keptPart({ fresh: 0, laying: 0, split: unstepped }, ", 5 weighing") === ""
                && Trials.keptPart(null) === "", "shown");
        expect("a showing says how long rows spent reading what they hold",
            Trials.readingSaid(12.6) === ", 13 reading what they hold"
                && Trials.readingSaid(0.4) === "" && Trials.readingSaid(undefined) === "",
            Trials.readingSaid(12.6));
        // A showing's own parts, as Launcher.qml's compose and search time
        // them apart: the putting in holds the laying and the weighing.
        const showingParts = { kept: { fresh: 3, laying: 3, built: 3, reused: 0,
            split: unstepped }, weighing: 5, listing: 6.4, putting: 45.2, selecting: 2.6 };
        expect("a showing says its listing, putting in past laying and weighing, and selecting",
            Trials.showingSaid(showingParts, ", 5 weighing their keys")
                === " (6 listing them, 37 putting them in besides laying and weighing,"
                + " 3 selecting and scrolling to it; 3 rows new, 3 laying them out:"
                + " 3 rows built, 0 taken back, 5 weighing their keys)",
            Trials.showingSaid(showingParts, ", 5 weighing their keys"));
        const keptShowing = Object.assign({}, showingParts, { kept: splitting });
        expect("a showing's keeping still ends on where a step back came to",
            Trials.showingSaid(keptShowing).startsWith(" (6 listing them, 10 putting them in")
                && Trials.showingSaid(keptShowing).endsWith(", 3 selecting and scrolling to it;"
                    + " 20 rows new, 30 laying them out: 2 rows built, 8 taken back;"
                    + " 6 of the laying splitting them at row 10, back to row 17)"),
            Trials.showingSaid(keptShowing));
        const bareShowing = { kept: { fresh: 0, laying: 0, split: unstepped }, weighing: 2,
            listing: 0.4, putting: 14.2, selecting: 0.4 };
        expect("a showing that laid nothing says its parts and what else it took",
            Trials.showingSaid(bareShowing, ", 2 weighing")
                === " (12 putting them in besides laying and weighing, 2 weighing)",
            Trials.showingSaid(bareShowing, ", 2 weighing"));
        expect("a showing with no part of a millisecond, or none timed, says what keptPart does",
            Trials.showingSaid(Object.assign({}, bareShowing, { putting: 2.4 })) === ""
                && Trials.showingSaid({ kept: splitting }, ", 5 weighing their keys")
                    === Trials.keptPart(splitting, ", 5 weighing their keys"),
            Trials.showingSaid({ kept: splitting }, ", 5 weighing their keys"));
        expect("a showing that laid nothing and split nothing still says its weighing",
            Trials.showingSaid(Object.assign({}, bareShowing, { putting: 2.4 }), ", 2 weighing")
                === " (2 weighing)",
            Trials.showingSaid(Object.assign({}, bareShowing, { putting: 2.4 }), ", 2 weighing"));
        // A drawer joined the key: 4 of it made as the rows went in, 3 as
        // the selection moved, each taken from its part and said as one.
        const joinedShowing = Object.assign({}, showingParts,
            { drawer: { putting: 4, selecting: 3 }, selecting: 4.2 });
        expect("a drawer that joined the key is said apart from putting in and selecting",
            Trials.showingSaid(joinedShowing).startsWith(" (6 listing them, 33 putting them in"
                + " besides laying and weighing, 7 making its drawer,"
                + " 1 selecting and scrolling to it; 3 rows new"),
            Trials.showingSaid(joinedShowing));
        expect("a part that rounds to a millisecond is said, from half of one, and less isn't",
            Trials.showingSaid({ kept: bareShowing.kept, listing: 0.5, selecting: 0.4 })
                === " (1 listing them)",
            Trials.showingSaid({ kept: bareShowing.kept, listing: 0.5, selecting: 0.4 }));
        // The keeping's laying is whole, 3 of a fine 3.45, so the putting in
        // past it is 1.45, said 1, where the whole one would say 2 and both
        // taken away would leave less than nothing.
        const fineLaying = Object.assign({}, bareShowing, { kept: showingParts.kept,
            laying: 3.45, weighing: 0, putting: 4.9 });
        expect("a showing's putting in is told less its laying as finely timed",
            Trials.showingSaid(fineLaying)
                .startsWith(" (1 putting them in besides laying and weighing; 3 rows new"),
            Trials.showingSaid(fineLaying));
        // A drawer timed longer than the selecting it fell in leaves less than
        // nothing, which is said, as a part timed outside its window.
        const overDrawn = { kept: bareShowing.kept, selecting: 1,
            drawer: { putting: 0, selecting: 3.2 } };
        expect("a part left below nothing is said rather than dropped as small",
            Trials.showingSaid(overDrawn)
                === " (3 making its drawer, -2 selecting and scrolling to it)",
            Trials.showingSaid(overDrawn));
        expect("a showing's weighing is said once it rounds to a millisecond",
            Trials.weighingSaid(12.6) === ", 13 weighing their keys"
                && Trials.weighingSaid(0.4) === "" && Trials.weighingSaid(undefined) === "",
            Trials.weighingSaid(12.6));
        // The trials file keeps every time whole, as it always has, with no
        // fine laying beside the keeping's own.
        // The journal's line is said from the same parts after, so they're
        // left fine.
        const filing = { lists: 1, composing: { asking: 2, listing: 6.41, laying: 3.45,
            weighing: 4.6, drawer: { putting: 1.26, selecting: 0 } } };
        const filed = Trials.timed(Trials.begun(0, "today"), 20, 30, filing)
            .timed.felt[0].parts.composing;
        expect("the trials file keeps a showing's parts in whole milliseconds, its own fine",
            filing.composing.laying === 3.45 && filing.composing.listing === 6.41
                && JSON.stringify(filed) === '{"asking":2,"listing":6,"weighing":5,'
                + '"drawer":{"putting":1,"selecting":0}}',
            JSON.stringify(filed));
        expect("a key held past its bound says what it was behind",
            Trials.behindSaid({ behind: 69, behindOn: "syncing 40, 29 unnamed" })
                === " (69 of it behind the shell's other work: syncing 40, 29 unnamed)",
            Trials.behindSaid({ behind: 69, behindOn: "syncing 40, 29 unnamed" }));
        expect("one with nothing named says only how far",
            Trials.behindSaid({ behind: 3, behindOn: "" })
                === " (3 of it behind the shell's other work)"
                && Trials.behindSaid({ behind: 0, behindOn: "" }) === "",
            Trials.behindSaid({ behind: 3, behindOn: "" }));
        const bare = Stepped.stepped(null, { began: 0, made: 4, pushed: 4, opened: 5, ended: 9 });
        expect("a step with no search said", bare.answers === 0 && bare.rows === 4
            && bare.composing === null, JSON.stringify(bare));

        // A close says its own opening alone, a line, whatever came before.
        expect("one opening a line", Trials.said({ at: 1, home: "today" })
            === '{"at":1,"home":"today"}\n', JSON.stringify(Trials.said({ at: 1 })));

        // As the shell starts, the oldest lines go once there are too many,
        // none while there aren't, and the list kept before lines is carried
        // into them only while there are none.
        const lines = '{"at":1}\n{"at":2}\n{"at":3}\n';
        expect("the oldest let go", Trials.carried(lines, null, 2) === '{"at":2}\n{"at":3}\n',
            JSON.stringify(Trials.carried(lines, null, 2)));
        expect("nothing to cut", Trials.carried(lines, null, 3) === null, "rewritten");
        expect("nothing at all", Trials.carried("", null, 3) === null, "rewritten");
        const listed = [{ at: 1 }, { at: 2 }, { at: 3 }];
        expect("the list carried", Trials.carried("", listed, 2) === '{"at":2}\n{"at":3}\n',
            JSON.stringify(Trials.carried("", listed, 2)));
        expect("the list left once there are lines",
            Trials.carried('{"at":9}\n', listed, 2) === null, "carried again");
        expect("a list that isn't one", Trials.carried("", { at: 1 }, 2) === null, "carried");

        // At 120 Hz: a frame 8 ms after the one before is on time, one 25 ms
        // after missed two, and one half a second after is the launcher at
        // rest moving again, as a caret's blink or a key after a pause.
        const pace = Trials.unpaced();
        for (const at of [1000, 1008, 1016, 1041, 1049, 1549, 1557])
            Trials.drew(pace, at, 120);
        expect("frames while moving", pace.drawn === 5, pace.drawn);
        expect("the late one", pace.late === 1 && pace.longest === 25, JSON.stringify(pace));
        const moved = Trials.paced(Trials.begun(0, "today"), pace);
        expect("kept with the opening", moved.frames.drawn === 5 && moved.frames.late === 1
            && moved.frames.longest === 25, JSON.stringify(moved.frames));
        // A screen that doesn't say its rate is taken at 60 Hz: 20 ms is on
        // time there, and an opening that never moved keeps no frames.
        const unsaid = Trials.unpaced();
        Trials.drew(unsaid, 0, 0);
        Trials.drew(unsaid, 20, 0);
        expect("60 Hz when unsaid", unsaid.drawn === 1 && unsaid.late === 0,
            JSON.stringify(unsaid));
        const still = Trials.begun(0, "today");
        expect("nothing moved", Trials.paced(still, Trials.unpaced()) === still, "framed");

        // The frames before a key's own, newest first, heard at 1000: two
        // began after it was heard, its own work too late for them; four in
        // the 100 ms before, 900 among them, as a sliding name draws; two
        // before that are at rest.
        const inFlight = Trials.framesBefore([1012, 1000, 996, 950, 901, 900, 899, 400], 1000);
        expect("frames since a key and just before it", inFlight !== null
            && inFlight.since === 2 && inFlight.still === 4, JSON.stringify(inFlight));
        expect("said in its line", Trials.framesBeforeSaid(inFlight)
            === ", after 2 other frames since it was heard and 4 in the 100 ms before",
            Trials.framesBeforeSaid(inFlight));
        // Where the oldest kept is inside the 100 ms, frames before it went
        // unread, so the count is a floor; one frame is one.
        const shortKept = Trials.framesBeforeSaid(Trials.framesBefore([1004, 950], 1000));
        expect("a short history is a floor", shortKept
            === ", after 1 other frame since it was heard and at least 1 in the 100 ms before",
            shortKept);
        // Where every frame read came since, none of the 100 ms before was
        // read, and that half goes unsaid. Each floor holds at its edge.
        const allSince = Trials.framesBeforeSaid(Trials.framesBefore([1010, 1000], 1000));
        expect("all since is a floor", allSince
            === ", after at least 2 other frames since it was heard", allSince);
        const stillEdge = Trials.framesBeforeSaid(Trials.framesBefore([950, 900], 1000));
        expect("the oldest at the 100 ms is a floor", stillEdge
            === ", after 0 other frames since it was heard and at least 2 in the 100 ms before",
            stillEdge);
        expect("none read, none said", Trials.framesBefore([], 1000) === null
            && Trials.framesBeforeSaid(null) === "", "said");
        expect("no time heard, none said", Trials.framesBefore([1012], undefined) === null,
            "counted");
        // Read back from the key's frame 20, never its own, to the oldest
        // the ring keeps; and from 30, up to the slot a newer frame took
        // (27's), no further, so what's read runs unbroken.
        function timesOf(frame) {
            return frame === 20 || frame === 27 ? {} : { rendering: 1000 + frame };
        }
        const begunAt = Trials.begunBefore(20, timesOf);
        expect("begun before, newest first", JSON.stringify(begunAt)
            === JSON.stringify([1019, 1018, 1017, 1016, 1015, 1014, 1013, 1012, 1011, 1010,
                1009, 1008, 1007, 1006, 1005]), JSON.stringify(begunAt));
        const taken = Trials.begunBefore(30, timesOf);
        expect("begun before, to a slot taken", JSON.stringify(taken)
            === JSON.stringify([1029, 1028]), JSON.stringify(taken));
        const early = Trials.begunBefore(3, timesOf);
        expect("begun before the first frames", JSON.stringify(early)
            === JSON.stringify([1002, 1001]), JSON.stringify(early));

        // Each key's time from its press to the screen is kept with the
        // opening, and how many were never presented, so health can pool a
        // day's keys (1JQPPC, 2026-11-12 [e1f448f2]).
        const shown = Trials.presented(still, [52, 48, 108], 1);
        expect("each key presented", JSON.stringify(shown.presented) === "[52,48,108]"
            && shown.unpresented === 1, JSON.stringify(shown));
        expect("an opening with no keys keeps none", Trials.presented(still, [], 0) === still,
            "presented");
        expect("nor does none at all", Trials.presented(null, [52], 0) === null, "a trial");

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
