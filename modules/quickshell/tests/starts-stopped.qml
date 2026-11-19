// Starts Stopped's Return and Let Them Start, run in Qt's own JavaScript
// engine, the one the shell runs them in. The
// `starts-stopped-lifts-only-on-joels-return` check copies
// modules/quickshell/launcher/StartsStopped.js, launcher/Sprouting.js and
// tests/lib/Expect.js beside this file and runs it offscreen.
import QtQuick
import "Expect.js" as Expect
import "launcher/StartsStopped.js" as StartsStopped

QtObject {
    // A case that doesn't hold throws, and ends the test as a failure.
    Component.onCompleted: Expect.run(cases)

    readonly property var letStart: ({ key: "verb:agents:starts:let",
        ask: { ask: "let-them-start" } })
    readonly property var stood: ({ kind: "starts-stopped",
        acts: [{ key: "verb:details", step: "drawer" }, letStart] })

    function cases() {
        // Return steps in, so one Return never lifts the stop.
        Expect.same("its Return steps in where the drawer's step is dropped",
            StartsStopped.stepsIn(stood, {}), true);
        Expect.same("and where there's no launcher to say",
            StartsStopped.stepsIn(stood, null), true);
        Expect.same("a launcher stepping into drawers takes the drawer's step itself",
            StartsStopped.stepsIn(stood, { intoDrawer: () => {} }), false);
        Expect.same("served with no look first, it isn't remade",
            StartsStopped.stepsIn({ kind: "starts-stopped", acts: [letStart] }, {}), false);
        Expect.same("any other row's Return is as made",
            StartsStopped.stepsIn({ kind: "network", acts: stood.acts }, {}), false);
        Expect.same("nor is no row", StartsStopped.stepsIn(null, {}), false);

        // Which act lifts it.
        Expect.same("Let Them Start lifts the stop", StartsStopped.lifts(letStart), true);
        Expect.same("a look at its drawer doesn't", StartsStopped.lifts(stood.acts[0]), false);
        Expect.same("nor does Let Them Start with nothing to ask",
            StartsStopped.lifts({ key: "verb:agents:starts:let" }), false);

        // Only Joel's own Return lifts it, failing closed.
        Expect.same("his Return in the launcher lifts it",
            StartsStopped.mayLift({ cataloguing: false }), true);
        Expect.same("a walk from outside lifts nothing",
            StartsStopped.mayLift({ cataloguing: true }), false);
        Expect.same("no launcher lifts nothing", StartsStopped.mayLift(null), false);
        Expect.same("a launcher that doesn't say it isn't walked lifts nothing",
            StartsStopped.mayLift({}), false);
    }
}
