// Where Reopen Them All puts each session back, run in Qt's own JavaScript
// engine, the one the shell runs it in, and how fast it may go. The
// `reopening-finds-each-screen` check copies modules/quickshell/Places.js,
// Reopening.js and Viewers.js beside this file and runs
// `qml -platform offscreen places.qml`. Each case is one a boot or a
// dock can make: the same monitor on another connector, a screen gone, and
// a place kept before screens were named by what they are.
import QtQuick
import "Places.js" as Places
import "Reopening.js" as Reopening
import "Viewers.js" as Viewers

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
        const monitor = "HP Inc. HP Z27k G3 CN41282GC1";
        const panel = "LG Display 0x07C6";

        expect("a screen as niri's config names it", Places.screensOf({
            "DP-4": { make: "HP Inc.", model: "HP Z27k G3", serial: "CN41282GC1" },
            "eDP-1": { make: "LG Display", model: "0x07C6", serial: null }
        }), { "DP-4": monitor, "eDP-1": panel });
        expect("a screen that's off as gone, as the lid shut on the dock leaves it",
            Places.screensOf({
                "DP-4": { make: "HP Inc.", model: "HP Z27k G3", serial: "CN41282GC1",
                    logical: { x: 0, y: 0, width: 2560, height: 1440 } },
                "eDP-1": { make: "LG Display", model: "0x07C6", serial: null, logical: null }
            }), { "DP-4": monitor });

        // Kept on the monitor as DP-4; after the boot it's DP-5, beside the
        // laptop's panel, which is in use.
        const kept = [
            { screen: monitor, output: "DP-4", workspace: 2, column: 1, tile: 1 },
            { screen: panel, output: "eDP-1", workspace: 1, column: 3, tile: 2 },
            null
        ];
        expect("each on its own screen, under its new connector",
            Places.placed(kept, { "DP-5": monitor, "eDP-1": panel }, "eDP-1"), [
                { screen: monitor, output: "DP-5", workspace: 2, column: 1, tile: 1 },
                { screen: panel, output: "eDP-1", workspace: 1, column: 3, tile: 2 },
                null
            ]);

        // Docked with the lid shut: the panel's sessions go on the
        // monitor, after its own workspaces, in their order.
        const docked = [
            { screen: monitor, output: "DP-4", workspace: 1, column: 1, tile: 1 },
            { screen: monitor, output: "DP-4", workspace: 3, column: 1, tile: 1 },
            { screen: panel, output: "eDP-1", workspace: 2, column: 1, tile: 1 },
            { screen: panel, output: "eDP-1", workspace: 1, column: 2, tile: 1 }
        ];
        expect("a gone screen's workspaces after the one in use's",
            Places.placed(docked, { "DP-4": monitor }, "DP-4").map(at => [at.output, at.workspace]),
            [["DP-4", 1], ["DP-4", 3], ["DP-4", 5], ["DP-4", 4]]);

        // The monitor is another one now, on the same connector: its
        // description says it isn't the screen the places were kept on.
        expect("a connector now another screen's isn't taken for it",
            Places.placed([docked[0]], { "DP-4": "Dell Inc. DELL U2723QE 7XY" }, "DP-4")
                .map(at => [at.output, at.workspace]), [["DP-4", 1]]);
        expect("two gone screens, one's workspaces after the other's",
            Places.placed([docked[0], docked[2]], { "DP-4": "Dell Inc. DELL U2723QE 7XY" }, "DP-4")
                .map(at => at.workspace), [1, 3]);

        // Two monitors alike, with no serial to tell them apart: each
        // place goes to the one it was kept on.
        const twin = "QEMU QEMU Monitor";
        expect("two screens alike, each place on its own",
            Places.placed([{ screen: twin, output: "Virtual-2", workspace: 1, column: 1, tile: 1 }],
                { "Virtual-1": twin, "Virtual-2": twin }, "Virtual-1").map(at => at.output),
            ["Virtual-2"]);

        // Kept before screens were named so: found by its connector.
        const older = [{ output: "eDP-1", workspace: 2, column: 1, tile: 1 }];
        expect("a place without a screen, by its connector",
            Places.placed(older, { "eDP-1": panel, "DP-4": monitor }, "DP-4")
                .map(at => [at.output, at.workspace]), [["eDP-1", 2]]);

        // No screen known, as when niri didn't answer: left as it was kept,
        // rather than every place taken for a gone screen's.
        expect("no screen known leaves it as kept",
            Places.placed([docked[2]], {}, "DP-4").map(at => [at.output, at.workspace]),
            [["eDP-1", 2]]);
        expect("no screen in use leaves a gone one as kept",
            Places.placed([docked[2]], { "DP-4": monitor }, null)
                .map(at => [at.output, at.workspace]), [["eDP-1", 2]]);

        // A gone screen's place says where it belongs, so its workspace
        // can go there when the screen comes; one on its screen doesn't.
        expect("a gone screen's place says where it belongs",
            Places.placed(docked, { "DP-4": monitor }, "DP-4").map(at => at.away ?? null), [
                null, null,
                { screen: panel, workspace: 2, output: "eDP-1" },
                { screen: panel, workspace: 1, output: "eDP-1" }
            ]);

        // The monitor comes after the reopen began: the workspaces given
        // the panel in its place go to it, lowest kept number first, their
        // order there the windows daemon's; one gone stays gone, and one
        // there already is marked and left.
        const away = { [monitor]: [
            { id: 11, workspace: 1 }, { id: 13, workspace: 3 }, { id: 12, workspace: 2 },
            { id: 14, workspace: 4 }, { id: 15, workspace: 5 }
        ] };
        const workspaces = [
            { id: 11, output: "eDP-1" }, { id: 12, output: "eDP-1" }, { id: 13, output: "eDP-1" },
            { id: 15, output: "DP-4" }, { id: 16, output: "DP-4" }
        ];
        expect("a screen come back takes its workspaces, lowest kept first",
            Places.homeward(away, { "eDP-1": panel, "DP-4": monitor }, workspaces), {
                moves: [{ output: "DP-4", id: 11 },
                    { output: "DP-4", id: 12 },
                    { output: "DP-4", id: 13 },
                    { output: "DP-4", id: 15, there: true }],
                waiting: {}
            });
        expect("a screen still gone keeps its workspaces waiting, but not one gone",
            Places.homeward(away, { "eDP-1": panel }, workspaces), { moves: [], waiting: {
                [monitor]: [{ id: 11, workspace: 1, stayed: [] }, { id: 12, workspace: 2, stayed: [] },
                    { id: 13, workspace: 3, stayed: [] }, { id: 15, workspace: 5, stayed: [] }]
            } });
        // The monitor is on another connector now, and none alike was
        // connected when its workspace was noted, so this is it.
        expect("a screen come back on another connector takes its workspaces",
            Places.homeward({ [monitor]: [{ id: 11, workspace: 1, output: "DP-4", stayed: [] }] },
                { "eDP-1": panel, "DP-5": monitor }, [{ id: 11, output: "eDP-1" }]),
            { moves: [{ output: "DP-5", id: 11 }], waiting: {} });

        // Two screens alike, as two monitors of one model without serials
        // are, and as a VM's are. Each workspace decides by the connector it
        // left: it goes back there, however niri lists them, rather than to
        // an alike screen as if home (vm-screens-keep-places flaked so,
        // 2026-11-16 and 18 Nov).
        const qemu = "Red Hat, Inc. QEMU Monitor Unknown";
        const left = { [qemu]: [{ id: 21, workspace: 2, output: "Virtual-2", stayed: ["Virtual-1"] }] };
        const wanted = { moves: [{ output: "Virtual-2", id: 21 }], waiting: {} };
        const onThePanel = [{ id: 21, output: "eDP-1" }];
        expect("a workspace goes back to the alike screen it left, listed first",
            Places.homeward(left, { "eDP-1": panel, "Virtual-2": qemu, "Virtual-1": qemu }, onThePanel),
            wanted);
        expect("a workspace goes back to the alike screen it left, listed last",
            Places.homeward(left, { "eDP-1": panel, "Virtual-1": qemu, "Virtual-2": qemu }, onThePanel),
            wanted);
        expect("a workspace niri handed back to the screen it left is there",
            Places.homeward(left, { "Virtual-1": qemu, "Virtual-2": qemu }, [{ id: 21, output: "Virtual-2" }]),
            { moves: [{ output: "Virtual-2", id: 21, there: true }], waiting: {} });
        // While the screen it left is gone, an alike screen that was
        // connected when it was noted isn't that screen come back, so it
        // waits wherever it is: on the alike screen it was sent to, on
        // another (as Joel might move it), or beside two more alike. Taken
        // for home, its note was dropped and the replug left it where it was.
        const waits = { moves: [], waiting: left };
        expect("a workspace on the alike screen it was sent to waits for the one it left",
            Places.homeward(left, { "Virtual-1": qemu }, [{ id: 21, output: "Virtual-1" }]), waits);
        expect("a workspace off the alike screen that stayed waits for the one it left",
            Places.homeward(left, { "eDP-1": panel, "Virtual-1": qemu }, onThePanel), waits);
        const three = { [qemu]: [{ id: 21, workspace: 2, output: "Virtual-2",
            stayed: ["Virtual-1", "Virtual-3"] }] };
        expect("a third alike screen isn't the one it left",
            Places.homeward(three, { "Virtual-1": qemu, "Virtual-3": qemu },
                [{ id: 21, output: "Virtual-1" }]), { moves: [], waiting: three });
        // Both alike screens gone at once, as at an undock: each workspace
        // goes to the one it left, one there already left there, and one
        // whose screen is still gone doesn't take its sibling's for it.
        const both = { [qemu]: [{ id: 1, workspace: 1, output: "Virtual-1", stayed: [] },
            { id: 2, workspace: 1, output: "Virtual-2", stayed: [] }] };
        expect("both alike screens back, each takes its own",
            Places.homeward(both, { "Virtual-1": qemu, "Virtual-2": qemu },
                [{ id: 1, output: "Virtual-1" }, { id: 2, output: "Virtual-1" }]), {
                moves: [{ output: "Virtual-1", id: 1, there: true }, { output: "Virtual-2", id: 2 }],
                waiting: {}
            });
        expect("one alike screen back, the other's workspace waits beside it",
            Places.homeward(both, { "Virtual-2": qemu },
                [{ id: 1, output: "Virtual-2" }, { id: 2, output: "Virtual-2" }]), {
                moves: [{ output: "Virtual-2", id: 2, there: true }],
                waiting: { [qemu]: [{ id: 1, workspace: 1, output: "Virtual-1", stayed: ["Virtual-2"] }] }
            });
        // What stayed beside a screen as it went: the connected connectors
        // alike it, but the one it left, each once.
        expect("the alike screens that stayed",
            Places.alikeStayed(qemu, "Virtual-2", { "eDP-1": panel, "Virtual-1": qemu, "Virtual-2": qemu },
                ["eDP-1", "Virtual-1", "Virtual-1", "Virtual-2"]), ["Virtual-1"]);
        expect("none stay beside a screen known by its connector",
            Places.alikeStayed("eDP-1", "eDP-1", { "eDP-1": panel }, ["eDP-1"]), []);

        // Windows that reopen themselves, matched by program and title,
        // else the first of that program, each kept window used once.
        const others = [
            { app: "google-chrome", title: "Docs" },
            { app: "google-chrome", title: "Open · Backwater" },
            { app: "slack", title: "Slack" }
        ];
        expect("a window is the kept one of its program with its title",
            Places.matchOther(others, [], "google-chrome", "Open · Backwater"), 1);
        expect("else the first of its program not used yet",
            Places.matchOther(others, [0], "google-chrome", "New Tab"), 1);
        expect("a program kept nowhere goes nowhere",
            Places.matchOther(others, [], "firefox", "Docs"), -1);
        expect("each kept window is used once",
            Places.matchOther(others, [2], "slack", "Slack"), -1);

        // Put back beside what's back: the session kept with Chrome on the
        // monitor's fifth came back as its fourth, since a workspace below
        // that held only other windows isn't made again.
        // A window Joel opened since sits first there, so it goes by its
        // neighbours, not by a column number.
        const session = (id, workspace, column, at) => ({
            id,
            at: { screen: monitor, output: "DP-4", workspace, column, tile: 1 },
            workspace: { id: 20 + at, idx: at, output: "DP-4" },
            column: column + 1
        });
        const beside = [session(1, 4, 1, 3), session(2, 5, 1, 4), session(3, 5, 3, 4),
            session(4, 5, 2, 4), session(5, 7, 1, 5)];
        const kept5 = column => ({ screen: monitor, output: "DP-4", workspace: 5, column, tile: 1 });
        expect("a window joins the workspace its session came back to, after the nearest left",
            Places.whereBack(kept5(3), beside), { join: 24, after: 4 });
        expect("before the nearest right when it sat first",
            Places.whereBack(kept5(1), beside.slice(2)), { join: 24, before: 4 });
        expect("a workspace that held no session is made after the one below it",
            Places.whereBack({ screen: monitor, output: "DP-4", workspace: 6, column: 1, tile: 1 },
                beside), { make: true, output: "DP-4", index: 5 });
        expect("first on its screen when nothing sat below it",
            Places.whereBack({ screen: panel, output: "eDP-1", workspace: 2, column: 1, tile: 1 },
                beside), { make: true, output: null, index: 1 });
        // One kept below it that opened while every screen was gone sits on
        // a screen standing in for its own, whose numbers say nothing of
        // its own screen: after it there, the second of the monitor's
        // stayed on the laptop's once the monitor came back
        // (screens-keep-places, 2026-11-14).
        const standing = [{ id: 7, at: { screen: monitor, output: "DP-4", workspace: 1, column: 1,
            tile: 1 }, workspace: { id: 31, idx: 1, output: "eDP-1" }, column: 2 }];
        expect("one below it on a screen standing in for its own isn't followed there",
            Places.whereBack({ screen: monitor, output: "DP-4", workspace: 2, column: 1, tile: 1 },
                standing), { make: true, output: null, index: 1 });
        expect("one kept under another goes under it",
            Places.whereBack({ screen: monitor, output: "DP-4", workspace: 5, column: 2, tile: 2 },
                beside), { join: 24, after: 4, under: true });

        // Whether each window back is where it goes already, so the pass
        // after Reopen Them All moves only what Joel's focus or a screen
        // coming and going sent somewhere else.
        const now = (id, idx, column) => ({ workspace: { id: 20 + idx, idx, output: "DP-4" },
            column });
        const after4 = { join: 24, after: 4 };
        expect("a window just right of the one it goes after holds",
            Places.holds(after4, now(3, 4, 4), beside), true);
        expect("one left of it doesn't", Places.holds(after4, now(3, 4, 1), beside), false);
        expect("nor one on another workspace",
            Places.holds(after4, now(3, 3, 4), beside), false);
        expect("one under another holds in its column",
            Places.holds({ join: 24, after: 4, under: true }, now(3, 4, 3), beside), true);
        expect("one just left of the one it goes before holds",
            Places.holds({ join: 24, before: 4 }, now(3, 4, 2), beside), true);
        const made = { make: true, output: "DP-4", index: 5 };
        // A workspace's number is where it stands now, never what it is: one
        // emptied before it, or made, moves every number after it, so a
        // window alone on its own holds whatever number it has (P9R1KI,
        // 2026-11-09: 17 windows moved on screen at the 08:06 boot, chasing
        // a number the workspace before them had taken away).
        expect("one on a workspace of its own holds at its index or after",
            Places.holds(made, now(6, 6, 1), beside), true);
        expect("and before it, once a workspace before it has gone",
            Places.holds(made, now(6, 2, 1), beside), true);
        expect("nor among the others", Places.holds(made, now(6, 5, 1), beside), false);
        expect("a window on the workspace it joins goes just right of the one it follows",
            [Places.columnTo(after4, 1, beside), Places.columnTo(after4, 5, beside)], [3, 4]);
        expect("or just left of the one it goes before",
            [Places.columnTo({ join: 24, before: 4 }, 1, beside),
                Places.columnTo({ join: 24, before: 4 }, 5, beside)], [2, 3]);
        expect("a window opening on the workspace it joins opens just right of the one it follows",
            Places.opensAt(after4, beside), 4);
        expect("or in the column of the one it goes before, which moves right",
            Places.opensAt({ join: 24, before: 4 }, beside), 3);
        expect("one going under another opens just right of it, to be taken in after",
            Places.opensAt({ join: 24, after: 4, under: true }, beside), 4);
        expect("one given a workspace of its own opens at none", Places.opensAt(made, beside),
            null);

        // A session its orchestrator hands over goes where it sat this boot,
        // else where the restored boot kept it.
        const here = { output: "eDP-1", workspace: 1, column: 2, tile: 1 };
        const restoredAt = { output: "DP-4", workspace: 5, column: 1, tile: 1 };
        const keptPlaces = { now: { "01-a": here },
            restored: { "01-a": restoredAt, "01-b": restoredAt } };
        expect("handed over: this boot's, the restored boot's, none",
            ["01-a", "01-b", "01-c"].map(id => Places.keptFor(id, keptPlaces)),
            [here, restoredAt, null]);
        // A session started under an orchestrator opens on the orchestrator's
        // workspace, in title order after it, wherever the session that
        // started it sits; one under it elsewhere doesn't count.
        const lead = { workspace: 7, column: 2, title: "Reef Orchestrator" };
        const under = [{ workspace: 7, column: 3, title: "Anchor Checks" },
            { workspace: 7, column: 4, title: "Kelp Survey" },
            { workspace: 5, column: 1, title: "Buoy Lights" }];
        expect("under an orchestrator, by title: between, first, last, untitled last",
            ["Coral Counts", "Abalone", "Zebra Mussels", ""]
                .map(title => Places.underOrchestrator(title, lead, under)),
            [{ workspace: 7, column: 4 }, { workspace: 7, column: 3 },
                { workspace: 7, column: 5 }, { workspace: 7, column: 5 }]);
        expect("with no orchestrator here, nowhere of its own",
            Places.underOrchestrator("Coral Counts", null, under), null);
        // A session goes home to its orchestrator's workspace, by the
        // address of the top orchestrator over its quest as the harness
        // hands it over, else the orchestrator it names, else the one its
        // place last boot was kept under; never by a word that's no address.
        expect("home: handed over, said, kept, none, and no address",
            [Places.homeOf({ workspace: "REEF", orchestratedBy: "KELP" }, { under: "BUOY" }),
                Places.homeOf({ orchestratedBy: "KELP" }, { under: "BUOY" }),
                Places.homeOf({ id: "01-a" }, { under: "BUOY" }),
                Places.homeOf({ id: "01-a" }, null),
                Places.homeOf({ workspace: "reef; rm" }, null)],
            ["REEF", "KELP", "BUOY", "", ""]);
        // An orchestrator back at its kept place names that workspace for
        // itself, so a session of its own back before its pi says who it is
        // joins it there, rather than making a workspace of its own and
        // staying alone on it (Joel, 2026-11-14 22:36: "why did this window
        // get put on a workspace by itself?").
        const spaces = [{ id: 3, name: null }, { id: 5, name: "KELP" }];
        expect("named for its orchestrator: by address, by what it orchestrates",
            [Places.homeToName({ address: "REEF" }, "REEF", 3, spaces),
                Places.homeToName({ orchestrates: "REEF" }, "REEF", 3, spaces)],
            ["REEF", "REEF"]);
        expect("named for nothing: one it leads, its own name, another's, nowhere, no home",
            [Places.homeToName({ id: "01-a" }, "REEF", 3, spaces),
                Places.homeToName({ address: "KELP" }, "KELP", 3, spaces),
                Places.homeToName({ address: "REEF" }, "REEF", 5, spaces),
                Places.homeToName({ address: "REEF" }, "REEF", null, spaces),
                Places.homeToName({ address: "REEF" }, "", 3, spaces)],
            ["", "", "", "", ""]);
        // One handed over while Reopen Them All already holds it, or has
        // begun it, joins nothing, or two pis would write one log.
        const asked = id => ({ one: { id } });
        expect("handed over again while queued or begun, it isn't asked twice",
            Places.queuedOnce([asked("q1"), asked("q2")], [asked("q2"), asked("q3"), asked("q4")],
                ["q4"]).map(entry => entry.one.id), ["q1", "q2", "q3"]);
        // Those whose kept place says they led others are begun first, so
        // the sessions they lead have them back to answer to; the rest,
        // those kept nowhere among them, keep their order.
        const queued = (id, leads) => ({ one: { id },
            kept: leads === undefined ? null : Object.assign({ leads }, here) });
        expect("orchestrators first, each part in its order",
            Places.leadersFirst([queued("a", false), queued("b", true), queued("c"),
                queued("d", true), queued("e", false)]).map(entry => entry.one.id),
            ["b", "d", "a", "c", "e"]);

        // Reopen Them All begins the next session only while the machine has
        // room for it, as the machine queue judges it, so eighty sessions
        // never start into a machine already short of memory or disk.
        const queue = (over, io, heartbeat) => JSON.stringify({ v: 1,
            heartbeat: heartbeat ?? 5000,
            machine: { over, pressure: { cpu: 30, memory: 2, io } } });
        expect("room: the next begins", Reopening.mayBegin(queue([], 10), 5010), true);
        expect("memory over: it waits", Reopening.mayBegin(queue(["memory"], 10), 5010), false);
        expect("processors over: it waits", Reopening.mayBegin(queue(["cpu"], 10), 5010), false);
        expect("the disk stalling: it waits", Reopening.mayBegin(queue([], 60), 5010), false);
        // A queue that isn't heard from says nothing of the machine, so it
        // never holds a reopening for good.
        expect("a queue gone quiet holds nothing",
            Reopening.mayBegin(queue(["memory"], 90, 5000), 5000 + 121), true);
        expect("no queue holds nothing",
            [Reopening.mayBegin("", 5010), Reopening.mayBegin("{", 5010)], [true, true]);

        // Thirty reopened at once each open as before, but are told to
        // carry on only as the uplink has room, since a session told to
        // carry on is what sends: on 2026-11-14 seventy-two told at once
        // filled the home line's upload for a quarter of an hour.
        {
        const thirty = Array.from({ length: 30 }, (_, at) => ({ id: `01-${at}`,
            address: `S${at}`, carried: at === 20 ? { from: "far", at: "1" } : undefined,
            leads: at === 25 }));
        const sending = (mbit, heartbeat) => JSON.stringify({ v: 1, heartbeat: heartbeat ?? 5000,
            machine: { over: [], pressure: { io: 1 }, uplink: mbit === undefined ? undefined
                : { sentMbit: mbit } } });
        const ids = told => told.map(one => one.id);
        const due = (text, told) => Reopening.carryOnsDue(thirty, told, text, 5010);
        expect("the line full: none is told yet", ids(due(sending(6.1), [])), []);
        expect("the line with room: one at a time", due(sending(2.0), []).length, 1);
        // Those waiting on Joel first, then one carried in, then a leader.
        const wanting = thirty.map(one => Object.assign({}, one, { wants: one.id === "01-7" }));
        expect("waiting on Joel, then carried in, then leaders, then the rest",
            ids(Reopening.carryOnsDue(wanting, [], sending(undefined), 5010)),
            ["01-7", "01-20", "01-25", "01-0"]);
        // With no uplink said yet, four at a time, the next once one has
        // gone idle or a minute has passed since it was told.
        const told = (at, idle) => thirty.slice(0, 4).map(one => ({ id: one.id, at, idle }));
        expect("four working: none more", due(sending(undefined), told(5000, false)).length, 0);
        expect("one gone idle: one more", due(sending(undefined),
            told(5000, false).map((one, at) => at === 0 ? Object.assign({}, one, { idle: true })
                : one)).length, 1);
        expect("a minute on: four more", due(sending(undefined), told(4940, false)).length, 4);
        // A queue not heard from says nothing of the line, so all are told.
        expect("a queue gone quiet: all told at once",
            due(sending(6.1, 5010 - 121), []).length, 30);
        expect("no queue: all told at once", Reopening.carryOnsDue(thirty, [], "", 5010)
            .length, 30);
        // Before the line has room again, no more than the few allowed go:
        // thirty pending through ten looks at a full line, none told.
        let sent = [];
        for (let look = 0; look < 10; look += 1)
            sent = sent.concat(due(sending(5.0), sent));
        expect("thirty reopened, the line full: none told before room returns", sent.length, 0);
        let few = [];
        for (let look = 0; look < 10; look += 1)
            few = few.concat(due(sending(undefined), few)
                .map(one => ({ id: one.id, at: 5010, idle: false })));
        expect("thirty reopened, no uplink said: four told until one is done", few.length, 4);
        }

        // A session moving onto pi's host keeps where its window was, as
        // the windows daemon's record of it says, so its viewer opens there
        // once it runs hosted; one with no window, or none laid out yet,
        // keeps nothing.
        const laid = (workspace, column) => ({ id: 7, workspace, column, down: 1 });
        expect("a window's place kept", Viewers.placeOf(laid(4, 3), 100),
            { workspace: 4, column: 3, at: 100 });
        expect("a window floating keeps its workspace alone",
            Viewers.placeOf(laid(4, null), 100), { workspace: 4, column: null, at: 100 });
        expect("no window keeps nothing", [Viewers.placeOf(null, 100),
            Viewers.placeOf(laid(null, null), 100)], [null, null]);

        // Each kept place is one viewer to open once its session runs
        // hosted with words to view it by, and then kept no more; one whose
        // viewer is open already is let go, and one a day old lapses, as
        // one whose session never moved would.
        const day = 24 * 60 * 60 * 1000;
        const hosted = (id, window) => ({ window: window ?? null, attach: ["attach", id],
            presence: { session: id, hosted: { host: "harbour-attended" } } });
        const plain = id => ({ window: laid(1, 1), attach: null, presence: { session: id } });
        const moving = { moved: { workspace: 4, column: 3, at: 0 },
            moving: { workspace: 5, column: 1, at: 0 },
            viewed: { workspace: 6, column: 2, at: 0 },
            stale: { workspace: 7, column: 1, at: -day } };
        // niri's workspaces by id: Joel on 9 of the monitor, the laptop's
        // 4 to 7 behind it, and the empty one niri keeps last on each.
        const workspace = (id, idx, output, windowId) => ({ id, idx, output,
            is_focused: false, is_active: false, active_window_id: windowId });
        const on = (focusedId, all) => {
            const now = {};
            for (const one of Object.values(all))
                now[one.id] = Object.assign({}, one, { is_focused: one.id === focusedId });
            return now;
        };
        const screens = on(9, {
            4: workspace(4, 1, "eDP-1", 40), 5: workspace(5, 2, "eDP-1", 50),
            6: workspace(6, 3, "eDP-1", 60), 7: workspace(7, 4, "eDP-1", 70),
            8: workspace(8, 5, "eDP-1", null),
            9: workspace(9, 1, "DP-2", 90), 10: workspace(10, 2, "DP-2", null) });
        // Kept since the shell started a day ago, with Joel away, unless a
        // case says otherwise.
        const gone = (all, said) => Object.assign({ workspaces: all, since: -day, away: true },
            said ?? {});
        const due = Viewers.viewersDue(moving, [hosted("moved"), plain("moving"),
            hosted("viewed", laid(6, 2)), hosted("stale")], 10, gone(screens));
        expect("the moved one's viewer opens where it was",
            due.open.map(one => [one.session.presence.session, one.into]),
            [["moved", { workspace: 4, column: 3 }]]);
        expect("only the one still moving stays kept", due.kept,
            { moving: { workspace: 5, column: 1, at: 0 } });
        expect("one hosted with no words waits", Viewers.viewersDue(moving,
            [Object.assign(hosted("moved"), { attach: [] })], 10, gone(screens)).kept.moved,
            { workspace: 4, column: 3, at: 0 });
        expect("nothing kept, nothing to open", Viewers.viewersDue({}, [hosted("moved")], 10,
            gone(screens)), { open: [], kept: {} });

        // Nothing opens by itself while Joel is at the machine, nothing kept
        // before this shell started, as one restarted by a switch finds,
        // and nothing parked in its host, which viewing would wake: each
        // waits, kept (OSPHT4 [0762284c], 13:41:38 on 11 November).
        const alone = { moved: { workspace: 4, column: 3, at: 0 } };
        expect("with Joel there it waits", Viewers.viewersDue(alone, [hosted("moved")], 10,
            gone(screens, { away: false })), { open: [], kept: alone });
        expect("kept before the shell started, it waits", Viewers.viewersDue(alone,
            [hosted("moved")], 10, gone(screens, { since: 5 })), { open: [], kept: alone });
        const parked = Object.assign(hosted("moved"), { presence: Object.assign(
            {}, hosted("moved").presence, { parked: { since: "2026-11-11T09:00:00Z" } }) });
        expect("parked in its host, it waits", Viewers.viewersDue(alone, [parked], 10,
            gone(screens)), { open: [], kept: alone });
        expect("its viewer open, it is let go however Joel is", Viewers.viewersDue(alone,
            [hosted("moved", laid(4, 3))], 10, gone(screens, { away: false })),
            { open: [], kept: {} });

        // A viewer never opens where Joel is, so it never takes his focus
        // or flashes beside him: on the workspace he's on it waits until he
        // leaves, and one whose workspace went with its window waits rather
        // than taking another (OSPHT4 [0762284c]).
        const place = (workspace, column) => ({ moved: { workspace, column, at: 0 } });
        const opened = (kept, all) => Viewers.viewersDue(kept, [hosted("moved")], 10, gone(all));
        expect("on Joel's workspace it waits", opened(place(9, 2), screens),
            { open: [], kept: place(9, 2) });
        expect("once he leaves, it opens there",
            opened(place(9, 2), on(4, screens)).open.map(one => one.into),
            [{ workspace: 9, column: 2 }]);
        expect("its workspace gone, it waits", opened(place(3, 1), screens),
            { open: [], kept: place(3, 1) });
        expect("no workspaces known, it waits", opened(place(4, 3), {}).kept, place(4, 3));

        // A session put to sleep has no window, and waking it opens none:
        // a window opens only when Joel asks for one (Joel, 16 November:
        // "i would love for any mycelial node to not require a live
        // window"). Only Reopen Them All asks, so a sleeper it wakes opens
        // where it was, in pi's host or in its holder, with Joel there too,
        // but never on his workspace, never while it sleeps, and only
        // within ASKED_FOR of his asking. Kept just before its window
        // closes, it waits to be seen asleep, and lapses in a day if it
        // never is; once seen, it's kept however long it sleeps, unless it
        // was asked for, which lapses ASKED_FOR after the ask.
        const slept = { napper: { workspace: 4, column: 3, at: 0, slept: true } };
        const asleep = { napper: { workspace: 4, column: 3, at: 0, slept: true, asleep: true } };
        const reopened = { napper: { workspace: 4, column: 3, at: 0, slept: true, asleep: true,
            asked: true } };
        // Joel at the machine, with the shell started after the place was kept.
        const there = gone(screens, { away: false, since: 5 });
        const napping = (session, extra) => Object.assign(session, { presence: Object.assign(
            {}, session.presence, extra) });
        const hostedNapping = napping(hosted("napper"), { parked: { since: "2026-11-14" } });
        const held = (window, state) => ({ window: window ?? null, attach: null, held: "h1",
            presence: { session: "napper", state } });
        expect("its window still open, not yet asleep, it waits", Viewers.viewersDue(slept,
            [hosted("napper", laid(4, 3))], 10, there), { open: [], kept: slept });
        expect("its window gone before the host parked it, it waits", Viewers.viewersDue(slept,
            [hosted("napper")], 10, there), { open: [], kept: slept });
        expect("asleep in its host, it's marked so and waits", Viewers.viewersDue(slept,
            [hostedNapping], 10, there), { open: [], kept: asleep });
        expect("asleep in its holder, the same", Viewers.viewersDue(slept,
            [held(null, "parked")], 10, there), { open: [], kept: asleep });
        expect("woken in its host, unasked, it opens no window and is let go",
            Viewers.viewersDue(asleep, [hosted("napper")], 10, there), { open: [], kept: {} });
        expect("woken in its holder, unasked, the same", Viewers.viewersDue(asleep,
            [held(null, "listening")], 10, there), { open: [], kept: {} });
        expect("woken by Reopen Them All in its host, it opens where it was",
            Viewers.viewersDue(reopened, [hosted("napper")], 10, there).open
                .map(one => [one.session.presence.session, one.into]),
            [["napper", { workspace: 4, column: 3 }]]);
        expect("woken by Reopen Them All in its holder, the same", Viewers.viewersDue(reopened,
            [held(null, "listening")], 10, there).open.map(one => [one.session.held, one.into]),
            [["h1", { workspace: 4, column: 3 }]]);
        expect("woken past ASKED_FOR of the ask, it opens none and is let go",
            Viewers.viewersDue(reopened, [held(null, "listening")], Viewers.ASKED_FOR + 1, there),
            { open: [], kept: {} });
        expect("asked and still asleep past ASKED_FOR, it's let go", Viewers.viewersDue(reopened,
            [hostedNapping], Viewers.ASKED_FOR + 1, there), { open: [], kept: {} });
        const askedLater = { napper: Object.assign({}, reopened.napper, { at: 5000 }) };
        expect("asked later, ASKED_FOR runs from the ask, not from nothing",
            Viewers.viewersDue(askedLater, [held(null, "listening")], 5000 + Viewers.ASKED_FOR,
                there).open.length, 1);
        expect("and past it from the ask, it's let go", Viewers.viewersDue(askedLater,
            [held(null, "listening")], 5001 + Viewers.ASKED_FOR, there), { open: [], kept: {} });
        const askedWhenUnread = { napper: Object.assign({}, reopened.napper, { at: "soon" }) };
        expect("asked when nobody can read, it opens none and is let go",
            Viewers.viewersDue(askedWhenUnread, [held(null, "listening")], 10, there),
            { open: [], kept: {} });
        expect("woken while Joel is on its workspace, it waits", Viewers.viewersDue(reopened,
            [hosted("napper")], 10, gone(on(4, screens), { away: false })),
            { open: [], kept: reopened });
        expect("woken with no words to view it by yet, it waits", Viewers.viewersDue(reopened,
            [Object.assign(hosted("napper"), { attach: [] })], 10, there),
            { open: [], kept: reopened });
        expect("woken and opened already, as by Go To, it's let go", Viewers.viewersDue(reopened,
            [hosted("napper", laid(5, 1))], 10, there), { open: [], kept: {} });
        expect("asleep two days, it's still kept", Viewers.viewersDue(asleep,
            [hostedNapping], 2 * day, there).kept, asleep);
        expect("asleep and unlisted, as its host starts again, it's kept",
            Viewers.viewersDue(asleep, [], 2 * day, there).kept, asleep);
        expect("never seen asleep, it lapses in a day", Viewers.viewersDue(slept,
            [hosted("napper")], 2 * day, there), { open: [], kept: {} });

        // Reopen Them All begins no session a pi runs here, wakes one
        // parked here, and believes a hosted presence only while its host's
        // own list names it (OSPHT4 [58d7fa8e], 15 November).
        const owner = { pid: 4100, startedAt: 17, bootId: "b1", startTicks: 7,
            host: "work-background", session: "s-a" };
        const list = (fields, ids) => JSON.stringify(Object.assign({ v: 1,
            key: "work-background", owner: { pid: 4100, startedAt: 17, bootId: "b1",
                startTicks: 7 }, sessions: ids.map(id => ({ id, state: "idle" })) }, fields));
        expect("a host's list named by its pid and start ticks", Places.hostListOf(owner),
            "4100-7.json");
        expect("by its start, without ticks", Places.hostListOf({ pid: 4100, startedAt: 17 }),
            "4100-17.json");
        expect("no host process, no list", Places.hostListOf({ host: "work-background" }), "");
        expect("its host lists it", Places.hostSays(owner, list({}, ["s-b", "s-a"])),
            "running");
        expect("its host lists others", Places.hostSays(owner, list({}, ["s-b"])), "gone");
        expect("no list says nothing", Places.hostSays(owner, ""), "unknown");
        expect("another host's list says nothing", Places.hostSays(owner,
            list({ key: "work-attended" }, [])), "unknown");
        expect("another start's list says nothing", Places.hostSays(owner,
            list({ owner: { pid: 4100, startedAt: 18, bootId: "b1", startTicks: 9 } }, [])),
            "unknown");
        const pi = (id, presence) => ({ here: true, address: id.toUpperCase(),
            presence: Object.assign({ session: id }, presence) });
        const hostedHere = pi("s-a", { hosted: { host: "work-background" }, owner });
        const found = (id, running, parked, says) => {
            const at = Places.foundHere(id, running, parked, () => says);
            return [at.as, at.session?.address ?? null];
        };
        expect("one a pi runs here is found running", found("s-a", [pi("s-a")], [], "gone"),
            ["running", "S-A"]);
        expect("one parked here is found parked", found("s-a", [], [pi("s-a")], "gone"),
            ["parked", "S-A"]);
        expect("one elsewhere isn't here", found("s-a",
            [Object.assign(pi("s-a"), { here: false })], [], "gone"), ["", null]);
        expect("one hosted its host lists runs", found("s-a", [hostedHere], [], "running"),
            ["running", "S-A"]);
        expect("one hosted its host doesn't list is stale",
            found("s-a", [hostedHere], [], "gone"), ["stale", "S-A"]);
        expect("one hosted with its host's list unread runs",
            found("s-a", [hostedHere], [], "unknown"), ["running", "S-A"]);
        expect("none", found("s-a", [pi("s-b")], [pi("s-c")], "gone"), ["", null]);

        // A session whose folder is gone starts afresh on its quest, in the
        // folder the harness's quest index says a new one starts in.
        const index = JSON.stringify({ quests: [
            { id: "QEST-REEF", start: "/home/skiff/harbour" },
            { id: "QEST-KEEL", start: "harbour" },
            { id: "QEST-HULL" }] });
        expect("a quest's start", Places.questStart(index, "QEST-REEF"), "/home/skiff/harbour");
        expect("a start not a whole path is none", Places.questStart(index, "QEST-KEEL"), "");
        expect("a quest saying no start has none", Places.questStart(index, "QEST-HULL"), "");
        expect("a quest not held has none", Places.questStart(index, "QEST-MAST"), "");
        expect("no index says none", Places.questStart("", "QEST-REEF"), "");
        let rolled = 0;
        const rolls = [0, 0.5, 0.99];
        const roll = () => rolls[rolled++ % rolls.length];
        const minted = Places.freshId(0x019a1b2c3d4e, roll);
        const version7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;
        expect("a fresh id is a UUID version 7", version7.test(minted), true);
        expect("a fresh id begins with when", minted.slice(0, 13), "019a1b2c-3d4e");
        expect("two fresh ids differ", Places.freshId(1, Math.random)
            === Places.freshId(1, Math.random), false);
        const lost = { id: "s-a", cwd: "/gone", quest: "QEST-REEF", title: "Reef" };
        expect("afresh starts anew in its quest's start", Places.afresh(lost, "/home/skiff/harbour",
            "s-new"), { id: "s-new", cwd: "/home/skiff/harbour", quest: "QEST-REEF",
            title: "Reef", fresh: true, afreshFor: "s-a" });
        expect("afresh leaves the one lost as it was", lost.id, "s-a");

        if (failures.length > 0) {
            for (const failure of failures)
                console.warn(failure);
            // Qt.exit only asks the engine to stop once this returns.
            Qt.exit(1);
            return;
        }
        console.info("every place holds");
        Qt.exit(0);
    }
}
