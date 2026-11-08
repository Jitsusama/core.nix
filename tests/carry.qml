// How the launcher reads `mycelium carry`'s progress, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `the-launcher-follows-a-carry` check copies
// modules/quickshell/launcher/Carry.js beside this file and runs
// `qml -platform offscreen carry.qml`. Each line mycelium prints says one
// step as it starts, ends or fails; the foot says the step that's under way,
// and what was carried, or why not.
import QtQuick
import "Carry.js" as Carry

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

    function check(failures, what, got, want) {
        const said = JSON.stringify(got);
        if (said !== JSON.stringify(want))
            failures.push(`${what}: got ${said}, wanted ${JSON.stringify(want)}`);
    }

    readonly property var words: ({
            carrying: "Carrying {quest} to {machine}",
            carried: "Carried {quest} to {machine}",
            there: "its window is there",
            thereMany: "its {n} windows are there",
            failed: "{quest} not carried",
            stopped: "mycelium stopped before it finished",
            steps: {
                find: "finding its work",
                push: "pushing {tree}",
                share: "sharing it",
                start: "starting there",
                close: "closing here"
            },
            done: {
                push: "its trees are pushed",
                share: "it's shared",
                start: "its session is started there"
            }
        })

    function line(fields) {
        return JSON.stringify(fields);
    }

    function heardAll(lines) {
        return lines.reduce((state, one) => Carry.heard(state, one), Carry.begun("reef",
            "lantern"));
    }

    function run() {
        const failures = [];
        const begun = Carry.begun("reef", "lantern");
        check(failures, "before mycelium says anything, the foot says what's carried",
            Carry.told(begun, words), ["Carrying reef to lantern"]);
        check(failures, "and it hasn't ended", Carry.ended(begun), false);

        const finding = heardAll([line({ step: "find", state: "doing", said: "3 trees" })]);
        check(failures, "a step starting is said beside what's carried",
            Carry.told(finding, words), ["Carrying reef to lantern", "finding its work"]);

        const pushing = heardAll([
            line({ step: "find", state: "done", said: "3 trees" }),
            line({ step: "push", state: "doing", tree: "/home/ada/src/reef", said: "" })
        ]);
        check(failures, "a tree being pushed is named by its folder",
            Carry.told(pushing, words), ["Carrying reef to lantern", "pushing reef"]);

        const skipped = heardAll([
            line({ step: "push", state: "doing", tree: "/home/ada/src/reef", said: "" }),
            line({ step: "push", state: "skipped", tree: "/home/ada/src/reef",
                said: "nothing to push" })
        ]);
        check(failures, "a step done or skipped leaves the foot as it was until the next",
            Carry.told(skipped, words), ["Carrying reef to lantern", "pushing reef"]);

        const failed = heardAll([
            line({ step: "push", state: "done", tree: "/home/ada/src/reef", said: "" }),
            line({ step: "share", state: "skipped", said: "shared already" }),
            line({ step: "start", state: "doing", said: "" }),
            line({ step: "start", state: "failed", said: "lantern can't reach github.com" }),
            line({ step: "close", state: "doing", said: "" })
        ]);
        check(failures, "a failure says what wasn't carried, mycelium's reason and what was done",
            Carry.told(failed, words),
            ["reef not carried", "lantern can't reach github.com", "its trees are pushed"]);
        const early = heardAll([line({ step: "find", state: "failed", said: "no such quest" })]);
        check(failures, "and only what was carried and why when nothing was done",
            Carry.told(early, words), ["reef not carried", "no such quest"]);
        check(failures, "it has ended", Carry.ended(failed), true);
        check(failures, "and nothing after it is read", failed.step, "start");

        const carried = heardAll([
            line({ step: "close", state: "done", said: "" }),
            line({ step: "carried", state: "done", machine: "lantern", sessions: [{
                address: "Q4RT2K", held: true, attach: "mycelium --realm home attach Q4RT2K" }] })
        ]);
        check(failures, "the last line says where it went",
            Carry.told(carried, words),
            ["Carried reef to lantern", "its window is there"]);
        check(failures, "it has ended well", [Carry.ended(carried), carried.failed], [true, ""]);
        check(failures, "and keeps each session's address there",
            carried.sessions.map(session => session.address), ["Q4RT2K"]);
        const several = heardAll([line({ step: "carried", state: "done", machine: "lantern",
            sessions: [{ address: "Q4RT2K" }, { address: "M7LX0P" }] })]);
        check(failures, "several sessions carried say how many windows are there",
            Carry.told(several, words), ["Carried reef to lantern", "its 2 windows are there"]);

        const garbled = heardAll(["not json", line({ said: "no step" }),
            line({ step: "share", state: "doing", said: "" })]);
        check(failures, "a line that isn't a step is passed over",
            Carry.told(garbled, words), ["Carrying reef to lantern", "sharing it"]);

        const quit = Carry.stopped(heardAll([line({ step: "share", state: "doing", said: "" })]),
            words);
        check(failures, "mycelium stopping without a last line is a failure",
            Carry.told(quit, words),
            ["reef not carried", "mycelium stopped before it finished"]);
        const kept = Carry.stopped(carried, words);
        check(failures, "but not once it said it carried it", kept.failed, "");

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            Qt.exit(1);
        } else {
            console.log(`carry: every case holds`);
            Qt.exit(0);
        }
    }
}
