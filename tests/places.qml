// Where Reopen Them All puts each session back, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `reopening-finds-each-screen`
// check copies modules/quickshell/Places.js beside this file and
// runs `qml -platform offscreen places.qml`. Each case is one a boot or a
// dock can make: the same monitor on another connector, a screen gone, and
// a place kept before screens were named by what they are.
import QtQuick
import "Places.js" as Places

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
                { screen: panel, workspace: 2 },
                { screen: panel, workspace: 1 }
            ]);

        // The monitor comes after the reopen began: the workspaces given
        // the panel in its place go to it, each to its rank by kept number,
        // lowest first, since where niri puts each can't be foretold; one
        // gone stays gone, and one there already is only set to its rank.
        const away = { [monitor]: [
            { id: 11, workspace: 1 }, { id: 13, workspace: 3 }, { id: 12, workspace: 2 },
            { id: 14, workspace: 4 }, { id: 15, workspace: 5 }
        ] };
        const workspaces = [
            { id: 11, output: "eDP-1" }, { id: 12, output: "eDP-1" }, { id: 13, output: "eDP-1" },
            { id: 15, output: "DP-4" }, { id: 16, output: "DP-4" }
        ];
        expect("a screen come back takes its workspaces, each to its rank",
            Places.homeward(away, { "eDP-1": panel, "DP-4": monitor }, workspaces), {
                moves: [{ output: "DP-4", id: 11, index: 1 },
                    { output: "DP-4", id: 12, index: 2 },
                    { output: "DP-4", id: 13, index: 3 },
                    { output: "DP-4", id: 15, index: 4, there: true }],
                home: [monitor]
            });
        expect("a screen still gone keeps its workspaces waiting",
            Places.homeward(away, { "eDP-1": panel }, workspaces), { moves: [], home: [] });

        // Windows that reopen themselves, matched by program and title,
        // else the first of that program, each kept window used once.
        const others = [
            { app: "google-chrome", title: "Docs" },
            { app: "google-chrome", title: "Open · Acceluary" },
            { app: "slack", title: "Slack" }
        ];
        expect("a window is the kept one of its program with its title",
            Places.matchOther(others, [], "google-chrome", "Open · Acceluary"), 1);
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
        expect("one on a workspace of its own holds at its index or after",
            Places.holds(made, now(6, 6, 1), beside), true);
        expect("but not before it", Places.holds(made, now(6, 2, 1), beside), false);
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

        // A reboot's list, taken a minute before this boot began.
        const began = Date.parse("2026-11-07T17:00:00Z");
        const listed = (fields) => JSON.stringify(Object.assign({ v: 1, boot: "the-last",
            taken: "2026-11-07T16:59:00Z", sessions: [
                { id: "01-boss", cwd: "/home/j/boss", title: "Boss", output: "eDP-1",
                    workspace: 2, column: 1 },
                { id: "01-loose", cwd: "/home/j/loose", title: "Loose", orchestrates: "REEF01" },
                { id: "01; rm -rf ~", cwd: "/tmp" },
                { id: "01-nowhere", title: "Nowhere" }
            ] }, fields));
        const ids = list => list.sessions.map(one => one.id);
        expect("a reboot's list names each whole one, with its place where it has one",
            Places.rebootList(listed({}), "this-one", began).sessions, [
                { id: "01-boss", cwd: "/home/j/boss", title: "Boss",
                    at: { output: "eDP-1", workspace: 2, column: 1, tile: 1 }, orchestrates: "" },
                { id: "01-loose", cwd: "/home/j/loose", title: "Loose", at: null,
                    orchestrates: "REEF01" }
            ]);
        // Two windows stacked in one column are each kept at their own
        // tile, as the list says it or else in the order it lists them, so
        // neither is moved onto the other's place: after the 21:24 boot on
        // 2026-11-08 two kept at one column, both at tile 1, swapped 25
        // times.
        const stacked = JSON.stringify({ v: 1, boot: "the-last",
            taken: "2026-11-07T16:59:00Z", sessions: [
                { id: "01-top", cwd: "/tmp", output: "eDP-1", workspace: 3, column: 3 },
                { id: "01-under", cwd: "/tmp", output: "eDP-1", workspace: 3, column: 3 },
                { id: "01-said", cwd: "/tmp", output: "eDP-1", workspace: 4, column: 1,
                    tile: 2 },
                { id: "01-next", cwd: "/tmp", output: "eDP-1", workspace: 3, column: 4 }
            ] });
        expect("two in one column at a tile each, a tile the list says kept",
            Places.rebootList(stacked, "this-one", began).sessions.map(one => one.at.tile),
            [1, 2, 2, 1]);
        // A session its orchestrator hands over goes where it sat this boot,
        // else where the reboot's list says, else where the restored boot
        // kept it.
        const here = { output: "eDP-1", workspace: 1, column: 2, tile: 1 };
        const listedAt = { output: "eDP-1", workspace: 3, column: 3, tile: 2 };
        const restoredAt = { output: "DP-4", workspace: 5, column: 1, tile: 1 };
        const keptPlaces = { now: { "01-a": here }, reboot: { "01-a": listedAt,
            "01-b": listedAt }, restored: { "01-a": restoredAt, "01-b": restoredAt,
            "01-c": restoredAt } };
        expect("handed over: this boot's, the reboot's list, the restored boot's, none",
            ["01-a", "01-b", "01-c", "01-d"].map(id => Places.keptFor(id, keptPlaces)),
            [here, listedAt, restoredAt, null]);
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
        // Those that orchestrate are begun first, so the sessions they lead
        // have them back to answer to; the rest keep their order.
        const queued = (id, orchestrates) => ({ one: { id, orchestrates } });
        expect("orchestrators first, each part in its order",
            Places.leadersFirst([queued("a", ""), queued("b", "REEF01"), queued("c", ""),
                queued("d", "HARB02"), queued("e")]).map(entry => entry.one.id),
            ["b", "d", "a", "c", "e"]);
        expect("it says the boot it was taken in",
            Places.rebootList(listed({}), "this-one", began).boot, "the-last");
        expect("a list this boot took names none, so a shell restart offers nothing",
            ids(Places.rebootList(listed({ boot: "this-one" }), "this-one", began)), []);
        expect("nor one taken more than an hour before this boot began",
            ids(Places.rebootList(listed({ taken: "2026-11-07T15:59:00Z" }), "this-one",
                began)), []);
        expect("nor one taken after it began",
            ids(Places.rebootList(listed({ taken: "2026-11-07T17:01:00Z" }), "this-one",
                began)), []);
        expect("nor one with no boot, nor what isn't a list",
            [ids(Places.rebootList(listed({ boot: "" }), "this-one", began)),
                ids(Places.rebootList("{", "this-one", began))], [[], []]);

        // What Sessions to Reopen offers of the last boot, which nothing
        // reopens until Joel asks: the reboot's list while it names any,
        // each until it's back here, else what restore.json kept.
        const rebooted = Places.rebootList(listed({}), "this-one", began);
        const unlisted = Places.rebootList("{", "this-one", began);
        const restored = { boot: "the-last", sessions: [{ id: "01-kept", cwd: "/tmp" },
            { id: "01-ferried", cwd: "/tmp", carried: { from: "far", at: "" } }] };
        const offered = (list, putOff, back) => {
            const last = Places.lastBoot(list, restored, { boot: "this-one", putOff, back });
            return [last.boot, ids(last)];
        };
        expect("the reboot's list, over what restore.json kept",
            offered(rebooted, "", []), ["the-last", ["01-boss", "01-loose"]]);
        expect("each of the list until it's back here",
            offered(rebooted, "", ["01-boss"]), ["the-last", ["01-loose"]]);
        expect("with no list, what restore.json kept but the carried",
            offered(unlisted, "", []), ["the-last", ["01-kept"]]);
        expect("nothing of a boot Joel put off",
            [offered(rebooted, "the-last", [])[1], offered(unlisted, "the-last", [])[1]],
            [[], []]);
        expect("nothing of this boot's own", ids(Places.lastBoot(unlisted,
            { boot: "this-one", sessions: restored.sessions },
            { boot: "this-one", putOff: "", back: [] })), []);

        // Reopen Them All begins the next session only while the machine has
        // room for it, as the machine queue judges it, so eighty sessions
        // never start into a machine already short of memory or disk.
        const queue = (over, io, heartbeat) => JSON.stringify({ v: 1,
            heartbeat: heartbeat ?? 5000,
            machine: { over, pressure: { cpu: 30, memory: 2, io } } });
        expect("room: the next begins", Places.mayBegin(queue([], 10), 5010), true);
        expect("memory over: it waits", Places.mayBegin(queue(["memory"], 10), 5010), false);
        expect("processors over: it waits", Places.mayBegin(queue(["cpu"], 10), 5010), false);
        expect("the disk stalling: it waits", Places.mayBegin(queue([], 60), 5010), false);
        // A queue that isn't heard from says nothing of the machine, so it
        // never holds a reopening for good.
        expect("a queue gone quiet holds nothing",
            Places.mayBegin(queue(["memory"], 90, 5000), 5000 + 121), true);
        expect("no queue holds nothing",
            [Places.mayBegin("", 5010), Places.mayBegin("{", 5010)], [true, true]);

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
