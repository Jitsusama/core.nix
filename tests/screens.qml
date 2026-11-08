// How Screens turns one screen off or on, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `the-launcher-turns-each-screen-off`
// check copies modules/quickshell/launcher/Screens.js beside this file and
// runs `qml -platform offscreen screens.qml`. A niri that powers one output
// off by name darkens a screen, keeping it docked with its hub powered, and
// unplugs it only as the bigger step; an older niri only unplugs, as before
// (Joel, 2026-11-07: "i want to be able to control the screen that gets shut
// off, one, the other, all").
import QtQuick
import "Screens.js" as Screens

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

    function check(failures, what, got, want) {
        const said = JSON.stringify(got);
        if (said !== JSON.stringify(want))
            failures.push(`${what}: got ${said}, wanted ${JSON.stringify(want)}`);
    }

    // A screen as niri lists it: in its outputs while `logical` isn't null,
    // and dark while `powered_off`, which an older niri never says.
    function screen(name, logical, dark) {
        const output = { name: name, logical: logical ? { scale: 1 } : null };
        if (dark !== undefined)
            output.powered_off = dark;
        return output;
    }

    // What each way offered asks niri, by its word, so a case reads as the
    // step-in would.
    function asked(ways) {
        return ways.map(way => [way.word, way.asks, way.note]);
    }

    function run() {
        const failures = [];
        const panel = screen("eDP-1", true, false);
        const hp = screen("DP-2", true, false);
        const darkHp = screen("DP-2", true, true);
        const pulled = screen("DP-2", false, false);

        check(failures, "a lit screen is lit", Screens.standing(panel), "lit");
        check(failures, "one powered off by name is dark", Screens.standing(darkHp), "dark");
        check(failures, "one out of niri's outputs is off", Screens.standing(pulled), "off");
        check(failures, "an older niri's lit screen is lit",
            Screens.standing(screen("DP-2", true)), "lit");

        check(failures, "beside another lit, Turn It Off darkens it, and Unplug It follows",
            asked(Screens.ways(hp, [panel, hp], true)), [
                ["off", { Action: { PowerOffMonitor: { output: "DP-2" } } }, ""],
                ["unplug", { Output: { output: "DP-2", action: "Off" } }, "moves"]
            ]);
        check(failures, "the last lit screen darkens too, a key waking it, and isn't unplugged",
            asked(Screens.ways(panel, [panel, darkHp], true)), [
                ["off", { Action: { PowerOffMonitor: { output: "eDP-1" } } }, "wakes"]
            ]);
        check(failures, "a dark screen turns on by name",
            asked(Screens.ways(darkHp, [panel, darkHp], true)), [
                ["on", { Action: { PowerOnMonitor: { output: "DP-2" } } }, ""]
            ]);
        check(failures, "an unplugged screen is put back in niri's outputs",
            asked(Screens.ways(pulled, [panel, pulled], true)), [
                ["on", { Output: { output: "DP-2", action: "On" } }, ""]
            ]);
        check(failures, "an older niri only unplugs, as Turn It Off",
            asked(Screens.ways(hp, [panel, hp], false)), [
                ["off", { Output: { output: "DP-2", action: "Off" } }, ""]
            ]);
        check(failures, "and keeps the last screen on",
            asked(Screens.ways(panel, [panel, pulled], false)), []);

        // niri lights every screen at any key once all are off by name, so
        // the key that asked would light them again at once.
        check(failures, "darkening the last lit screen waits for the key to come up",
            Screens.ways(panel, [panel, darkHp], true).map(way => way.waits), [true]);
        check(failures, "darkening one beside another lit doesn't",
            Screens.ways(hp, [panel, hp], true).map(way => way.waits), [false, false]);

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
