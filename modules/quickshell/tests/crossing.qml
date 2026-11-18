// How a battery that reports only its charge stands, run in Qt's own
// JavaScript engine, the one the shell runs it in. The
// `a-low-charge-is-said-once` check copies modules/quickshell/Crossing.js
// beside this file and runs `qml -platform offscreen crossing.qml`.
import QtQuick
import "Crossing.js" as Crossing

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
        // [what, readings as [charge, charging], how it stands after each],
        // with the laptop's own marks: low at 20, critical at 10.
        const cases = [
            ["a full charge is fine", [[80, false]], [""]],
            ["it runs low at the mark", [[21, false], [20, false]], ["", "low"]],
            ["and stays low as it drains", [[20, false], [15, false]], ["low", "low"]],
            ["wavering about the mark says nothing more",
                [[20, false], [21, false], [20, false], [24, false], [19, false]],
                ["low", "low", "low", "low", "low"]],
            ["rising 5 past the mark makes it fine again", [[20, false], [25, false]], ["low", ""]],
            ["so the next drop is news", [[20, false], [25, false], [20, false]],
                ["low", "", "low"]],
            ["it's critical at its mark", [[20, false], [10, false]], ["low", "critical"]],
            ["and stays critical as it rises short of the re-arm",
                [[10, false], [11, false], [19, false], [24, false]],
                ["critical", "critical", "critical", "critical"]],
            ["critical rising past the re-arm is fine", [[10, false], [25, false]],
                ["critical", ""]],
            ["a device first heard low is low", [[15, false]], ["low"]],
            ["and first heard critical, critical", [[5, false]], ["critical"]],
            ["charging is fine at once", [[8, false], [8, true]], ["critical", ""]],
            ["and unplugged low, it's news again", [[8, true], [8, false]], ["", "critical"]]
        ];
        for (const [what, readings, want] of cases) {
            let was = "";
            const got = readings.map(([charge, charging]) => {
                was = Crossing.next(was, charge, charging, { low: 20, critical: 10 });
                return was;
            });
            if (JSON.stringify(got) !== JSON.stringify(want))
                failures.push(`${what}: ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
        }

        for (const failure of failures)
            console.warn(`FAIL ${failure}`);
        Qt.exit(failures.length === 0 ? 0 : 1);
    }
}
