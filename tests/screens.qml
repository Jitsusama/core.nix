// What the : line's Brightness and the Warmth's switches ask of the rows
// Screens serves, run in Qt's own JavaScript engine, the one the shell
// runs it in. The `the-launcher-asks-the-served-light` check copies
// modules/quickshell/launcher/Screens.js beside this file and runs
// `qml -platform offscreen screens.qml`. Which ways a screen offers, and
// what :setl power and unplugged do with them, are the windows daemon's,
// tested in modules/windows/src/screens/acts.rs and screens/line/tests.rs
// (Joel, 2026-11-07: "i want to be able to control the screen that gets
// shut off, one, the other, all").
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

    function run() {
        const failures = [];
        // The last lit screen's row, as screens/acts.rs serves it.
        const lastLit = ["displays:focus", "verb:display-off"].map(key => ({
            key: key,
            ask: { ask: "screen", do: key === "displays:focus" ? "focus" : "dark",
                output: "eDP-1" }
        }));

        // The : line's :set brightness= and a typed "brightness 40" ask
        // what a Brightness level asks, at the share typed, the first step
        // kept lit; with no level served they ask nothing.
        const level = share => ({ key: "row:displays:choice:eDP-1:brightness-" + share,
            acts: [{ key: "displays:brightness:level",
                ask: { ask: "light", do: "brightness", share: share } }] });
        const rows = [{ key: "row:displays:display:eDP-1", acts: lastLit }, level(100),
            level(90)];
        check(failures, 'a share typed asks as a level does',
            Screens.brightening(rows, 45), { ask: "light", do: "brightness", share: 45 });
        check(failures, 'and none at all keeps the first step',
            Screens.brightening(rows, 0)?.share, 1);
        check(failures, 'but asks nothing with no level served',
            Screens.brightening(rows.slice(0, 1), 45), null);
        check(failures, 'and leaves the level\'s own ask as it was',
            rows[1].acts[0].ask.share, 100);

        // The : line's :set nightlight and :set warmth ask what the
        // Warmth's served switches ask, only to turn one the other way,
        // and are offered only while it's served.
        const night = on => ({ key: "row:displays:switch:night-light", held: { on: on },
            acts: [{ key: "displays:night-light",
                ask: { ask: "light", do: "night-light", on: !on } }] });
        check(failures, 'a switch served says how it stands',
            Screens.switchOf([rows[0], night(true)], "displays:night-light")?.on, true);
        check(failures, 'and turned the other way asks what its Return asks',
            Screens.switchOf([night(true)], "displays:night-light").asked(false),
            { ask: "light", do: "night-light", on: false });
        check(failures, 'but turned the way it stands asks nothing',
            Screens.switchOf([night(false)], "displays:night-light").asked(false), null);
        check(failures, 'and one not served is no switch',
            Screens.switchOf(rows, "displays:follow-room"), null);

        // The Brightness the windows daemon serves says the share it read, which :set brightness and a typed
        // "brightness 40" say; none while it's being read, and nothing
        // with no Brightness served.
        const brightness = (held, side) => ({ key: "row:displays:setting:DP-1:brightness",
            thing: "displays:DP-1:brightness", kind: "setting",
            parent: "row:displays:display:DP-1", held: held, side: side,
            acts: [{ key: "verb:more", step: "rows" }] });
        check(failures, 'a Brightness served says its share',
            Screens.brightnessOf([rows[0], brightness({ share: 60 }, [{ said: "60%" }]),
                level(100)]),
            { share: 60, said: "60%" });
        check(failures, 'and no share while it\'s read, saying nothing at first',
            Screens.brightnessOf([brightness({}, [])]),
            { share: null, said: "" });
        check(failures, 'then that it isn\'t read yet',
            Screens.brightnessOf([brightness({}, [{ said: "not read yet" }])])?.said,
            "not read yet");
        check(failures, 'and none with none served, its levels alone not one',
            Screens.brightnessOf(rows), null);

        for (const failure of failures)
            console.warn(failure);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
