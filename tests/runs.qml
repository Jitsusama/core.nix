// Drives the launcher's runs (modules/quickshell/launcher/Runs.qml)
// through Quickshell itself, over runs' records as the realm's mycelium
// node serves them, with niri, pi's sessions and the screen's sharing
// stood in. It lists the running fleet first, then the newest; a council
// is named for the change it read, its kind read from its id since the
// node's record keeps its own, unless the harness wrote it beside as
// run.kind; one the node says runs whose session has
// gone wears failure's mark; a run says how far along it is or what it
// cost, and how long it ran; the session that started one is named beside
// it, and Return goes to its window; stepping in lists its participants,
// and one with an answer opens it in a terminal; while the screen is
// shared no amount shows; and typing finds a run by the change or its id.
// It exits 0 once every case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher

ShellRoot {
    id: root

    function expect(what, holds, said) {
        if (!holds)
            throw new Error(`${what}: ${said}`);
    }

    function fail(message) {
        console.warn(`FAIL ${message}`);
        Qt.callLater(() => Qt.exit(1));
    }

    function ago(minutes) {
        return new Date(Date.now() - minutes * 60000).toISOString();
    }

    // Where a participant's result is, as the harness names it.
    readonly property string answer: "/state/subagent-workflow/runs/fleet-live/reviewers/b/result.json"

    // As the node serves them: a fleet at work, its session parked, one of
    // its two done; a council done, named for its change; and a fleet the
    // node still says runs, whose session is gone.
    readonly property var records: [
        {
            key: "run:fleet.fleet-live", kind: "run", id: "fleet.fleet-live", about: "fleet-live",
            parent: "s-leader", state: "running", since: ago(10), "pi.cost.usd": 1.25,
            members: [
                { id: "a", state: "running", model: "anthropic/opus", since: ago(10) },
                { id: "b", state: "complete", model: "anthropic/opus", since: ago(10),
                    ended: ago(0), "pi.cost.usd": 1.25, answer: root.answer }
            ]
        },
        {
            key: "run:review.council-20261022T100000000-000001", kind: "run",
            id: "review.council-20261022T100000000-000001", about: "core.nix#380",
            parent: "s-gone", state: "done", since: "2026-10-22T10:00:00.000Z",
            ended: "2026-10-22T10:03:00.000Z", "pi.cost.usd": 1.25,
            members: [
                { id: "tests", state: "complete", model: "anthropic/opus",
                    since: "2026-10-22T10:00:00.000Z", ended: "2026-10-22T10:02:00.000Z",
                    "pi.cost.usd": 0.5 },
                { id: "safety", state: "complete", model: "anthropic/opus",
                    since: "2026-10-22T10:00:30.000Z", ended: "2026-10-22T10:03:00.000Z",
                    "pi.cost.usd": 0.75 }
            ]
        },
        {
            key: "run:fleet.fleet-dead", kind: "run", id: "fleet.fleet-dead", about: "fleet-dead",
            parent: "s-gone", state: "running", since: "",
            members: [{ id: "y", state: "running", model: "anthropic/opus" }]
        }
    ]

    QtObject {
        id: theLauncher

        property bool open: false
    }

    Launcher.Runs {
        id: runs

        launcher: theLauncher
        settings: ({ name: "runs" })
    }

    Component.onCompleted: Qt.callLater(root.cases)

    function names(rows) {
        return rows.map(row => row.name).join(", ");
    }

    function cases() {
        try {
            Sessions.parked = [{ address: "LEADER", presence: { session: "s-leader" } }];
            runs.records = root.records;
            const all = runs.everything();
            expect("running first, then the newest",
                names(all) === "fleet-live, council on core.nix#380, fleet-dead", names(all));
            const [live, council, dead] = all;
            expect("a running one's mark", live.glyph === Theme.glyph.running, live.glyph);
            expect("how far along it is, and for how long",
                /^fleet · 1 of 2 done · 1[01] min$/.test(live.side), live.side);
            expect("a finished council's mark", council.glyph === Theme.glyph.done, council.glyph);
            expect("what it cost, and how long it ran", council.side === "$1.25 · 3 min", council.side);
            expect("its id below", council.note === "council-20261022T100000000-000001", council.note);
            expect("one whose session is gone has failed",
                dead.glyph === Theme.glyph.failed && dead.side === "fleet", `${dead.glyph} ${dead.side}`);
            expect("one with no session copies its id", council.verb === "copy", council.verb);
            expect("one whose session is parked has none to go to", live.verb === "copy", live.verb);
            expect("the foot's count", runs.counted() === "1 running, 3 today", runs.counted());

            const byChange = runs.search("380").rows;
            expect("found by its change", names(byChange) === "council on core.nix#380", names(byChange));
            // Its kind finds it, and names it.
            const byKind = runs.search("council").rows;
            expect("found by its kind", names(byKind) === "council on core.nix#380", names(byKind));
            const byId = runs.search("dead").rows;
            expect("found by its id", names(byId) === "fleet-dead", names(byId));

            // The session that started it, by its own id, which the run names.
            Sessions.running = [{ address: "LEADER", window: { id: 9 },
                presence: { session: "s-leader" } }];
            const led = runs.everything()[0];
            expect("its session named",
                /^fleet · 1 of 2 done · 1[01] min · LEADER$/.test(led.side) && led.verb === "go to", led.side);
            led.run();
            expect("going to its session", JSON.stringify(Niri.acted) === '{"FocusWindow":{"id":9}}',
                JSON.stringify(Niri.acted));

            const members = led.verbs();
            expect("stepping in", names(members) === "Go To, a, b", names(members));
            expect("one at work says so, its model and how long so far",
                /^running · opus · 1[01] min$/.test(members[1].side), members[1].side);
            expect("one done says its model, cost and time", members[2].side === "opus · $1.25 · 10 min",
                members[2].side);
            expect("one at work has no answer to read", members[1].verb === "close", members[1].verb);
            members[2].run();
            expect("reading its answer in a terminal",
                Niri.spawned[0] === "kitty" && Niri.spawned.includes("fleet-live \u00b7 b")
                    && Niri.spawned.some(word => word.startsWith("jq -r '.finalAssistantText"))
                    && Niri.spawned[Niri.spawned.length - 1] === root.answer,
                JSON.stringify(Niri.spawned));

            // Told a change, a finished run's row is the one made before while
            // nothing it says changed; only the running one's is made again,
            // its time moving. Every opening made all of Joel's day's runs
            // again, 279 rows, with only a few of them running (2026-11-03).
            const before = runs.everything();
            const counted = runs.tally.rows;
            Sessions.running = Sessions.running.concat([{ address: "OTHER", window: { id: 10 },
                presence: { session: "s-other" } }]);
            const after = runs.everything();
            expect("a finished run's row kept", after[1] === before[1] && after[2] === before[2],
                names(after));
            expect("a running one's made again", after[0] !== before[0], names(after));
            expect("one row made, for the running one", runs.tally.rows - counted === 1,
                `${runs.tally.rows - counted} rows`);

            Quiet.sharing = true;
            const hidden = runs.everything();
            expect("no amount while sharing",
                hidden.every(row => !row.side.includes("$"))
                    && hidden[0].verbs().every(row => !row.side.includes("$")),
                hidden.map(row => row.side).join(", "));

            // A run's own kind, where the harness wrote it beside the node's.
            runs.records = [{ key: "run:review.r7-1", kind: "run", "run.kind": "judge",
                id: "review.r7-1", about: "core.nix#381", parent: "s-gone", state: "done" }];
            const judged = runs.everything();
            expect("its kind as the harness wrote it", names(judged) === "judge on core.nix#381",
                names(judged));
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            fail(error.message);
        }
    }
}
