// Where the Brightness Screens serves stands, run in Qt's own JavaScript
// engine, the one the shell runs it in. The `the-launcher-asks-the-served-light` check copies
// modules/quickshell/launcher/Screens.js beside this file and runs
// `qml -platform offscreen screens.qml`. Which ways a screen offers, and
// what :setl power and unplugged do with them, are the windows daemon's,
// tested in modules/windows/src/screens/acts.rs and screens/line/tests.rs,
// and so is what the place's :set line asks (screens/tests/set.rs)
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

        // A screen's row and the Brightness's levels, which are no
        // Brightness of their own.
        const level = share => ({ key: "row:displays:choice:eDP-1:brightness-" + share,
            acts: [{ key: "displays:brightness:level",
                ask: { ask: "light", do: "brightness", share: share } }] });
        const rows = [{ key: "row:displays:display:eDP-1", acts: lastLit }, level(100),
            level(90)];
        // The Brightness the windows daemon serves says the share it read,
        // which a typed "brightness 40" says; none while it's being read, and nothing
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
