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
