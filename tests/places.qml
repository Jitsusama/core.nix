// Where Reopen Them All puts each session back, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `reopening-finds-each-screen`
// check copies modules/quickshell/Places.js and Viewers.js beside this file and
// runs `qml -platform offscreen places.qml`. Each case is one a boot or a
// dock can make: the same monitor on another connector, a screen gone, and
// a place kept before screens were named by what they are.
import QtQuick
import "Places.js" as Places
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
                { screen: panel, workspace: 2 },
                { screen: panel, workspace: 1 }
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

        // Where one of those offered apart from the last boot came from:
        // another machine carried it here, or pi's host gave it back, as
        // the agents daemon serves it (modules/agents's reopen.rs, whose
        // tests say which are offered, and in what order).
        const carried = (id, from, at) => ({ id, carried: { from, at } });
        const released = (id, from, at) => ({ id, released: { from, at } });
        expect("where each came from", [carried("c", "lighthouse", "1"),
            released("r", "attended", "2"), { id: "b" }].map(Places.apartOf),
            [{ kind: "carried", from: "lighthouse", at: "1" },
                { kind: "released", from: "attended", at: "2" }, null]);

        // Where they came from, as Sessions to Reopen's note says it: one
        // place alone by itself, several each counted, and many machines
        // counted rather than listed; those given back are counted alone
        // too, and never by their host's name.
        const said = { lastBoot: "from the last boot", carried: "carried from {machine}",
            machines: "{count} machines", givenBack: "given back by its host",
            counted: "{count} {from}" };
        const noted = (booted, apart) => Places.reopenNote(said, booted, apart);
        expect("the last boot's alone", noted(3, []), "from the last boot");
        expect("nothing at all reads as the last boot", noted(0, []), "from the last boot");
        expect("those given back alone, counted", noted(0, [released("a", "attended", ""),
            released("b", "background", "")]), "2 given back by its host");
        expect("one carried alone", noted(0, [carried("a", "lighthouse", "")]),
            "carried from lighthouse");
        expect("the last boot's and the carried", noted(3, [carried("a", "lighthouse", ""),
            carried("b", "lighthouse", "")]), "3 from the last boot, 2 carried from lighthouse");
        expect("every way they came", noted(1, [carried("a", "lighthouse", ""),
            carried("b", "buoy", ""), released("c", "attended", "")]),
            "1 from the last boot, 2 carried from 2 machines, 1 given back by its host");

        // Each row's own way, its host's name never among the words.
        expect("each row's way", [{ id: "b" }, carried("c", "lighthouse", ""),
            released("r", "attended", "")].map(one => Places.wayOf(said, one)),
            ["from the last boot", "carried from lighthouse", "given back by its host"]);

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
        const due = Viewers.viewersDue(moving, [hosted("moved"), plain("moving"),
            hosted("viewed", laid(6, 2)), hosted("stale")], 10, screens);
        expect("the moved one's viewer opens where it was",
            due.open.map(one => [one.session.presence.session, one.into]),
            [["moved", { workspace: 4, column: 3 }]]);
        expect("only the one still moving stays kept", due.kept,
            { moving: { workspace: 5, column: 1, at: 0 } });
        expect("one hosted with no words waits", Viewers.viewersDue(moving,
            [Object.assign(hosted("moved"), { attach: [] })], 10, screens).kept.moved,
            { workspace: 4, column: 3, at: 0 });
        expect("nothing kept, nothing to open",
            Viewers.viewersDue({}, [hosted("moved")], 10, screens), { open: [], kept: {} });

        // A viewer never opens where Joel is, so it never takes his focus
        // or flashes beside him: on the workspace he's on it waits until he
        // leaves, and one whose workspace went with its window opens on the
        // empty workspace last on his screen, out of his sight.
        const place = (workspace, column) => ({ moved: { workspace, column, at: 0 } });
        const opened = (kept, all) => Viewers.viewersDue(kept, [hosted("moved")], 10, all);
        expect("on Joel's workspace it waits", opened(place(9, 2), screens),
            { open: [], kept: place(9, 2) });
        expect("once he leaves, it opens there",
            opened(place(9, 2), on(4, screens)).open.map(one => one.into),
            [{ workspace: 9, column: 2 }]);
        expect("its workspace gone, it opens on his screen's empty one",
            opened(place(3, 1), screens).open.map(one => one.into),
            [{ workspace: 10, column: null }]);
        expect("its workspace gone with Joel on the empty one, it waits",
            opened(place(3, 1), on(10, screens)).kept, place(3, 1));
        expect("no workspaces known, it waits", opened(place(4, 3), {}).kept, place(4, 3));

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
