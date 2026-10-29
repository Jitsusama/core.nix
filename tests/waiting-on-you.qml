// What truly waits on Joel, run in Qt's own JavaScript engine, the one the
// shell runs it in. The `waiting-on-you-is-true` check copies
// Waiting.js beside this file and runs `qml -platform offscreen
// waiting-on-you.qml`. The files are those the agents kept on 2026-10-26,
// cut to what's read, when Waiting on You counted eleven things and three
// were Joel's to do.
import QtQuick
import "Waiting.js" as Waiting

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

    // When Joel said his word on an item.
    readonly property real at: Date.parse("2026-10-26T20:30:00Z")

    readonly property var queue: ({
        updated: "2026-10-26T13:40:00-04:00",
        orchestrator: "P9R1KI",
        items: [
            { id: "laptop-restart", title: "One laptop restart with the lid open",
                when: "not yet", minutes: 10, window: null },
            { id: "guest-watch", title: "The guest's watch", when: "now", minutes: 0 }
        ]
    })

    readonly property var files: ({
        "/state/mission/waiting/19AMVE.json": {
            updated: "2026-10-26T20:26:12.665Z",
            quest: "QEST-20260924-19AMVE",
            items: [{ id: "publish-pp-1802", title: "Publish profiles-processor#1802",
                when: "now", minutes: 2, ready: true, window: "π - Aquifer - karst" }]
        },
        "/state/mission/waiting/2HN63P.json": {
            updated: "2026-10-26T14:20:00Z",
            quest: "QEST-20261011-2HN63P",
            items: [{ id: "guest-mics", title: "The guest's microphones", ready: false,
                when: "a later sitting with the guest" }]
        },
        "/state/mission/waiting/M4RL61.json": {
            updated: "2026-10-26T14:20:37.793Z",
            quest: "QEST-20261025-M4RL61",
            items: [
                { id: "suspend-sitting", title: "Suspend sitting on battery", ready: false },
                { id: "reboot-sitting", title: "One laptop restart with the lid  OPEN",
                    ready: false }
            ]
        },
        "/state/mission/waiting/QGB4ON.json": {
            updated: "2026-10-26T18:25:40.137Z",
            quest: "QEST-20261025-QGB4ON",
            items: [{ id: "tuple-pointer", title: "A Tuple call to the Mac", ready: false,
                when: "about 11:00" }]
        }
    })

    readonly property var active: ({
        "QEST-20260924-19AMVE": { status: "active", title: "Aquifer" },
        "QEST-20261011-2HN63P": { status: "active", title: "Guests" },
        "QEST-20261025-M4RL61": { status: "active", title: "Hardware" },
        "QEST-20261025-QGB4ON": { status: "active", title: "Calls" }
    })
    // Joel has said nothing yet.
    readonly property var unsaid: ({})

    function run() {
        const failures = [];
        const expect = (name, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${name}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };
        const ids = list => list.map(item => item.id);
        const with_ = (base, more) => Object.assign({}, base, more);

        // Now holds only what Joel can do now, the orchestrator's first;
        // the rest wait under Later in the owners' order, and the quest's
        // copy of the orchestrator's restart is shown once.
        const today = Waiting.sorted(queue, files, active, unsaid);
        expect("now is what Joel can do now", ids(today.now),
            ["guest-watch", "publish-pp-1802"]);
        expect("later is what he can't yet, each once", ids(today.later),
            ["laptop-restart", "guest-mics", "suspend-sitting", "tuple-pointer"]);
        expect("a when of now is ready where no ready is given",
            today.now[0].ready, true);
        expect("an item is from who keeps it", today.now[1].from, "19AMVE");
        expect("and its quest is its file's", today.now[1].quest, "QEST-20260924-19AMVE");
        expect("its identity is its quest and id", Waiting.identity(today.now[1]),
            "QEST-20260924-19AMVE:publish-pp-1802");
        expect("it's said since its file was", today.now[1].since,
            Date.parse("2026-10-26T20:26:12.665Z"));
        expect("or since it says it was",
            Waiting.sorted({ items: [{ title: "x", since: "2026-10-26T10:00:00Z" }] }, {}, {},
                unsaid).later[0].since, Date.parse("2026-10-26T10:00:00Z"));

        // A quest's copy the queue says it covers goes, by quest or address.
        const covering = with_(queue, { items: [with_(queue.items[0],
            { title: "Restart", covers: ["QEST-20261025-M4RL61:reboot-sitting",
                "M4RL61:suspend-sitting"] })] });
        expect("what the queue covers is shown once",
            ids(Waiting.sorted(covering, files, active, unsaid).later),
            ["laptop-restart", "guest-mics", "tuple-pointer"]);

        // A quest that's ended, or whose README is gone, keeps nothing
        // waiting; one not read yet keeps all it had.
        expect("a concluded quest's items go",
            ids(Waiting.sorted(queue, files, with_(active,
                { "QEST-20261025-QGB4ON": { status: "concluded" } }), unsaid).later),
            ["laptop-restart", "guest-mics", "suspend-sitting"]);
        expect("a retired one's too",
            ids(Waiting.sorted(queue, files, with_(active,
                { "QEST-20260924-19AMVE": { status: "retired" } }), unsaid).now),
            ["guest-watch"]);
        expect("and one whose README is gone",
            ids(Waiting.sorted(queue, files, with_(active,
                { "QEST-20261011-2HN63P": null }), unsaid).later),
            ["laptop-restart", "suspend-sitting", "tuple-pointer"]);
        expect("a quest not read yet keeps its items",
            Waiting.sorted(queue, files, {}, unsaid).later.length, 4);

        // Nothing goes by its age: a file not said again in days still
        // holds what it holds, until an event takes it.
        expect("an old file's ready item is still for now",
            ids(Waiting.sorted(null, { "/w/OLD.json": { updated: "2026-10-01T00:00:00Z",
                items: [{ id: "old", title: "Old", ready: true }] } }, {}, unsaid).now),
            ["old"]);

        // Joel's word stays: one done is gone, one put off waits under
        // Later until its owner says something new of it.
        const publish = today.now[1];
        const finished = Waiting.settle(unsaid, publish, "done", at);
        expect("done is kept by its identity", finished.done,
            { "QEST-20260924-19AMVE:publish-pp-1802": at });
        expect("one done is gone", ids(Waiting.sorted(queue, files, active, finished).now),
            ["guest-watch"]);
        expect("and stays gone when its file is said again",
            ids(Waiting.sorted(queue, with_(files, { "/state/mission/waiting/19AMVE.json":
                with_(files["/state/mission/waiting/19AMVE.json"],
                    { updated: "2026-10-26T21:00:00Z" }) }), active, finished).now),
            ["guest-watch"]);
        const putOff = Waiting.settle(finished, today.now[0], "notNow", at);
        expect("what's done before stays done", Object.keys(putOff.done).length, 1);
        const off = Waiting.sorted(queue, files, active, putOff);
        expect("one put off leaves now", ids(off.now), []);
        expect("for Later, first, as its owner ordered it", ids(off.later).slice(0, 2),
            ["laptop-restart", "guest-watch"]);
        expect("saying it was put off", off.later[1].putOff, true);
        const resaid = with_(queue, { items: [queue.items[0],
            with_(queue.items[1], { why: "they're here" })] });
        expect("and comes back when its owner says something new of it",
            ids(Waiting.sorted(resaid, files, active, putOff).now), ["guest-watch"]);

        // Files half written or of another shape say nothing, and break
        // nothing else.
        expect("a file that isn't an object adds nothing",
            Waiting.sorted(queue, { "/w/X.json": null, "/w/Y.json": { items: [{}] } },
                active, unsaid).now.length, 1);
        expect("a bare list is a file's items",
            ids(Waiting.sorted(null, { "/w/ABC.json": [{ id: "a", title: "A", ready: true }] },
                {}, null).now), ["a"]);
        expect("the orchestrator is its address, or the steward it once was",
            Waiting.orchestratorOf({ steward: "P9R1KI" }), "P9R1KI");

        // The quests the files name, and a README's status.
        expect("the quests named", Waiting.quests(queue, files),
            ["QEST-20260924-19AMVE", "QEST-20261011-2HN63P", "QEST-20261025-M4RL61",
                "QEST-20261025-QGB4ON"]);
        expect("a README's status and title",
            Waiting.readme("---\nid: Q\nstatus: concluded\n---\n# The Title\n"),
            { status: "concluded", title: "The Title" });
        expect("one with no front matter says no status", Waiting.readme("# T\nstatus: x"),
            { status: "", title: "T" });

        // A session's want is Joel's only while it's true. The presence
        // files as they stood: three quests mirrored items Joel can't do
        // yet, and 19AMVE's was a publish he could.
        const by = Waiting.standing(today.now);
        const presence = (address, kind, more) => with_({ addresses: [address],
            owns: [`QEST-X-${address}`], wants: { kind: kind } }, more ?? {});
        expect("an action whose quest has something ready is Joel's",
            Waiting.asksJoel(presence("19AMVE", "action"), by), true);
        expect("by its quest as well as its address",
            Waiting.asksJoel(presence("ZZZZZZ", "action",
                { owns: ["QEST-20260924-19AMVE"] }), by), true);
        expect("an action with nothing ready isn't",
            ["M4RL61", "QGB4ON", "2HN63P"].map(address =>
                Waiting.asksJoel(presence(address, "action"), by)), [false, false, false]);
        expect("an orchestrated session's question is its orchestrator's",
            Waiting.asksJoel(presence("20KSI7", "question", { orchestratedBy: "WCJQSG" }), by),
            false);
        expect("the orchestrator's own question is Joel's",
            Waiting.asksJoel(presence("WCJQSG", "question"), by), true);
        expect("as is a lone session's",
            Waiting.asksJoel(presence("N10TYF", "question", { orchestratedBy: null }), by), true);
        expect("a yes is always his",
            Waiting.asksJoel(presence("M4RL61", "approval", { orchestratedBy: "WCJQSG" }), by),
            true);
        expect("and a run stuck without him",
            Waiting.asksJoel(presence("M4RL61", "blocked", { orchestratedBy: "WCJQSG" }), by),
            true);
        expect("no want is none", Waiting.asksJoel({ wants: null }, by), false);

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
