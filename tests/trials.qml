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

        // The oldest go once there are too many; none while there aren't.
        const three = Trials.kept(Trials.kept(Trials.kept([], { at: 1 }, 2), { at: 2 }, 2),
            { at: 3 }, 2);
        expect("the oldest let go", three.map(one => one.at).join(" ") === "2 3",
            JSON.stringify(three));
        expect("kept from nothing", Trials.kept(null, { at: 1 }, 2).length === 1, "lost");

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
