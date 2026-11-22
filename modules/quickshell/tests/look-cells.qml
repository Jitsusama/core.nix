// Where a look leads, run in Qt's own JavaScript engine, the one the shell
// runs it in. The `a-look-leads-to-its-cell` check copies
// modules/quickshell/launcher/Looks.js, and the ForYou.js and Order.js it
// imports, beside this file and runs
// `qml -platform offscreen look-cells.qml`. Each case is a rule of the
// cell a look's Talk and Go To reach (Looks.js's cellOf and cellFor): the
// session that left it, the agents daemon's answer for its cell, and the
// sessions on its quest, and another quest's only as the one that left a
// look naming no cell; when a Go says its session moved on (movedOn); a
// file's lent mark never read; and when a served look is settled once it
// opened. How For You weighs a look is for-you.qml's.
import QtQuick
import "Looks.js" as Looks

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
        const expect = (what, got, wanted) => {
            if (JSON.stringify(got) !== JSON.stringify(wanted))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(wanted)}`);
        };
        const now = Date.parse("2026-11-08T14:00:00Z");

        // The cell a look leads to, which Talk to It and Open It reach (Joel,
        // 2026-11-16: "i have no way to go from the thing it shows to engaging
        // with the thing working on it"): the session that left it, running
        // or parked, else the quest's own session, else one on its quest,
        // running first, else none.
        const leaver = { address: "AAAAAA", presence: { session: "s-1", quest: "Q-1" } };
        const heir = { address: "BBBBBB", presence: { session: "s-2", quest: "Q-1" } };
        const resting = { address: "CCCCCC", presence: { session: "s-3", quest: "Q-1" } };
        const stranger = { address: "DDDDDD", presence: { session: "s-4", quest: "Q-2" } };
        const lookLeft = { session: "s-1", address: "AAAAAA", quest: "Q-1" };
        expect("a look leads to the session that left it",
            Looks.cellOf(lookLeft, [stranger, heir, leaver], []).address, "AAAAAA");
        expect("parked, that session still",
            Looks.cellOf(lookLeft, [heir], [leaver]).address, "AAAAAA");
        expect("gone, a running session on its quest",
            Looks.cellOf(lookLeft, [stranger, heir], [resting]).address, "BBBBBB");
        expect("none running there, a parked one on its quest",
            Looks.cellOf(lookLeft, [stranger], [resting]).address, "CCCCCC");
        expect("none on its quest, none", Looks.cellOf(lookLeft, [stranger], []), null);
        expect("a look naming no quest leads only to its own session",
            Looks.cellOf({ session: "s-9" }, [heir], [resting]), null);
        expect("and to that session while it's held",
            Looks.cellOf({ session: "s-2" }, [stranger, heir], []).address, "BBBBBB");
        expect("its session found by an address it had before",
            Looks.cellOf({ address: "OLDADR" }, [stranger, { address: "EEEEEE",
                presence: { session: "s-5", addresses: ["OLDADR"] } }], []).address,
            "EEEEEE");
        // The quest's own session, where a look's answers go once the one
        // that left it has ended, before another listed ahead of it.
        const own = { address: "ABCDEF", presence: { session: "s-6", quest: "QEST-1-ABCDEF" } };
        const helper = { address: "FFFFFF", presence: { session: "s-7", quest: "QEST-1-ABCDEF" } };
        expect("gone, the quest's own session before another on it",
            Looks.cellOf({ session: "s-0", quest: "QEST-1-ABCDEF" }, [helper], [own]).address,
            "ABCDEF");
        expect("nothing held, none", Looks.cellOf(lookLeft, undefined, undefined), null);
        // As the agents daemon names it (cells.rs): the cell a look names goes
        // to the live session it is now, running or parked; ended with none
        // to go to, or to one not listed yet, itself, else one on its quest
        // only; not served yet, or the daemon away, the same; naming no cell,
        // cellOf's search over every session.
        const cells = {
            "agent-cell:agents:AAAAAA": { cell: "AAAAAA", goes: "BBBBBB" },
            "agent-cell:agents:CCCCCC": { cell: "CCCCCC", goes: "CCCCCC" },
            "agent-cell:agents:GGGGGG": { cell: "GGGGGG", goes: null }
        };
        const naming = cell => Object.assign({}, lookLeft, { cell: cell });
        expect("a cell goes where the daemon says, running",
            Looks.cellFor(naming("AAAAAA"), cells, [stranger, heir], []).address, "BBBBBB");
        expect("the daemon's answer before another on its quest, running first",
            Looks.cellFor(naming("AAAAAA"), { "agent-cell:agents:AAAAAA": { goes: "CCCCCC" } },
                [heir], [resting]).address, "CCCCCC");
        expect("a look on no quest said to go nowhere, never one listing its address",
            Looks.cellFor({ cell: "GGGGGG", address: "GGGGGG" }, cells, [{ address: "LLLLLL",
                presence: { quest: "Q-2", addresses: ["LLLLLL", "GGGGGG"] } }], []), null);
        expect("a cell back again before an older answer",
            Looks.cellFor(naming("AAAAAA"), cells, [stranger, heir, leaver], []).address,
            "AAAAAA");
        expect("or parked",
            Looks.cellFor({ cell: "CCCCCC" }, cells, [stranger], [resting]).address, "CCCCCC");
        expect("ended with none to go to, none where the lists hold none",
            Looks.cellFor(naming("GGGGGG"), cells, [stranger], []), null);
        expect("said to go nowhere, one on its quest the lists hold, as the record may be stale",
            Looks.cellFor(naming("GGGGGG"), cells, [stranger, heir], []).address, "BBBBBB");
        expect("a name contested, the one on the look's quest",
            Looks.cellFor(naming("OLDOLD"), { "agent-cell:agents:OLDOLD": { goes: null } },
                [{ address: "TTTTTT", presence: { quest: "Q-2", addresses: ["AAAAAA"] } },
                    heir], []).address, "BBBBBB");
        const lister = { address: "TTTTTT",
            presence: { session: "s-9", quest: "Q-2", addresses: ["TTTTTT", "AAAAAA"] } };
        expect("ended with none to go to, never one on another quest listing its address",
            Looks.cellFor(naming("GGGGGG"), cells, [lister], []), null);
        expect("ended with none to go to, itself where the lists hold it back",
            Looks.cellFor(naming("GGGGGG"), cells, [lister],
                [{ address: "GGGGGG", presence: { session: "s-10" } }]).address, "GGGGGG");
        expect("the address a cell goes to, keyed", Looks.cellGoes(naming("AAAAAA"), cells),
            "BBBBBB");
        expect("null for one going nowhere", Looks.cellGoes(naming("GGGGGG"), cells), null);
        expect("none for one not served", Looks.cellGoes(naming("HHHHHH"), cells), "");
        expect("none for one naming no cell", Looks.cellGoes(naming(7), cells), "");
        expect("going to a session not listed yet, the cell itself",
            Looks.cellFor(naming("AAAAAA"), cells, [leaver], []).address, "AAAAAA");
        expect("going to a session not listed yet, the quest's own before another on it",
            Looks.cellFor({ cell: "OLDOLD", quest: "QEST-1-ABCDEF" },
                { "agent-cell:agents:OLDOLD": { goes: "ZZZZZZ" } }, [helper], [own]).address,
            "ABCDEF");
        expect("said to go nowhere, one on its quest by the quest pi publishes",
            Looks.cellFor(naming("GGGGGG"), cells, [{ address: "QQQQQQ",
                presence: { questInfo: { id: "Q-1" } } }], []).address, "QQQQQQ");
        expect("its questInfo's quest before an older quest it names",
            Looks.cellFor(naming("GGGGGG"), cells, [{ address: "PPPPPP",
                presence: { questInfo: { id: "Q-2" }, quest: "Q-1" } }], []), null);
        expect("not served yet, a look on no quest, the session that left it by its id",
            Looks.cellFor({ session: "s-4", cell: "HHHHHH" }, cells, [stranger], []).address,
            "DDDDDD");
        expect("the cell itself on another quest, not followed",
            Looks.cellFor({ cell: "AAAAAA", quest: "Q-9" }, cells, [stranger, leaver], []),
            null);
        expect("one going by the cell's address on another quest, the look's quest's own instead",
            Looks.cellFor({ cell: "AAAAAA", quest: "QEST-1-ABCDEF" }, {}, [leaver, own], [])
                .address, "ABCDEF");
        expect("the cell itself on its quest, before one on another going by its address",
            Looks.cellFor(naming("AAAAAA"), {}, [{ address: "AAAAAA",
                presence: { session: "s-9", quest: "Q-2" } }, leaver], []).presence.session,
            "s-1");
        expect("on its quest, one answering to the cell's old address, as cells.rs claims it",
            Looks.cellFor(naming("OLDOLD"), {}, [heir, { address: "TTTTTT",
                presence: { session: "s-9", quest: "Q-1", addresses: ["TTTTTT", "AAAAAA"] } }],
                []).address, "TTTTTT");
        const listing = { address: "TTTTTT",
            presence: { session: "s-9", quest: "Q-2", addresses: ["TTTTTT", "AAAAAA"] } };
        expect("going to a session not listed yet, never one on another quest listing it",
            Looks.cellFor(naming("AAAAAA"), cells, [listing], []), null);
        expect("going to a session not listed yet, one on its quest",
            Looks.cellFor(naming("AAAAAA"), cells, [listing], [resting]).address, "CCCCCC");
        expect("not served yet, cellOf's search on its quest",
            Looks.cellFor(naming("HHHHHH"), cells, [heir], [leaver]).address,
            "AAAAAA");
        expect("the daemon away, never one on another quest listing its address",
            Looks.cellFor(naming("GGGGGG"), {}, [listing], []), null);
        expect("naming no cell, cellOf's search over every quest",
            Looks.cellFor(lookLeft, {}, [listing], []).address, "TTTTTT");
        expect("done with, naming no cell, never one on another quest listing its address",
            Looks.cellFor(Looks.doneWith(lookLeft), {}, [listing], []), null);
        expect("done with, naming no cell, the session that left it by its address",
            Looks.cellFor(Looks.doneWith(lookLeft), {}, [listing, leaver], []).address,
            "AAAAAA");
        expect("done with, naming its cell, that cell kept",
            Looks.doneWith({ cell: "BBBBBB", address: "AAAAAA" }).cell, "BBBBBB");
        expect("naming no cell, cellOf's search",
            Looks.cellFor(naming(7), cells, [heir], [leaver]).address, "AAAAAA");
        expect("nothing served, cellOf's search",
            Looks.cellFor(naming("AAAAAA"), undefined, [heir], [leaver]).address, "AAAAAA");
        // The daemon reads the cell's quest, and names a lone lister on any
        // once the node forgets it (cells.rs), so its answer is followed only
        // onto the look's quest, or for a look on none to the one that left it.
        expect("the daemon naming a session on another quest, not followed",
            Looks.cellFor(naming("AAAAAA"), { "agent-cell:agents:AAAAAA": { goes: "DDDDDD" } },
                [stranger], []), null);
        expect("a look on no quest, the daemon naming a lister, not followed",
            Looks.cellFor({ cell: "GGGGGG", session: "s-0" },
                { "agent-cell:agents:GGGGGG": { goes: "LLLLLL" } }, [{ address: "LLLLLL",
                    presence: { session: "s-8", quest: "Q-2" } }], []), null);
        expect("a look on no quest, the one that left it by its id, not the one the daemon names",
            Looks.cellFor({ cell: "GGGGGG", session: "s-8" },
                { "agent-cell:agents:GGGGGG": { goes: "MMMMMM" } }, [{ address: "MMMMMM",
                    presence: { session: "s-5", quest: "Q-2" } }, { address: "LLLLLL",
                    presence: { session: "s-8", quest: "Q-2" } }], []).address, "LLLLLL");
        expect("a look on no quest naming no session, the daemon naming a move, not followed",
            Looks.cellFor({ cell: "GGGGGG" }, { "agent-cell:agents:GGGGGG": { goes: "LLLLLL" } },
                [{ address: "LLLLLL", presence: { session: "s-8", quest: "Q-2" } }], []),
            null);
        const moved = { address: "NNNNNN", presence: { session: "s-1", quest: "Q-1" } };
        expect("said to go nowhere, the one that left it under a new address, on its quest",
            Looks.cellFor(naming("GGGGGG"), cells, [heir, moved], []).address, "NNNNNN");
        const movedOnNone = { address: "NNNNNN", presence: { session: "s-1" } };
        expect("the one that left it under a new address, on no quest yet, by its id",
            Looks.cellFor(naming("GGGGGG"), cells, [movedOnNone, heir], []).address, "NNNNNN");
        expect("but never another on no quest yet that only lists its address",
            Looks.cellFor(naming("GGGGGG"), cells, [{ address: "PPPPPP",
                presence: { session: "s-7", addresses: ["AAAAAA"] } }, heir], []).address,
            "BBBBBB");
        const movedAway = { address: "NNNNNN", presence: { session: "s-1", quest: "Q-9" } };
        expect("the one that left it, on another quest now, not followed",
            Looks.cellFor(naming("GGGGGG"), cells, [movedAway], []), null);
        expect("the one that left it on another quest now, the look's quest's session instead",
            Looks.cellFor(naming("GGGGGG"), cells, [movedAway, heir], []).address, "BBBBBB");
        // Go on such a look says its session has moved on rather than ended,
        // where the one that left it is held by its id or the very address.
        expect("the one that left it, held on another quest, moved on; gone or listing, not",
            [Looks.movedOn(naming("GGGGGG"), [movedAway], []),
                Looks.movedOn(naming("GGGGGG"), [], [{ address: "AAAAAA",
                    presence: { quest: "Q-9" } }]),
                Looks.movedOn(naming("GGGGGG"), [stranger], []),
                Looks.movedOn(naming("GGGGGG"), [{ address: "PPPPPP",
                    presence: { session: "s-7", addresses: ["AAAAAA"] } }], [])],
            [true, true, false, false]);
        expect("held on no quest, or on the look's own, not moved on; for a look on none, on any",
            [Looks.movedOn(naming("GGGGGG"), [{ address: "AAAAAA", presence: { session: "s-9" } }],
                []),
                Looks.movedOn(naming("GGGGGG"), [{ address: "AAAAAA",
                    presence: { session: "s-9", quest: "Q-1" } }], []),
                Looks.movedOn(Object.assign({}, naming("GGGGGG"), { quest: "" }),
                    [{ address: "AAAAAA", presence: { session: "s-9" } }], []),
                Looks.movedOn(Object.assign({}, naming("GGGGGG"), { quest: "" }),
                    [{ address: "AAAAAA", presence: { session: "s-9", quest: "Q-9" } }], [])],
            [false, false, false, true]);
        expect("the daemon away, the look's quest's session still, never the one moved away",
            Looks.cellFor(naming("GGGGGG"), {}, [movedAway, heir], []).address, "BBBBBB");
        expect("the daemon's answer on its quest before the one that left it there",
            Looks.cellFor(naming("AAAAAA"), { "agent-cell:agents:AAAAAA": { goes: "BBBBBB" } },
                [moved, heir], []).address, "BBBBBB");
        // Its quest as the daemon reads it to place a cell (cells.rs
        // quest_of): the first it owns, else its questInfo's, else its quest.
        expect("a session owning the look's quest only, found on it",
            Looks.cellFor({ cell: "GGGGGG", quest: "QEST-1-ABCDEF" }, cells, [{ address: "OOOOOO",
                presence: { owns: ["PLAN-1-AAAAAA", "QEST-1-ABCDEF"] } }], []).address, "OOOOOO");
        expect("owning another, still on the quest it has loaded, where a look names it",
            Looks.cellFor({ cell: "GGGGGG", quest: "QEST-1-ABCDEF" }, cells, [{ address: "OOOOOO",
                presence: { owns: ["QEST-2-BBBBBB"], questInfo: { id: "QEST-1-ABCDEF" } } }],
                []).address, "OOOOOO");
        expect("the quest it owns before its questInfo's",
            Looks.questOf({ presence: { owns: ["QEST-2-BBBBBB"],
                questInfo: { id: "QEST-1-ABCDEF" } } }), "QEST-2-BBBBBB");
        expect("the first quest it owns",
            Looks.questOf({ presence: { owns: ["QEST-2-BBBBBB", "QEST-1-ABCDEF"] } }),
            "QEST-2-BBBBBB");
        expect("owning two, on the second too",
            Looks.cellFor({ cell: "GGGGGG", quest: "QEST-1-ABCDEF" }, cells, [{ address: "OOOOOO",
                presence: { owns: ["QEST-2-BBBBBB", "QEST-1-ABCDEF"] } }], []).address,
            "OOOOOO");
        expect("a questInfo id not words passed over, as cells.rs does",
            Looks.questOf({ presence: { questInfo: { id: 7 }, quest: "QEST-X" } }), "QEST-X");
        expect("a quest not words, none", Looks.questOf({ presence: { quest: 7 } }), "");
        expect("owning no quest, its questInfo's",
            Looks.questOf({ presence: { owns: ["PLAN-1-AAAAAA"], questInfo: { id: "Q-1" } } }),
            "Q-1");
        expect("owns not a list, its questInfo's",
            Looks.questOf({ presence: { owns: "QEST-2-BBBBBB", questInfo: { id: "Q-1" } } }),
            "Q-1");
        expect("past what isn't words in owns, the quest it owns",
            Looks.questOf({ presence: { owns: [7, null, "QEST-2-BBBBBB"] } }), "QEST-2-BBBBBB");
        expect("a quest's id spelled otherwise owns nothing",
            Looks.questOf({ presence: { owns: ["qest-2-bbbbbb", "QESTX-2"], quest: "Q-1" } }),
            "Q-1");
        expect("the daemon's answer followed onto a session owning the look's quest",
            Looks.cellFor({ cell: "OLDOLD", quest: "QEST-1-ABCDEF" },
                { "agent-cell:agents:OLDOLD": { goes: "OOOOOO" } }, [own, { address: "OOOOOO",
                    presence: { owns: ["QEST-1-ABCDEF"], quest: "Q-2" } }], []).address,
            "OOOOOO");
        // A served look the shell holds no file of is settled by the daemon
        // once opened, never where its session was gone and nothing opened.
        expect("a served quiet look opened, settled",
            Looks.servedSettles({ file: "", state: "quiet" }, true), true);
        expect("a served waiting look opened, settled",
            Looks.servedSettles({ file: "", state: "waiting" }, true), true);
        expect("a served waiting look that opened nothing, left as it was",
            Looks.servedSettles({ file: "", state: "waiting" }, false), false);
        expect("one the shell holds a file of, settled by the shell",
            Looks.servedSettles({ file: "q.json", state: "quiet" }, true), false);
        expect("one opened before, left as it was",
            Looks.servedSettles({ file: "", state: "opened" }, true), false);
        // A lent cell is the shell's own mark, never read from a file.
        const lentRead = Looks.look(JSON.stringify({ v: 1, id: "l-1", title: "A look",
            target: "/x", kind: "file", state: "cleared", cell: "BBBBBB", cellLent: true }));
        expect("a file's lent mark is not read", "cellLent" in lentRead, false);
        // Written by hand, since a literal's __proto__ is no key of its own.
        const protoText = '{"v":1,"id":"l-2","title":"A look","target":"/x",'
            + '"kind":"file","state":"cleared","cell":"BBBBBB","__proto__":{"cellLent":true}}';
        expect("the engine parses a __proto__ key as the file's own",
            Object.keys(JSON.parse(protoText)).includes("__proto__"), true);
        expect("nor one lent through its prototype",
            Looks.look(protoText).cellLent === true, false);
        expect("so a cell it names stays written as its own", JSON.parse(Looks.rewritten(
            Looks.doneWith(lentRead), "cleared", "cleared", now)).cell, "BBBBBB");

        // What a Go whose session isn't held says: the moved words where it
        // has moved on, else its own, and its own where the moved words are
        // missing, never a bare foot.
        const said = { gone: "its session isn't here", why: "it has ended",
            moved: { gone: "its session has moved on", why: "Agents lists it" } };
        expect("moved on, the moved words; gone, its own",
            [Looks.goneWords(said, true), Looks.goneWords(said, false)],
            [{ gone: "its session has moved on", why: "Agents lists it" },
                { gone: "its session isn't here", why: "it has ended" }]);
        expect("moved on with no moved words, or empty ones, its own still",
            [Looks.goneWords({ gone: "g", why: "w" }, true),
                Looks.goneWords({ gone: "g", why: "w", moved: { gone: "" } }, true)],
            [{ gone: "g", why: "w" }, { gone: "g", why: "w" }]);
        expect("no words at all, empty ones rather than a throw",
            Looks.goneWords(undefined, true), { gone: "", why: "" });

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            // Qt.exit only asks the engine to stop once this returns.
            Qt.exit(1);
            return;
        }
        console.info("A look leads to its cell");
        Qt.exit(0);
    }
}
