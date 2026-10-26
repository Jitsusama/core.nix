// Drives the launcher's runs (modules/quickshell/launcher/Runs.qml)
// through Quickshell itself, over a harness's state written beside this
// and read by the real pi-runs, with niri, pi's sessions and the screen's
// sharing stood in. It lists the running fleet first, then the newest,
// leaving out one older than a day; a council is named for the change it
// read; one whose process is gone wears failure's mark; a run says how
// far along it is or what it cost, and how long it ran; the session that
// started one is named beside it, and Return goes to its window; stepping
// in lists its participants, and one with an answer opens it in a
// terminal; while the screen is shared no amount shows; and typing finds
// a run by the change or its id. It exits 0 once every case holds, and
// otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    readonly property string state: Quickshell.env("PI_STATE")
    property string waiting: "the first reading"

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    QtObject {
        id: launcher

        property bool open: false
    }

    Launcher.Runs {
        id: runs

        launcher: launcher
        settings: ({ name: "runs" })
        // After the change is done, since a case changes them again.
        onAllChanged: {
            if (root.waiting === "the first reading" && runs.all.length > 0)
                Qt.callLater(root.first);
        }
    }

    // Nothing here waits forever: a case that never comes fails, by name.
    Timer {
        running: true
        interval: 30000
        onTriggered: root.fail(`never came: ${root.waiting}`)
    }

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function first() {
        if (root.waiting !== "the first reading")
            return;
        root.waiting = "the cases";
        try {
            const all = runs.everything();
            expect("running first, then the newest, none older than a day",
                names(all) === "fleet-live, council on core.nix#380, fleet-dead", names(all));
            const [live, council, dead] = all;
            expect("a running one's mark", live.glyph === Theme.glyph.running, live.glyph);
            expect("how far along it is, and for how long",
                /^fleet · 1 of 2 done · 1[01] min$/.test(live.side), live.side);
            expect("a finished council's mark", council.glyph === Theme.glyph.done, council.glyph);
            expect("what it cost, and how long it ran", council.side === "$1.25 · 3 min", council.side);
            expect("its id below", council.note === "council-20261022T100000000-000001", council.note);
            expect("one whose process is gone has failed",
                dead.glyph === Theme.glyph.failed && dead.side === "fleet", `${dead.glyph} ${dead.side}`);
            expect("one with no session copies its id", council.verb === "copy", council.verb);
            expect("the foot's count", runs.counted() === "1 running, 3 today", runs.counted());

            const byChange = runs.search("380").rows;
            expect("found by its change", names(byChange) === "council on core.nix#380", names(byChange));
            // Its kind finds it, and names it.
            const byKind = runs.search("council").rows;
            expect("found by its kind", names(byKind) === "council on core.nix#380", names(byKind));
            const byId = runs.search("dead").rows;
            expect("found by its id", names(byId) === "fleet-dead", names(byId));

            // The session that started it, by its pid.
            Sessions.running = [{ address: "LEADER", window: { id: 9 }, presence: { owner: { pid: 4242 } } }];
            const led = runs.everything()[0];
            expect("its session named",
                /^fleet · 1 of 2 done · 1[01] min · LEADER$/.test(led.side) && led.verb === "switch", led.side);
            led.run();
            expect("going to its session", JSON.stringify(Niri.acted) === '{"FocusWindow":{"id":9}}',
                JSON.stringify(Niri.acted));

            const members = led.verbs();
            expect("stepping in", names(members) === "Switch, a, b", names(members));
            expect("one at work says what it does", members[1].side === "reading checks.nix · opus",
                members[1].side);
            expect("one done says its model, cost and time", members[2].side === "opus · $1.25 · 10 min",
                members[2].side);
            expect("one at work has no answer to read", members[1].verb === "close", members[1].verb);
            members[2].run();
            expect("reading its answer in a terminal",
                Niri.spawned[0] === "kitty" && Niri.spawned.includes("fleet-live \u00b7 b")
                    && Niri.spawned.some(word => word.startsWith("pi-runs answer"))
                    && Niri.spawned[Niri.spawned.length - 1]
                        === `${root.state}/subagent-workflow/runs/fleet-live/reviewers/b/result.json`,
                JSON.stringify(Niri.spawned));

            Quiet.sharing = true;
            const hidden = runs.everything();
            expect("no amount while sharing",
                hidden.every(row => !row.side.includes("$"))
                    && hidden[0].verbs().every(row => !row.side.includes("$")),
                hidden.map(row => row.side).join(", "));
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            fail(error.message);
        }
    }
}
