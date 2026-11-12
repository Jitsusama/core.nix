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

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
