// What only Joel can do, run in Qt's own JavaScript engine, the one the
// shell runs it in. The `needs-you-is-true` check copies Waiting.js, the
// PiSource.js it compares by, launcher/NeedsYou.js and the Ago.js it says a
// span by beside this file
// and runs `qml -platform offscreen waiting-on-you.qml`.
// The files are those the agents kept on 2026-10-26, cut to what's read,
// when Waiting on You counted eleven things and three were Joel's to do,
// and one written as mesh_wait writes them since 2026-10-30 (v 2), each
// given as the mycelium node serves it.
import QtQuick
import "Waiting.js" as Waiting
import "NeedsYou.js" as NeedsYou

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
                { id: "suspend-sitting", title: "Suspend sitting on battery", ready: false,
                    raisedAt: "2026-10-26T14:20:00.000Z" },
                { id: "reboot-sitting", title: "One laptop restart with the lid open",
                    ready: false, raisedAt: "2026-10-26T14:20:30.000Z" }
            ]
        },
        "/state/mission/waiting/QGB4ON.json": {
            updated: "2026-10-26T18:25:40.137Z",
            quest: "QEST-20261025-QGB4ON",
            items: [{ id: "tuple-pointer", title: "A Tuple call to the Mac", ready: false,
                when: "about 11:00" }]
        },
        "/state/mission/waiting/WJATIN.json": {
            v: 2,
            updated: "2026-10-30T19:40:00Z",
            quest: "QEST-20261030-WJATIN",
            items: [
                { id: "deploy-v3", quest: "QEST-20261030-WJATIN",
                    title: "Send the Deploy v3 Update", why: "It goes out in his name.",
                    minutes: 3, doneWhen: { by: "person" }, ready: true,
                    owner: { session: "s1", address: "WJATIN" },
                    raisedAt: "2026-10-30T19:30:00Z", steps: ["Read it", "Send it"] },
                { id: "keys", quest: "QEST-20261030-WJATIN", title: "Prove Each YubiKey",
                    why: "They're his to touch.", minutes: 10,
                    doneWhen: { check: "keys-proved" }, readyWhen: { signal: "device:dock" },
                    ready: false, owner: { session: "s1", address: "WJATIN" },
                    raisedAt: "2026-10-30T19:31:00Z" },
                { id: "call", quest: "QEST-20261030-WJATIN", title: "Join the Call",
                    why: "Only he can talk.", minutes: 30, at: "2026-10-30T20:00:00Z",
                    doneWhen: { signal: "switch" }, ready: true,
                    owner: { session: "s1", address: "WJATIN" },
                    raisedAt: "2026-10-30T19:32:00Z" }
            ]
        }
    })

    readonly property var active: ({
        "QEST-20260924-19AMVE": { status: "active", title: "Aquifer" },
        "QEST-20261011-2HN63P": { status: "active", title: "Guests" },
        "QEST-20261025-M4RL61": { status: "active", title: "Hardware" },
        "QEST-20261025-QGB4ON": { status: "active", title: "Calls" },
        "QEST-20261030-WJATIN": { status: "active", title: "Deploys" }
    })
    // Joel has said nothing yet, and the to do daemon has found nothing.
    readonly property var unsaid: ({})
    readonly property var unfound: ({ v: 1, items: {} })

    // Files as the node serves them (mycelium's PROTOCOL.md, The Daemon
    // Interface): each item a record keyed `waiting:<quest>:<id>` by its
    // file's `quest`, saying its kind and when its file was heard, the
    // node's keys in order.
    function heldOf(files) {
        const held = {};
        for (const path in files) {
            for (const item of files[path].items) {
                const key = `waiting:${files[path].quest}:${item.id}`;
                held[key] = Object.assign({ key: key, kind: "waiting", heard: at }, item);
            }
        }
        return held;
    }
    function served(files) {
        return Waiting.served(heldOf(files));
    }

    function run() {
        const failures = [];
        const expect = (name, got, want) => {
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${name}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        };
        const ids = list => list.map(item => item.id);
        const with_ = (base, more) => Object.assign({}, base, more);

        // Now holds only what Joel can do now, anything for a time first,
        // then the quickest; the rest wait under Later, the quickest first
        // and those that say nothing of how long last, as their files list
        // them.
        const today = Waiting.sorted(served(files), active, unsaid, unfound);
        expect("now is what Joel can do now, timed first, then quickest", ids(today.now),
            ["call", "publish-pp-1802", "deploy-v3"]);
        expect("later is what he can't yet, quickest first", ids(today.later),
            ["keys", "guest-mics", "suspend-sitting", "reboot-sitting", "tuple-pointer"]);
        expect("a when of now is ready where no ready is given",
            Waiting.sorted(served({ "/w/OLD.json": { quest: "QEST-X-OLDQA",
                items: [{ id: "x", title: "x", when: "now" }] } }), {}, unsaid, unfound)
                .now.length, 1);
        const publish = today.now[1];
        expect("an item is from who keeps it", publish.from, "19AMVE");
        expect("and its quest is its file's", publish.quest, "QEST-20260924-19AMVE");
        expect("its identity is its quest and id", Waiting.identity(publish),
            "QEST-20260924-19AMVE:publish-pp-1802");
        expect("one not of a quest is from its own name",
            Waiting.served({ "waiting:slack:sign-in": { id: "sign-in", title: "Sign In" } })[0]
                .from, "SLACK");
        expect("one an item names is its quest", Waiting.served({ "waiting:QEST-A-AAAAAA:x":
            { id: "x", title: "X", quest: "QEST-B-BBBBBB" } })[0].quest, "QEST-B-BBBBBB");
        expect("a record of another kind, or with no title, says nothing",
            Waiting.served({ "quest:QEST-A-AAAAAA": { title: "A" },
                "waiting:QEST-A-AAAAAA:y": { id: "y" } }), []);
        expect("a quest's are as its owner raised them, whatever their ids",
            ids(today.later).slice(2, 4), ["suspend-sitting", "reboot-sitting"]);

        // The to do daemon's word: a check that passed or a signal that came
        // takes an item away, and one found ready is for now.
        const found = { v: 1, items: {
            "QEST-20261030-WJATIN:keys": { state: "ready", at: at, how: { signal: "device:dock" } },
            "QEST-20261030-WJATIN:call": { state: "done", at: at, how: { signal: "switch" } }
        } };
        expect("as the node serves them", Waiting.verdictsOf(
            Object.keys(found.items).reduce((held, identity) => Object.assign(held, {
                [`verdict:${identity}`]: found.items[identity] }), { "quest:QEST-A": {} })),
            { items: found.items });
        const verdicted = Waiting.sorted(served(files), active, unsaid, found);
        expect("one found done is gone, one found ready is for now", ids(verdicted.now),
            ["publish-pp-1802", "deploy-v3", "keys"]);
        expect("and says what found it", verdicted.now[2].verdict.how, { signal: "device:dock" });

        // When the node heard a record is said by nothing here, so it's
        // left out, and a kind told again saying nothing else new is kept.
        const heardAt = heard => ({ "waiting:QEST-A-AAAAAA:x": { id: "x", title: "X",
            heard: heard } });
        expect("an item holds no heard", "heard" in Waiting.served(heardAt(1))[0], false);
        expect("nor does a verdict", "heard" in Waiting.verdictsOf({ "verdict:Q:x":
            { state: "ready", heard: 1 } }).items["Q:x"], false);
        const was = {};
        const kept = Waiting.kept(was, "asked", Waiting.served(heardAt(1)));
        expect("told again with only heard moved, it's the one kept",
            Waiting.kept(was, "asked", Waiting.served(heardAt(2))) === kept, true);
        expect("told something new, it's the new one", Waiting.kept(was, "asked",
            [Object.assign({}, kept[0], { title: "Y" })])[0].title, "Y");

        // What one not ready yet waits for, as its owner said.
        const waits = id => Waiting.waitsFor(today.later.find(item => item.id === id) ?? {});
        expect("a signal and its value", waits("keys"), { signal: "device", value: "dock" });
        expect("a signal with none",
            Waiting.waitsFor({ readyWhen: { signal: "switch" } }), { signal: "switch", value: "" });
        expect("a check", Waiting.waitsFor({ readyWhen: { check: "true" } }), { check: true });
        expect("an older file's words", waits("tuple-pointer"), { words: "about 11:00" });
        expect("or nothing", waits("suspend-sitting"), null);

        // How one will be known done, as its owner said.
        expect("done by a signal and its value",
            Waiting.doneBy({ doneWhen: { signal: "device:dock" } }), { signal: "device", value: "dock" });
        expect("done by a check", Waiting.doneBy({ doneWhen: { check: "true" } }), { check: true });
        expect("done only by Joel's word", Waiting.doneBy({ doneWhen: { by: "person" } }),
            { person: true });
        expect("or said nothing of", Waiting.doneBy({}), null);

        // Whether anything but Joel will see it done.
        const item = id => today.now.concat(today.later).find(one => one.id === id);
        expect("a check watches it", Waiting.watched(item("keys"), {}), true);
        expect("a signal does", Waiting.watched(item("call"), {}), true);
        expect("only Joel's word, with its session gone, nobody does",
            Waiting.watched(item("deploy-v3"), {}), false);
        expect("its session live does", Waiting.watched(item("deploy-v3"), { WJATIN: true }),
            true);
        expect("an older file's by its file's address",
            Waiting.watched(item("publish-pp-1802"), { "19AMVE": true }), true);

        // A quest that's ended, or gone from the index, keeps nothing
        // waiting; one not known yet keeps all it had.
        expect("a concluded quest's items go",
            ids(Waiting.sorted(served(files), with_(active,
                { "QEST-20261025-QGB4ON": { status: "concluded" } }), unsaid, unfound).later),
            ["keys", "guest-mics", "suspend-sitting", "reboot-sitting"]);
        expect("a retired one's too",
            ids(Waiting.sorted(served(files), with_(active,
                { "QEST-20260924-19AMVE": { status: "retired" } }), unsaid, unfound).now),
            ["call", "deploy-v3"]);
        expect("and one gone from the index",
            ids(Waiting.sorted(served(files), with_(active,
                { "QEST-20261011-2HN63P": null }), unsaid, unfound).later),
            ["keys", "suspend-sitting", "reboot-sitting", "tuple-pointer"]);
        expect("a quest not known yet keeps its items",
            Waiting.sorted(served(files), {}, unsaid, unfound).later.length, 5);

        // The index's word on each quest named: its status and title while
        // it's open, null once it's gone, and nothing before the node has
        // answered.
        const index = { "quest:QEST-20261025-M4RL61": { key: "quest:QEST-20261025-M4RL61",
            kind: "quest", status: "active", title: "Hardware", address: "M4RL61" } };
        expect("an open quest's status and title, a gone one's null",
            Waiting.owners(index, ["QEST-20261025-M4RL61", "QEST-20261011-2HN63P"]),
            { "QEST-20261025-M4RL61": { status: "active", title: "Hardware" },
                "QEST-20261011-2HN63P": null });
        expect("nothing is known before the node answers",
            Waiting.owners(null, ["QEST-20261025-M4RL61"]), {});

        // Nothing goes by its age: a file not said again in days still
        // holds what it holds, until an event takes it.
        expect("an old file's ready item is still for now",
            ids(Waiting.sorted(served({ "/w/OLD.json": { quest: "QEST-X-OLDQA",
                updated: "2026-10-01T00:00:00Z",
                items: [{ id: "old", title: "Old", ready: true }] } }), {}, unsaid, unfound)
                .now), ["old"]);

        // Joel's word stays: one done is gone, one put off waits under
        // Later until its owner says something new of it.
        const finished = Waiting.settle(unsaid, publish, "done", at);
        expect("done is kept by its identity", finished.done,
            { "QEST-20260924-19AMVE:publish-pp-1802": at });
        expect("one done is gone",
            ids(Waiting.sorted(served(files), active, finished, unfound).now),
            ["call", "deploy-v3"]);
        expect("and stays gone when its file is said again",
            ids(Waiting.sorted(served(with_(files, { "/state/mission/waiting/19AMVE.json":
                with_(files["/state/mission/waiting/19AMVE.json"],
                    { updated: "2026-10-26T21:00:00Z" }) })), active, finished, unfound).now),
            ["call", "deploy-v3"]);
        const deploy = today.now[2];
        const putOff = Waiting.settle(finished, deploy, "notNow", at);
        expect("what's done before stays done", Object.keys(putOff.done).length, 1);
        const off = Waiting.sorted(served(files), active, putOff, unfound);
        expect("one put off leaves now", ids(off.now), ["call"]);
        expect("for Later, in its turn", ids(off.later).slice(0, 2), ["deploy-v3", "keys"]);
        expect("saying it was put off", off.later[0].putOff, true);
        const wjatin = "/state/mission/waiting/WJATIN.json";
        const resaid = with_(files, { [wjatin]: with_(files[wjatin], { items: [
            with_(files[wjatin].items[0], { steps: ["Read it", "Fix it", "Send it"] })
        ] }) });
        expect("and comes back when its owner says something new of it",
            ids(Waiting.sorted(served(resaid), active, putOff, unfound).now), ["deploy-v3"]);
        expect("nothing served is nothing waiting",
            Waiting.sorted(Waiting.served(null), active, null, null).now.length, 0);

        // The quests the items name, which the index says are open or not.
        expect("the quests named", Waiting.quests(served(with_(files, { "/w/slack.json":
            { quest: "slack", items: [{ id: "sign-in", title: "Sign In" }] } }))),
            ["QEST-20260924-19AMVE", "QEST-20261011-2HN63P", "QEST-20261025-M4RL61",
                "QEST-20261025-QGB4ON", "QEST-20261030-WJATIN"]);

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

        // The bell says something needs Joel's keys now: a session stopped
        // on a question or a yes (WCJQSG's pivot, 2026-11-12), never a run
        // that ended blocked while he was away, nor an item of its own,
        // which For You lists to do when he likes.
        expect("a question and a yes ring",
            ["question", "approval"].map(kind => Waiting.rings({ kind })), [true, true]);
        expect("a run stuck and an item don't",
            ["blocked", "action"].map(kind => Waiting.rings({ kind })), [false, false]);
        expect("nor does no want", [null, undefined, {}].map(Waiting.rings),
            [false, false, false]);

        // A want is who it is, whatever its words say as they count up.
        const since = "2026-11-06T21:00:00.000Z";
        expect("a want reworded is the same want",
            Waiting.wantKey("8IDU4E", { kind: "approval", since, said: "Holds 2 calls" }),
            Waiting.wantKey("8IDU4E", { kind: "approval", since, said: "Holds 3 calls" }));
        expect("one raised again later is another",
            Waiting.wantKey("8IDU4E", { kind: "approval", since }) === Waiting.wantKey(
                "8IDU4E", { kind: "approval", since: "2026-11-06T21:05:00.000Z" }), false);
        expect("as is another kind begun at once",
            Waiting.wantKey("8IDU4E", { kind: "approval", since }) === Waiting.wantKey(
                "8IDU4E", { kind: "question", since }), false);
        expect("the harness's own id comes first",
            Waiting.wantKey("8IDU4E", { id: "approval-1", since }), "8IDU4E approval-1");
        expect("its words only when it gives no start",
            Waiting.wantKey("8IDU4E", { kind: "question", said: "Push it?" }),
            "8IDU4E question Push it?");

        // What an item tells beyond its title (NeedsYou.js): its note leads
        // with its owner's when, unless its side says it, and ends with how
        // long it has waited (VKCF3Z, 2026-11-07); its times and machine
        // are its drawer's facts alone (Y2YBTS, 2026-11-08).
        const said = { nobody: "nobody's watching this", waitedFor: "waiting {span}" };
        const raised = Date.parse("2026-11-07T06:46:47.006Z");
        const cable = { v: 2, id: "dock-cable", title: "Swap the network cable",
            why: "Only Joel can swap a cable.", minutes: 2, when: "any time, outside a call",
            raisedAt: "2026-11-07T06:46:47.006Z" };
        const told = { watched: true, side: "2 min", ready: false,
            now: raised + 15 * 3600 * 1000 + 5000 };
        expect("a note leads with when and ends with how long it's waited",
            NeedsYou.note(cable, said, told), "any time, outside a call \u00b7 waiting 15 h");
        expect("when the side says when, the note doesn't",
            NeedsYou.note(cable, said, with_(told, { side: "any time, outside a call",
                watched: false })),
            "nobody's watching this \u00b7 waiting 15 h");
        expect("its times and machine are left to its drawer",
            NeedsYou.note(with_(cable, { at: "2026-11-07T20:00:00Z",
                deadline: "2026-11-07T22:00:00Z", where: { machine: "harbour" } }), said, told),
            "any time, outside a call \u00b7 waiting 15 h");
        expect("\"now\" is no reason for one ready now",
            NeedsYou.note(with_(cable, { when: "now" }), said, with_(told, { ready: true })),
            "waiting 15 h");
        expect("but is for one not ready yet",
            NeedsYou.note(with_(cable, { when: "now" }), said, told), "now \u00b7 waiting 15 h");
        expect("a wait shorter than a minute is a minute",
            NeedsYou.waited(cable.raisedAt, raised + 20_000), "1 min");
        expect("one of days says days", NeedsYou.waited(cable.raisedAt, raised + 3 * 86_400_000),
            "3 d");
        expect("one raised at no time says none", NeedsYou.waited(undefined, raised), "");
        expect("an older file's item that says nothing else says why",
            NeedsYou.note({ why: "It needs you." }, said, with_(told, { now: raised })),
            "It needs you.");
        expect("and how long it has waited after",
            NeedsYou.note({ why: "It needs you.", raisedAt: cable.raisedAt }, said, told),
            "It needs you. \u00b7 waiting 15 h");

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
