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
