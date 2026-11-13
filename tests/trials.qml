// What the launcher keeps of each opening, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `the-launcher-keeps-its-trials`
// check copies modules/quickshell/launcher/Trials.js beside this file and
// runs `qml -platform offscreen trials.qml`. Each case is what a verdict on a
// home leans on: the keys counted by kind, the pick and its time, a miss
// only where letters were typed and nothing picked, and the oldest let go.
import QtQuick
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
        const step = Trials.stepped(inner, { began: 100, made: 105, pushed: 106, opened: 106,
            ended: 136 });
        expect("a step keeps the place's answers", step.answers === 20
            && step.composing.showing === 18, JSON.stringify(step));
        expect("a step's lists are its own and the place's", step.lists === 12
            && step.each.making === 5 && step.each.finding === 7 && step.each.holding === 1
            && step.rows === 3, JSON.stringify(step));
        // Its search's time before searching is said apart, as a key's is,
        // so its rows are only what stepping in did after the search: 30 ms
        // after opening are 2 finding, 8 answering, 4 before and 16 rows.
        const searched = Trials.stepped({ before: 4, lists: 2, rows: 0, answers: 8, each: {},
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
        const bare = Trials.stepped(null, { began: 0, made: 4, pushed: 4, opened: 5, ended: 9 });
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
